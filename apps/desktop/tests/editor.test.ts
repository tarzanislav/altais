import { describe, expect, it } from "vitest";
import richNote from "../../../spec/adm/examples/rich-note.json";
import { type AdmBlock, admBlocksToDoc, docToAdmBlocks } from "../src/editor/adm";

describe("editor content", () => {
  it("round-trips the ADM example unchanged", () => {
    const blocks = richNote.content as unknown as AdmBlock[];
    const doc = admBlocksToDoc(blocks);
    doc.check();
    expect(docToAdmBlocks(doc)).toEqual(blocks);
  });
});
