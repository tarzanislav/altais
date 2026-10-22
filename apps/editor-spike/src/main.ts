// Spike page: two editors, each with its own Loro document, kept in sync by
// exchanging Loro updates. "Offline" holds the updates back until it is
// switched off again, which shows how concurrent edits merge.

import type { Command } from "prosemirror-state";
import exampleNote from "../../../spec/adm/examples/rich-note.json";
import { type AdmBlock, docToAdmBlocks } from "./adm";
import { type AltaisEditor, commands, createEditor, newLoroDoc, seedLoroDoc } from "./editor";
import "./style.css";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el as T;
}

// Device A starts with the example note; device B receives it as a snapshot.
const loroA = newLoroDoc(1n);
seedLoroDoc(loroA, exampleNote.content as AdmBlock[]);
const loroB = newLoroDoc(2n);
loroB.import(loroA.export({ mode: "snapshot" }));

const editorA = createEditor(byId("editor-a"), loroA);
const editorB = createEditor(byId("editor-b"), loroB);

let online = true;
const pendingForA: Uint8Array[] = [];
const pendingForB: Uint8Array[] = [];

function connect(from: AltaisEditor, to: AltaisEditor, queue: Uint8Array[]) {
  from.loro.subscribeLocalUpdates((bytes) => {
    if (online) to.loro.import(bytes);
    else queue.push(bytes);
  });
}
connect(editorA, editorB, pendingForB);
connect(editorB, editorA, pendingForA);

const onlineBox = byId<HTMLInputElement>("online");
onlineBox.addEventListener("change", () => {
  online = onlineBox.checked;
  if (online) {
    if (pendingForB.length) loroB.importBatch(pendingForB.splice(0));
    if (pendingForA.length) loroA.importBatch(pendingForA.splice(0));
  }
  render();
});

let active: AltaisEditor = editorA;
for (const editor of [editorA, editorB]) {
  editor.view.dom.addEventListener("focus", () => {
    active = editor;
  });
}

for (const button of document.querySelectorAll<HTMLButtonElement>("[data-command]")) {
  button.addEventListener("mousedown", (event) => event.preventDefault()); // keep the editor focused
  button.addEventListener("click", () => {
    const name = button.dataset.command as string;
    const command: Command | undefined = name.startsWith("heading")
      ? commands.heading(Number(name.slice(7)))
      : (commands[name as keyof typeof commands] as Command | undefined);
    if (!command) return;
    command(active.view.state, active.view.dispatch, active.view);
    active.view.focus();
  });
}

function admJson(editor: AltaisEditor): string {
  return JSON.stringify(docToAdmBlocks(editor.view.state.doc), null, 2);
}

function render() {
  const a = admJson(editorA);
  const b = admJson(editorB);
  byId("adm").textContent = a;
  const status = byId("status");
  status.textContent = a === b ? "In sync" : online ? "Syncing…" : "Offline: devices differ";
  status.dataset.state = a === b ? "same" : "different";
}

// Render after every change in either editor (ProseMirror has no global event).
for (const editor of [editorA, editorB]) {
  editor.view.setProps({
    dispatchTransaction(tr) {
      editor.view.updateState(editor.view.state.apply(tr));
      queueMicrotask(render);
    },
  });
}
setTimeout(render, 50);

// Used by the end-to-end tests.
declare global {
  interface Window {
    spike: { a: AltaisEditor; b: AltaisEditor; adm: (which: "a" | "b") => AdmBlock[] };
  }
}
window.spike = {
  a: editorA,
  b: editorB,
  adm: (which) => docToAdmBlocks((which === "a" ? editorA : editorB).view.state.doc),
};
