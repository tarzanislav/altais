// One editor instance: ProseMirror view + Loro document + ALTAIS plugins.

import { LoroDoc } from "loro-crdt";
import {
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
import { type Command, EditorState, Plugin, TextSelection } from "prosemirror-state";
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
 * Keeps the cursor in place when a change arrives from another device.
 * loro-prosemirror applies a remote change by replacing the whole document,
 * which moves the cursor to the end, and then tries to restore it from a Loro
 * cursor; that lookup fails when the remote change touched the paragraph with
 * the cursor. Here the cursor is moved by comparing the old and new document:
 * before the changed range it stays, after it it shifts by the size change.
 */
export const keepCursorOnRemoteChange = new Plugin({
  appendTransaction(transactions, oldState, newState) {
    const remote = transactions.some(
      (tr) => (tr.getMeta(loroSyncPluginKey) as { type?: string } | undefined)?.type === "non-local-updates",
    );
    if (!remote || !(oldState.selection instanceof TextSelection)) return null;
    const before = oldState.doc.content;
    const after = newState.doc.content;
    const start = before.findDiffStart(after);
    if (start === null) return null;
    const end = before.findDiffEnd(after);
    let endOld = end?.a ?? before.size;
    let endNew = end?.b ?? after.size;
    // When the unchanged ends overlap (e.g. repeated text), shrink them back.
    const overlap = Math.max(0, start - Math.min(endOld, endNew));
    endOld += overlap;
    endNew += overlap;
    const map = (pos: number) => (pos <= start ? pos : pos >= endOld ? pos + (endNew - endOld) : start);
    const { anchor, head } = oldState.selection;
    const doc = newState.doc;
    const clamp = (pos: number) => Math.max(0, Math.min(pos, doc.content.size));
    const selection = TextSelection.between(doc.resolve(clamp(map(anchor))), doc.resolve(clamp(map(head))));
    return newState.selection.eq(selection) ? null : newState.tr.setSelection(selection).setMeta("addToHistory", false);
  },
});

/** Writes ADM blocks into an empty Loro document, as the starting content. */
export function seedLoroDoc(loro: LoroDocType, blocks: AdmBlock[]): void {
  let state = EditorState.create({ doc: admBlocksToDoc(blocks), plugins: [LoroSyncPlugin({ doc: loro })] });
  const fixed = fixBids(state.doc, state.tr);
  if (fixed) state = state.apply(fixed);
  updateLoroToPmState(loro, new Map(), state);
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
}

export function createEditor(mount: HTMLElement, loro: LoroDocType): AltaisEditor {
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
  const view = new EditorView(mount, {
    state,
    transformPastedHTML: cleanPastedHtml,
  });
  return { view, loro };
}
