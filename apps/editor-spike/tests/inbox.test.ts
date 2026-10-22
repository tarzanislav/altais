import { UndoManager } from "loro-crdt";
import { describe, expect, it } from "vitest";
import { importUpdates, newLoroDoc } from "../src/editor";
import { IdlePolicy, Inbox } from "../src/inbox";

describe("Inbox", () => {
  it("collects updates until they are taken, and reports the count", () => {
    const inbox = new Inbox();
    const counts: number[] = [];
    inbox.onChange((count) => counts.push(count));
    inbox.add(new Uint8Array([1]));
    inbox.add(new Uint8Array([2]));
    expect(inbox.count).toBe(2);
    expect(inbox.takeAll().map((u) => u[0])).toEqual([1, 2]);
    expect(inbox.count).toBe(0);
    expect(inbox.takeAll()).toEqual([]);
    expect(counts).toEqual([1, 2, 0]);
  });
});

describe("IdlePolicy", () => {
  function clock() {
    let now = 0;
    return { now: () => now, advance: (ms: number) => (now += ms) };
  }

  it("applies only after the pause and only when something waits", () => {
    const time = clock();
    const policy = new IdlePolicy(10_000, time.now);
    expect(policy.shouldApply(3, false)).toBe(false);
    time.advance(9_999);
    expect(policy.shouldApply(3, false)).toBe(false);
    time.advance(1);
    expect(policy.shouldApply(3, false)).toBe(true);
    expect(policy.shouldApply(0, false)).toBe(false);
  });

  it("starts the pause again on every input", () => {
    const time = clock();
    const policy = new IdlePolicy(10_000, time.now);
    time.advance(8_000);
    policy.noteInput();
    time.advance(8_000);
    expect(policy.shouldApply(1, false)).toBe(false);
    time.advance(2_000);
    expect(policy.shouldApply(1, false)).toBe(true);
  });

  it("never applies while text is being composed (input methods)", () => {
    const time = clock();
    const policy = new IdlePolicy(1_000, time.now);
    time.advance(60_000);
    expect(policy.shouldApply(1, true)).toBe(false);
  });
});

describe("importUpdates", () => {
  it("keeps the undo history of the receiving device", () => {
    const a = newLoroDoc(1n);
    a.getText("t").insert(0, "Výlet");
    a.commit();
    const b = newLoroDoc(2n);
    b.import(a.export({ mode: "snapshot" }));
    const undo = new UndoManager(b, {});
    b.getText("t").insert(0, "!");
    b.commit();

    const updates: Uint8Array[] = [];
    a.subscribeLocalUpdates((u) => updates.push(u));
    a.getText("t").insert(5, " A");
    a.commit();
    a.getText("t").insert(7, "B");
    a.commit();
    expect(updates).toHaveLength(2);

    importUpdates(b, updates);
    expect(b.getText("t").toString()).toBe("!Výlet AB");
    expect(undo.canUndo()).toBe(true);
    undo.undo();
    expect(b.getText("t").toString()).toBe("Výlet AB");
  });
});
