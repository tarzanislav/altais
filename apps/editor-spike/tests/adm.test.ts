import Ajv2020 from "ajv/dist/2020.js";
import { createNodeFromLoroObj, type LoroDocType, LoroSyncPlugin, updateLoroToPmState } from "loro-prosemirror";
import { EditorState, TextSelection } from "prosemirror-state";
import { splitBlock } from "prosemirror-commands";
import { describe, expect, it } from "vitest";
import { type AdmBlock, admBlocksToDoc, docToAdmBlocks } from "../src/adm";
import { bidsPlugin } from "../src/bids";
import { newLoroDoc, seedLoroDoc } from "../src/editor";
import { schema } from "../src/schema";

import admSchema from "../../../spec/adm/adm-0.1.schema.json";
import plainNoteJson from "../../../spec/adm/examples/note.json";
import richNoteJson from "../../../spec/adm/examples/rich-note.json";

const richNote = richNoteJson as unknown as { content: AdmBlock[] };
const plainNote = plainNoteJson as unknown as { content: AdmBlock[] };

const ajv = new Ajv2020({ allErrors: true, strict: false });
ajv.addFormat("date-time", /^\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(\.\d+)?(Z|[+-]\d\d:\d\d)$/);
const validateAdm = ajv.compile(admSchema);

function expectValidAdm(content: AdmBlock[]) {
  const doc = { ...richNote, content };
  const ok = validateAdm(doc);
  expect(validateAdm.errors ?? []).toEqual([]);
  expect(ok).toBe(true);
}

describe("ADM <-> ProseMirror", () => {
  it.each([
    ["rich-note.json", richNote],
    ["note.json", plainNote],
  ])("round-trips %s without changes", (_name, note) => {
    const doc = admBlocksToDoc(note.content);
    doc.check(); // the document fits the schema
    expect(docToAdmBlocks(doc)).toEqual(note.content);
  });

  it("produces ADM that passes the JSON Schema", () => {
    expectValidAdm(docToAdmBlocks(admBlocksToDoc(richNote.content)));
  });

  it("keeps blocks it cannot show unchanged", () => {
    const encrypted = richNote.content.find((b) => b.t === "evernote_encrypted");
    const doc = admBlocksToDoc([encrypted!]);
    expect(doc.firstChild?.type.name).toBe("unknown");
    expect(docToAdmBlocks(doc)).toEqual([encrypted]);
  });

  it("keeps a line break with the text that follows it", () => {
    const blocks: AdmBlock[] = [
      { t: "paragraph", bid: "p001", text: [{ s: "tučné", marks: ["bold"] }, { s: "\nďalší riadok" }] },
    ];
    expect(docToAdmBlocks(admBlocksToDoc(blocks))).toEqual(blocks);
  });
});

describe("block IDs", () => {
  it("gives the new half of a split paragraph its own ID", () => {
    let state = EditorState.create({
      doc: admBlocksToDoc([{ t: "paragraph", bid: "p001", text: [{ s: "jedna dva" }] }]),
      plugins: [bidsPlugin],
    });
    state = state.apply(state.tr.setSelection(TextSelection.create(state.doc, 6)));
    splitBlock(state, (tr) => {
      state = state.apply(tr);
    });
    const blocks = docToAdmBlocks(state.doc);
    expect(blocks).toHaveLength(2);
    expect(blocks[0].bid).toBe("p001");
    expect(blocks[1].bid).toMatch(/^[A-Za-z0-9_-]{4,32}$/);
    expect(blocks[1].bid).not.toBe("p001");
    expectValidAdm(blocks);
  });

  it("assigns IDs to blocks that have none", () => {
    let state = EditorState.create({ schema, plugins: [bidsPlugin] });
    state = state.apply(state.tr.insertText("ahoj"));
    const [block] = docToAdmBlocks(state.doc);
    expect(block.bid).toMatch(/^[A-Za-z0-9]{10}$/);
  });
});

describe("Loro sync", () => {
  function stateFromLoro(loro: LoroDocType): EditorState {
    const doc = createNodeFromLoroObj(schema, loro.getMap("doc"), new Map());
    return EditorState.create({ doc, plugins: [LoroSyncPlugin({ doc: loro })] });
  }

  function edit(loro: LoroDocType, change: (state: EditorState) => EditorState) {
    const next = change(stateFromLoro(loro));
    updateLoroToPmState(loro, new Map(), next);
  }

  function insertAt(text: string, pos: number) {
    return (state: EditorState) => state.apply(state.tr.insertText(text, pos));
  }

  it("brings two devices with concurrent edits to the same note", () => {
    const a = newLoroDoc(1n);
    seedLoroDoc(a, richNote.content);
    const b = newLoroDoc(2n);
    b.import(a.export({ mode: "snapshot" }));

    // Both devices are offline and edit different places.
    edit(a, insertAt("Veľký ", 1));
    edit(b, (state) => {
      const end = state.doc.child(0).nodeSize; // start of the second block
      return state.apply(state.tr.insertText("Pozor: ", end + 1));
    });

    // Back online: exchange what the other side has not seen.
    const fromA = a.export({ mode: "update", from: b.oplogVersion() });
    const fromB = b.export({ mode: "update", from: a.oplogVersion() });
    a.import(fromB);
    b.import(fromA);

    const admA = docToAdmBlocks(stateFromLoro(a).doc);
    const admB = docToAdmBlocks(stateFromLoro(b).doc);
    expect(admA).toEqual(admB);
    expect(admA[0]).toMatchObject({ t: "heading", text: [{ s: "Veľký Výlet" }] });
    expect((admA[1].text as { s: string }[])[0].s).toBe("Pozor: H");
    expectValidAdm(admA);
  });

  it("keeps formatting and block IDs through Loro", () => {
    const loro = newLoroDoc();
    seedLoroDoc(loro, richNote.content);
    expect(docToAdmBlocks(stateFromLoro(loro).doc)).toEqual(richNote.content);
  });
});
