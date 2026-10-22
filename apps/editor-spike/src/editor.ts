// One editor instance: ProseMirror view + Loro document + ALTAIS plugins.

import { LoroDoc } from "loro-crdt";
import {
  createNodeFromLoroObj,
  type LoroDocType,
  LoroSyncPlugin,
  LoroUndoPlugin,
  loroSyncPluginKey,
  redo,
  undo,
  updateLoroToPmState,
} from "loro-prosemirror";
import { baseKeymap, chainCommands, exitCode, setBlockType, toggleMark } from "prosemirror-commands";
import { keymap } from "prosemirror-keymap";
import { liftListItem, sinkListItem, splitListItem, wrapInList } from "prosemirror-schema-list";
import type { Node as PMNode } from "prosemirror-model";
import { type Command, EditorState, Plugin, type Selection, TextSelection } from "prosemirror-state";
import { columnResizing, goToNextCell, tableEditing } from "prosemirror-tables";
import { EditorView } from "prosemirror-view";
import { type AdmBlock, admBlocksToDoc } from "./adm";
import { bidsPlugin, fixBids } from "./bids";
import { cleanPastedHtml } from "./paste";
import { schema } from "./schema";

const n = schema.nodes;
const m = schema.marks;

const insertHardBreak: Command = chainCommands(exitCode, (state, dispatch) => {
  dispatch?.(state.tr.replaceSelectionWith(n.hard_break.create()).scrollIntoView());
  return true;
});

export const commands = {
  bold: toggleMark(m.bold),
  italic: toggleMark(m.italic),
  underline: toggleMark(m.underline),
  strike: toggleMark(m.strike),
  code: toggleMark(m.code),
  paragraph: setBlockType(n.paragraph),
  heading: (level: number) => setBlockType(n.heading, { level }),
  bulletList: wrapInList(n.list, { kind: "bullet" }),
  orderedList: wrapInList(n.list, { kind: "ordered" }),
  checkList: wrapInList(n.list, { kind: "check" }),
  undo: undo as Command,
  redo: redo as Command,
};

const altaisKeymap = keymap({
  "Mod-z": commands.undo,
  "Mod-y": commands.redo,
  "Mod-Shift-z": commands.redo,
  "Mod-b": commands.bold,
  "Mod-i": commands.italic,
  "Mod-u": commands.underline,
  "Mod-Shift-x": commands.strike,
  "Mod-e": commands.code,
  "Mod-Alt-0": commands.paragraph,
  "Mod-Alt-1": commands.heading(1),
  "Mod-Alt-2": commands.heading(2),
  "Mod-Alt-3": commands.heading(3),
  "Mod-Shift-8": commands.bulletList,
  "Mod-Shift-7": commands.orderedList,
  "Mod-Shift-9": commands.checkList,
  "Shift-Enter": insertHardBreak,
  Enter: splitListItem(n.list_item),
  Tab: chainCommands(goToNextCell(1), sinkListItem(n.list_item)),
  "Shift-Tab": chainCommands(goToNextCell(-1), liftListItem(n.list_item)),
});

/** Ticks or unticks a checklist item when its box (left of the text) is clicked. */
const checklistPlugin = new Plugin({
  props: {
    handleDOMEvents: {
      mousedown(view, event) {
        const li = (event.target as HTMLElement).closest?.("ul[data-kind='check'] > li");
        if (!li || !view.dom.contains(li)) return false;
        if (event.clientX >= li.getBoundingClientRect().left) return false; // clicked the text, not the box
        const pos = view.posAtDOM(li, 0) - 1;
        const node = view.state.doc.nodeAt(pos);
        if (node?.type !== n.list_item) return false;
        view.dispatch(view.state.tr.setNodeAttribute(pos, "checked", !node.attrs.checked));
        event.preventDefault();
        return true;
      },
    },
  },
});

/**
 * New check lists start unticked; items in other lists carry no tick.
 * Runs after every change so that `checked` always matches the list kind.
 */
const checkStatePlugin = new Plugin({
  appendTransaction(transactions, _old, state) {
    if (!transactions.some((tr) => tr.docChanged)) return null;
    const tr = state.tr;
    state.doc.descendants((node, pos, parent) => {
      if (node.type !== n.list_item || !parent) return true;
      const isCheck = parent.attrs.kind === "check";
      if (isCheck && node.attrs.checked === null) tr.setNodeAttribute(pos, "checked", false);
      if (!isCheck && node.attrs.checked !== null) tr.setNodeAttribute(pos, "checked", null);
      return true;
    });
    return tr.docChanged ? tr : null;
  },
});

/**
 * Where the cursor belongs after the document changed under it: compare the
 * old and new document; before the changed range it stays, after it it
 * shifts by the size change, inside it it goes to the start of the change.
 */
export function mapSelectionThroughChange(oldDoc: PMNode, selection: TextSelection, newDoc: PMNode): Selection {
  const before = oldDoc.content;
  const after = newDoc.content;
  const clamp = (pos: number) => Math.max(0, Math.min(pos, newDoc.content.size));
  const start = before.findDiffStart(after);
  let map = (pos: number) => pos;
  if (start !== null) {
    const end = before.findDiffEnd(after);
    let endOld = end?.a ?? before.size;
    let endNew = end?.b ?? after.size;
    // When the unchanged ends overlap (e.g. repeated text), shrink them back.
    const overlap = Math.max(0, start - Math.min(endOld, endNew));
    endOld += overlap;
    endNew += overlap;
    map = (pos) => (pos <= start ? pos : pos >= endOld ? pos + (endNew - endOld) : start);
  }
  return TextSelection.between(
    newDoc.resolve(clamp(map(selection.anchor))),
    newDoc.resolve(clamp(map(selection.head))),
  );
}

/**
 * Keeps the cursor in place when a change arrives from another device.
 * loro-prosemirror applies a remote change by replacing the whole document,
 * which moves the cursor to the end, and then tries to restore it from a Loro
 * cursor; that lookup fails when the remote change touched the paragraph with
 * the cursor.
 */
export const keepCursorOnRemoteChange = new Plugin({
  appendTransaction(transactions, oldState, newState) {
    const remote = transactions.some(
      (tr) => (tr.getMeta(loroSyncPluginKey) as { type?: string } | undefined)?.type === "non-local-updates",
    );
    if (!remote || !(oldState.selection instanceof TextSelection)) return null;
    const selection = mapSelectionThroughChange(oldState.doc, oldState.selection, newState.doc);
    return newState.selection.eq(selection) ? null : newState.tr.setSelection(selection).setMeta("addToHistory", false);
  },
});

/**
 * Imports updates from other devices one by one. Loro 1.17's `importBatch`
 * with more than one update erases the undo history of the receiving
 * document (checked in tests/adm.test.ts), so it is not used.
 */
export function importUpdates(loro: LoroDocType, updates: Uint8Array[]): void {
  for (const update of updates) loro.import(update);
}

const nextTask = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

/**
 * Applies changes from other devices to an open note without moving the
 * user's cursor.
 *
 * Loro reports the import synchronously and loro-prosemirror then restores
 * the cursor in a timer; a click landing before that timer would be undone.
 * So the editor ignores input until the timer has run, and the cursor is then
 * set to where it belongs. The pause lasts a few milliseconds.
 */
export async function applyRemoteUpdates(
  editor: AltaisEditor,
  updates: Uint8Array[],
  options: { focus?: boolean } = {},
): Promise<void> {
  if (!updates.length) return;
  const { view, loro } = editor;
  const oldDoc = view.state.doc;
  const oldSelection = view.state.selection;
  const hadFocus = view.hasFocus();
  editor.setEditable(false);
  try {
    importUpdates(loro, updates);
    await nextTask(); // loro-prosemirror's cursor timer runs here
    await nextTask();
  } finally {
    if (!view.isDestroyed) editor.setEditable(true);
  }
  if (view.isDestroyed) return;
  if (oldSelection instanceof TextSelection) {
    const selection = mapSelectionThroughChange(oldDoc, oldSelection, view.state.doc);
    if (!view.state.selection.eq(selection)) {
      view.dispatch(view.state.tr.setSelection(selection).setMeta("addToHistory", false));
    }
  }
  if (options.focus || hadFocus) view.focus();
}

/** Writes ADM blocks into an empty Loro document, as the starting content. */
export function seedLoroDoc(loro: LoroDocType, blocks: AdmBlock[]): void {
  let state = EditorState.create({ doc: admBlocksToDoc(blocks), plugins: [LoroSyncPlugin({ doc: loro })] });
  const fixed = fixBids(state.doc, state.tr);
  if (fixed) state = state.apply(fixed);
  updateLoroToPmState(loro, new Map(), state);
}

/** The note stored in a Loro document, without an editor. */
export function docFromLoro(loro: LoroDocType): PMNode {
  return createNodeFromLoroObj(schema, loro.getMap("doc"), new Map());
}

/** A Loro document for one device. `peer` identifies the device in the history. */
export function newLoroDoc(peer?: bigint): LoroDocType {
  const loro = new LoroDoc() as LoroDocType;
  if (peer !== undefined) loro.setPeerId(peer);
  return loro;
}

export interface AltaisEditor {
  view: EditorView;
  loro: LoroDocType;
  /** Stops or allows editing (used for a moment while changes are applied). */
  setEditable(editable: boolean): void;
}

export interface EditorOptions {
  /** Called after every change of the editor state. */
  onUpdate?: () => void;
}

// Note: everything is passed to EditorView once. `view.setProps` makes
// ProseMirror recreate all plugin views, which resets loro-prosemirror's undo
// history and re-runs its initial load.
export function createEditor(mount: HTMLElement, loro: LoroDocType, options: EditorOptions = {}): AltaisEditor {
  let editable = true;
  const state = EditorState.create({
    schema,
    // Order matters: the plugins that complete a change (block IDs, ticks)
    // run before LoroSyncPlugin, so the other device never receives a block
    // without an ID and does not race to assign its own.
    plugins: [
      checkStatePlugin,
      bidsPlugin,
      LoroSyncPlugin({ doc: loro }),
      LoroUndoPlugin({ doc: loro }),
      altaisKeymap,
      keymap(baseKeymap),
      columnResizing(),
      tableEditing(),
      checklistPlugin,
      keepCursorOnRemoteChange,
    ],
  });
  const view: EditorView = new EditorView(mount, {
    state,
    transformPastedHTML: cleanPastedHtml,
    editable: () => editable,
    dispatchTransaction(tr) {
      view.updateState(view.state.apply(tr));
      options.onUpdate?.();
    },
  });
  return {
    view,
    loro,
    setEditable(value) {
      editable = value;
      view.updateState(view.state); // refreshes contenteditable
    },
  };
}
