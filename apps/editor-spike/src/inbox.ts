// Changes from other devices wait in an inbox instead of changing the note
// while the user is writing. In the app the inbox lives in the encrypted local
// database, so nothing is lost if the program stops before they are applied.

export type ApplyReason = "button" | "idle" | "close" | "open";

/** Updates received from other devices that the open note does not show yet. */
export class Inbox {
  private updates: Uint8Array[] = [];
  private listeners = new Set<(count: number) => void>();

  get count(): number {
    return this.updates.length;
  }

  add(update: Uint8Array): void {
    this.updates.push(update);
    this.emit();
  }

  /** Removes and returns everything that is waiting. */
  takeAll(): Uint8Array[] {
    const all = this.updates.splice(0);
    if (all.length) this.emit();
    return all;
  }

  onChange(listener: (count: number) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit() {
    for (const listener of this.listeners) listener(this.updates.length);
  }
}

/**
 * Decides when waiting changes are applied without the user asking: after
 * `idleMs` without typing or clicking in the program, and never in the middle
 * of composing text with an input method (Chinese, Japanese, …).
 * Applying on the button, on close and on open is decided by the caller.
 */
export class IdlePolicy {
  private lastInput: number;

  constructor(
    readonly idleMs: number,
    private readonly now: () => number = () => Date.now(),
  ) {
    this.lastInput = now();
  }

  noteInput(): void {
    this.lastInput = this.now();
  }

  shouldApply(waiting: number, composing: boolean): boolean {
    return waiting > 0 && !composing && this.now() - this.lastInput >= this.idleMs;
  }
}
