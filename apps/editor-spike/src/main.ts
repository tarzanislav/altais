// Spike page: two devices, each with its own Loro document. Changes travel
// as Loro updates; on arrival they wait in the receiving device's inbox and
// are applied when the user asks, after a pause in typing, or when the
// program closes or opens. "Online" off holds the updates in transit.

import { type Command, TextSelection } from "prosemirror-state";
import exampleNote from "../../../spec/adm/examples/rich-note.json";
import { type AdmBlock, docToAdmBlocks } from "./adm";
import { Device } from "./device";
import { commands, docFromLoro, newLoroDoc, seedLoroDoc } from "./editor";
import "./style.css";

function byId<T extends HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`missing #${id}`);
  return el as T;
}

// ?idle=<milliseconds> changes the pause after which waiting changes apply.
const idleMs = Number(new URLSearchParams(location.search).get("idle")) || 10_000;

// Device A starts with the example note; device B receives it as a snapshot.
const loroA = newLoroDoc(1n);
seedLoroDoc(loroA, exampleNote.content as AdmBlock[]);
const loroB = newLoroDoc(2n);
loroB.import(loroA.export({ mode: "snapshot" }));

function device(id: "a" | "b", name: string, loro: typeof loroA): Device {
  return new Device(
    name,
    loro,
    {
      mount: byId(`editor-${id}`),
      bar: byId(`syncbar-${id}`),
      barText: byId(`syncbar-${id}`).querySelector(".syncbar-text") as HTMLElement,
      applyButton: byId(`syncbar-${id}`).querySelector("button") as HTMLButtonElement,
      powerButton: byId(`power-${id}`) as HTMLButtonElement,
      notice: byId(`notice-${id}`),
    },
    idleMs,
    () => queueMicrotask(render),
  );
}

const deviceA = device("a", "Device A", loroA);
const deviceB = device("b", "Device B", loroB);

// Transport between the devices; when offline, updates wait in transit.
let online = true;
const inTransit = new Map<Device, Uint8Array[]>([
  [deviceA, []],
  [deviceB, []],
]);
function connect(from: Device, to: Device) {
  from.loro.subscribeLocalUpdates((bytes) => {
    if (online) to.receive(bytes);
    else inTransit.get(to)?.push(bytes);
    render();
  });
}
connect(deviceA, deviceB);
connect(deviceB, deviceA);

const onlineBox = byId<HTMLInputElement>("online");
onlineBox.addEventListener("change", () => {
  online = onlineBox.checked;
  if (online) for (const [target, queue] of inTransit) for (const bytes of queue.splice(0)) target.receive(bytes);
  render();
});

let active: Device = deviceA;
document.addEventListener("focusin", (event) => {
  for (const d of [deviceA, deviceB]) if (d.editor?.view.dom.contains(event.target as Node)) active = d;
});

for (const button of document.querySelectorAll<HTMLButtonElement>("[data-command]")) {
  button.addEventListener("mousedown", (event) => event.preventDefault()); // keep the editor focused
  button.addEventListener("click", () => {
    const view = active.editor?.view;
    if (!view) return;
    const name = button.dataset.command as string;
    const command: Command | undefined = name.startsWith("heading")
      ? commands.heading(Number(name.slice(7)))
      : (commands[name as keyof typeof commands] as Command | undefined);
    if (!command) return;
    command(view.state, view.dispatch, view);
    view.focus();
  });
}

function admOf(d: Device): AdmBlock[] {
  // A closed device has no editor; read the note from its stored document.
  return docToAdmBlocks(d.editor ? d.editor.view.state.doc : docFromLoro(d.loro));
}

function render() {
  const a = JSON.stringify(admOf(deviceA));
  const b = JSON.stringify(admOf(deviceB));
  byId("adm").textContent = JSON.stringify(admOf(deviceA), null, 2);
  const waiting = deviceA.inbox.count + deviceB.inbox.count;
  const transit = [...inTransit.values()].reduce((sum, q) => sum + q.length, 0);
  const status = byId("status");
  const same = a === b && waiting === 0 && transit === 0;
  status.textContent = same
    ? "In sync"
    : transit > 0
      ? "Offline: changes in transit"
      : waiting > 0
        ? "Changes waiting on a device"
        : "Devices differ";
  status.dataset.state = same ? "same" : "different";
}
setTimeout(render, 50);

// Used by the end-to-end tests.
declare global {
  interface Window {
    spike: {
      a: Device;
      b: Device;
      adm: (which: "a" | "b") => AdmBlock[];
      placeCursor: (which: "a" | "b", text: string, offset: number) => void;
    };
  }
}
window.spike = {
  a: deviceA,
  b: deviceB,
  adm: (which) => admOf(which === "a" ? deviceA : deviceB),
  /** Puts the cursor `offset` characters into the first text that contains `text`. */
  placeCursor(which, text, offset) {
    const view = (which === "a" ? deviceA : deviceB).editor?.view;
    if (!view) throw new Error("device is closed");
    let found = -1;
    view.state.doc.descendants((node, pos) => {
      if (found < 0 && node.isText && node.text?.includes(text)) found = pos + node.text.indexOf(text) + offset;
      return found < 0;
    });
    if (found < 0) throw new Error(`text not found: ${text}`);
    view.focus();
    view.dispatch(view.state.tr.setSelection(TextSelection.create(view.state.doc, found)));
  },
};
