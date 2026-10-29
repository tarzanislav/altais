// The note editor: ProseMirror with the ALTAIS schema, local undo history
// and the plugins from the editor spike (block IDs, checklists, Word paste).

import { baseKeymap, chainCommands, exitCode, setBlockType, toggleMark } from "prosemirror-commands";
import { history, redo, undo } from "prosemirror-history";
import { keymap } from "prosemirror-keymap";
import { liftListItem, sinkListItem, splitListItem, wrapInList } from "prosemirror-schema-list";
import { type Command, EditorState, Plugin } from "prosemirror-state";
import { columnResizing, goToNextCell, tableEditing } from "prosemirror-tables";
import { EditorView } from "prosemirror-view";
import { type AdmBlock, admBlocksToDoc, docToAdmBlocks } from "./adm";
import { bidsPlugin, fixBids } from "./bids";
import { cleanPastedHtml } from "./paste";
import { schema } from "./schema";
import { type AttachmentHandler, attachmentView } from "./attachment-view";

export type { AttachmentHandler };

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
  heading1: setBlockType(n.heading, { level: 1 }),
  heading2: setBlockType(n.heading, { level: 2 }),
  heading3: setBlockType(n.heading, { level: 3 }),
  bulletList: wrapInList(n.list, { kind: "bullet" }),
  orderedList: wrapInList(n.list, { kind: "ordered" }),
  checkList: wrapInList(n.list, { kind: "check" }),
  undo,
  redo,
};

export type CommandName = keyof typeof commands;

const altaisKeymap = keymap({
  "Mod-z": undo,
  "Mod-y": redo,
  "Mod-Shift-z": redo,
  "Mod-b": commands.bold,
  "Mod-i": commands.italic,
  "Mod-u": commands.underline,
  "Mod-Shift-x": commands.strike,
  "Mod-e": commands.code,
  "Mod-Alt-0": commands.paragraph,
  "Mod-Alt-1": commands.heading1,
  "Mod-Alt-2": commands.heading2,
  "Mod-Alt-3": commands.heading3,
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
        if (event.clientX >= li.getBoundingClientRect().left) return false; // the text, not the box
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

/** Items of check lists carry a tick, items of other lists none. */
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

export interface NoteEditor {
  view: EditorView;
  /** The current content as ADM blocks. */
  content(): AdmBlock[];
  run(name: CommandName): void;
  /** Inserts a stored attachment at the cursor. */
  insertAttachment(info: { blob: string; name: string; mime: string; size: number }): void;
  destroy(): void;
}

/** Creates an editor for ADM content; `onChange` runs after every change of the text. */
export function createNoteEditor(
  mount: HTMLElement,
  blocks: AdmBlock[],
  onChange: () => void,
  attachments: AttachmentHandler,
  readOnly = false,
): NoteEditor {
  let state = EditorState.create({
    doc: admBlocksToDoc(blocks),
    plugins: [checkStatePlugin, bidsPlugin, history(), altaisKeymap, keymap(baseKeymap), columnResizing(), tableEditing(), checklistPlugin],
  });
  // Notes from older sources may lack block IDs; give them IDs without an undo step.
  const fixed = fixBids(state.doc, state.tr);
  if (fixed) state = state.apply(fixed.setMeta("addToHistory", false));

  const view: EditorView = new EditorView(mount, {
    state,
    transformPastedHTML: cleanPastedHtml,
    editable: () => !readOnly,
    nodeViews: { attachment: (node) => attachmentView(node, attachments) },
    attributes: { spellcheck: "true", "aria-label": "Note text" },
    dispatchTransaction(tr) {
      view.updateState(view.state.apply(tr));
      if (tr.docChanged) onChange();
    },
  });
  return {
    view,
    content: () => docToAdmBlocks(view.state.doc),
    run(name) {
      commands[name](view.state, view.dispatch, view);
      view.focus();
    },
    insertAttachment(info) {
      const node = n.attachment.create({ bid: null, ...info });
      view.dispatch(view.state.tr.replaceSelectionWith(node).scrollIntoView());
      view.focus();
    },
    destroy: () => view.destroy(),
  };
}
