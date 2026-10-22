// One simulated device: its own Loro document, an editor, an inbox for
// changes from other devices and the sync bar that offers them.

import type { LoroDocType } from "loro-prosemirror";
import { type AltaisEditor, applyRemoteUpdates, createEditor, importUpdates } from "./editor";
import { type ApplyReason, IdlePolicy, Inbox } from "./inbox";

export interface DeviceElements {
  mount: HTMLElement;
  bar: HTMLElement;
  barText: HTMLElement;
  applyButton: HTMLButtonElement;
  powerButton: HTMLButtonElement;
  notice: HTMLElement;
}

const INPUT_EVENTS = ["keydown", "beforeinput", "mousedown", "touchstart", "paste", "compositionstart", "compositionupdate"];

export class Device {
  editor: AltaisEditor | null = null;
  readonly inbox = new Inbox();
  readonly policy: IdlePolicy;
  /** Every time waiting changes were applied, and why (shown to the tests). */
  readonly applied: { reason: ApplyReason; count: number }[] = [];
  private applying = false;
  private timer: ReturnType<typeof setInterval>;

  constructor(
    readonly name: string,
    readonly loro: LoroDocType,
    private readonly el: DeviceElements,
    idleMs: number,
    private readonly onChange: () => void,
  ) {
    this.policy = new IdlePolicy(idleMs);
    this.inbox.onChange(() => this.renderBar());
    el.applyButton.addEventListener("click", () => void this.applyWaiting("button"));
    el.powerButton.addEventListener("click", () => void (this.editor ? this.close() : this.open()));
    this.timer = setInterval(() => this.checkIdle(), 250);
    this.open();
  }

  get isOpen(): boolean {
    return this.editor !== null;
  }

  /** A change from another device arrives: it waits until it is applied. */
  receive(update: Uint8Array): void {
    this.inbox.add(update);
    this.onChange();
  }

  async applyWaiting(reason: ApplyReason): Promise<void> {
    if (this.applying || this.inbox.count === 0) return;
    this.applying = true;
    try {
      const updates = this.inbox.takeAll();
      if (this.editor) await applyRemoteUpdates(this.editor, updates, { focus: reason === "button" });
      else importUpdates(this.loro, updates);
      this.applied.push({ reason, count: updates.length });
    } finally {
      this.applying = false;
      this.onChange();
    }
  }

  /** Closing the program: apply what is waiting, then close the editor. */
  async close(): Promise<void> {
    if (!this.editor) return;
    this.showNotice("Saving changes from other devices…");
    await this.applyWaiting("close");
    this.editor.view.destroy();
    this.editor = null;
    this.el.mount.replaceChildren();
    this.el.powerButton.textContent = "Open app";
    this.showNotice(`${this.name} is closed. Changes from other devices are kept for it.`);
    this.renderBar();
    this.onChange();
  }

  /** Opening the program: apply what arrived while it was closed, then show the note. */
  async open(): Promise<void> {
    if (this.editor) return;
    const waiting = this.inbox.count;
    if (waiting) {
      this.showNotice("Synchronizing…");
      await this.applyWaiting("open");
    }
    const editor = createEditor(this.el.mount, this.loro, { onUpdate: () => queueMicrotask(this.onChange) });
    this.editor = editor;
    for (const type of INPUT_EVENTS) editor.view.dom.addEventListener(type, () => this.policy.noteInput());
    this.el.powerButton.textContent = "Close app";
    this.showNotice(waiting ? `Synchronized ${waiting} ${waiting === 1 ? "change" : "changes"} from other devices.` : "");
    this.renderBar();
    this.onChange();
  }

  destroy(): void {
    clearInterval(this.timer);
    this.editor?.view.destroy();
  }

  private checkIdle() {
    if (!this.editor) return;
    if (this.policy.shouldApply(this.inbox.count, this.editor.view.composing)) void this.applyWaiting("idle");
  }

  private renderBar() {
    const count = this.inbox.count;
    const show = count > 0 && this.isOpen;
    this.el.bar.hidden = !show;
    if (!show) return;
    const seconds = Math.round(this.policy.idleMs / 1000);
    this.el.barText.textContent =
      `${count} ${count === 1 ? "change" : "changes"} from another device waiting. ` +
      `Applied automatically after ${seconds} s without typing.`;
  }

  private showNotice(text: string) {
    this.el.notice.textContent = text;
    this.el.notice.hidden = text === "";
  }
}
