// Conversion between ProseMirror documents and ADM 0.1 JSON.

import { type Mark, type Node as PMNode } from "prosemirror-model";
import { schema } from "./schema";

export type AdmMark =
  | "bold"
  | "italic"
  | "underline"
  | "strike"
  | "code"
  | "highlight"
  | "superscript"
  | "subscript"
  | { link: string }
  | { color: string }
  | { highlight: string };

export interface AdmSpan {
  s: string;
  marks?: AdmMark[];
}

export type AdmBlock = Record<string, unknown> & { t: string; bid?: string };

export interface AdmDocument {
  adm: string;
  content: AdmBlock[];
  [key: string]: unknown;
}

const SIMPLE_MARKS = ["bold", "italic", "underline", "strike", "code", "superscript", "subscript"] as const;

// ---------- ProseMirror -> ADM ----------

function markToAdm(mark: Mark): AdmMark {
  switch (mark.type.name) {
    case "link":
      return { link: mark.attrs.href as string };
    case "color":
      return { color: mark.attrs.color as string };
    case "highlight":
      return mark.attrs.color ? { highlight: mark.attrs.color as string } : "highlight";
    default:
      return mark.type.name as AdmMark;
  }
}

function spansOf(node: PMNode): AdmSpan[] {
  const spans: AdmSpan[] = [];
  node.forEach((child) => {
    // A line break inside a block is a "\n" in the span text.
    const text = child.type.name === "hard_break" ? "\n" : child.isText ? (child.text ?? "") : null;
    if (text === null) return;
    const marks = child.marks.map(markToAdm);
    const last = spans.at(-1);
    if (last && JSON.stringify(last.marks ?? []) === JSON.stringify(marks)) {
      last.s += text;
    } else {
      spans.push(marks.length ? { s: text, marks } : { s: text });
    }
  });
  return spans;
}

function blocksOf(node: PMNode): AdmBlock[] {
  const out: AdmBlock[] = [];
  node.forEach((child) => out.push(blockToAdm(child)));
  return out;
}

function blockToAdm(node: PMNode): AdmBlock {
  const bid = node.attrs.bid as string;
  switch (node.type.name) {
    case "paragraph":
      return { t: "paragraph", bid, text: spansOf(node) };
    case "heading":
      return { t: "heading", bid, level: node.attrs.level as number, text: spansOf(node) };
    case "list": {
      const items: Record<string, unknown>[] = [];
      node.forEach((item) => {
        const entry: Record<string, unknown> = { bid: item.attrs.bid, content: blocksOf(item) };
        if (item.attrs.checked !== null) entry.checked = item.attrs.checked;
        items.push(entry);
      });
      return { t: "list", bid, kind: node.attrs.kind as string, items };
    }
    case "table": {
      const rows: Record<string, unknown>[] = [];
      node.forEach((row) => {
        const cells: Record<string, unknown>[] = [];
        row.forEach((cell) => {
          const entry: Record<string, unknown> = { content: blocksOf(cell) };
          if (cell.attrs.colspan !== 1) entry.colspan = cell.attrs.colspan;
          if (cell.attrs.rowspan !== 1) entry.rowspan = cell.attrs.rowspan;
          if (cell.type.name === "table_header") entry.header = true;
          cells.push(entry);
        });
        rows.push({ cells });
      });
      return { t: "table", bid, rows };
    }
    case "quote":
      return { t: "quote", bid, content: blocksOf(node) };
    case "code_block": {
      const block: AdmBlock = { t: "code", bid, text: node.textContent };
      if (node.attrs.lang) block.lang = node.attrs.lang;
      return block;
    }
    case "divider":
      return { t: "divider", bid };
    case "bookmark": {
      const block: AdmBlock = { t: "bookmark", bid, url: node.attrs.url };
      if (node.attrs.title) block.title = node.attrs.title;
      return block;
    }
    case "attachment":
      return {
        t: "attachment",
        bid,
        blob: node.attrs.blob,
        name: node.attrs.name,
        mime: node.attrs.mime,
        size: node.attrs.size,
      };
    case "unknown":
      return JSON.parse(node.attrs.raw as string) as AdmBlock;
    default:
      throw new Error(`no ADM mapping for node type ${node.type.name}`);
  }
}

/** Converts the editor document into ADM blocks. */
export function docToAdmBlocks(doc: PMNode): AdmBlock[] {
  return blocksOf(doc);
}

// ---------- ADM -> ProseMirror ----------

function admMarkToPm(mark: AdmMark): Mark {
  if (typeof mark === "string") {
    if (mark === "highlight") return schema.marks.highlight.create({ color: null });
    if ((SIMPLE_MARKS as readonly string[]).includes(mark)) return schema.marks[mark].create();
    throw new Error(`unknown mark ${mark}`);
  }
  if ("link" in mark) return schema.marks.link.create({ href: mark.link });
  if ("color" in mark) return schema.marks.color.create({ color: mark.color });
  return schema.marks.highlight.create({ color: mark.highlight });
}

function inlineOf(spans: AdmSpan[] = []): PMNode[] {
  const out: PMNode[] = [];
  for (const span of spans) {
    const marks = (span.marks ?? []).map(admMarkToPm);
    span.s.split("\n").forEach((part, index) => {
      if (index > 0) out.push(schema.nodes.hard_break.create(null, null, marks));
      if (part) out.push(schema.text(part, marks));
    });
  }
  return out;
}

function blocksToPm(blocks: AdmBlock[] = []): PMNode[] {
  const nodes = blocks.map(admBlockToPm);
  return nodes.length ? nodes : [schema.nodes.paragraph.create()];
}

function admBlockToPm(block: AdmBlock): PMNode {
  const bid = block.bid ?? null;
  const n = schema.nodes;
  switch (block.t) {
    case "paragraph":
      return n.paragraph.create({ bid }, inlineOf(block.text as AdmSpan[]));
    case "heading":
      return n.heading.create({ bid, level: block.level }, inlineOf(block.text as AdmSpan[]));
    case "list": {
      const items = (block.items as { bid: string; checked?: boolean; content?: AdmBlock[] }[]).map((item) =>
        n.list_item.create({ bid: item.bid, checked: item.checked ?? null }, blocksToPm(item.content)),
      );
      return n.list.create({ bid, kind: block.kind }, items);
    }
    case "table": {
      const rows = (block.rows as { cells: { content?: AdmBlock[]; colspan?: number; rowspan?: number; header?: boolean }[] }[]).map(
        (row) =>
          n.table_row.create(
            null,
            row.cells.map((cell) =>
              (cell.header ? n.table_header : n.table_cell).create(
                { colspan: cell.colspan ?? 1, rowspan: cell.rowspan ?? 1 },
                blocksToPm(cell.content),
              ),
            ),
          ),
      );
      return n.table.create({ bid }, rows);
    }
    case "quote":
      return n.quote.create({ bid }, blocksToPm(block.content as AdmBlock[]));
    case "code": {
      const text = block.text as string;
      return n.code_block.create({ bid, lang: block.lang ?? null }, text ? schema.text(text) : []);
    }
    case "divider":
      return n.divider.create({ bid });
    case "bookmark":
      return n.bookmark.create({ bid, url: block.url, title: block.title ?? null });
    case "attachment":
      return n.attachment.create({ bid, blob: block.blob, name: block.name, mime: block.mime, size: block.size });
    default:
      // task_ref and future block types are kept verbatim.
      return n.unknown.create({ bid, raw: JSON.stringify(block) });
  }
}

/** Builds an editor document from ADM blocks. */
export function admBlocksToDoc(blocks: AdmBlock[]): PMNode {
  return schema.nodes.doc.create(null, blocksToPm(blocks));
}
