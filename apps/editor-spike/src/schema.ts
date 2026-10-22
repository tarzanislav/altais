// ProseMirror schema that mirrors the ALTAIS Document Model (ADM 0.1).
// Every block carries a stable `bid`; see bids.ts.

import { type DOMOutputSpec, type MarkSpec, type NodeSpec, Schema } from "prosemirror-model";
import { tableNodes } from "prosemirror-tables";

const bid = { bid: { default: null as string | null } };

function bidOf(dom: HTMLElement | string): { bid: string | null } {
  return { bid: typeof dom === "string" ? null : dom.getAttribute("data-bid") };
}

const tables = tableNodes({
  tableGroup: "block",
  cellContent: "block+",
  cellAttributes: {},
});
// Tables carry a block ID like every other block.
tables.table = { ...tables.table, attrs: { ...bid } };

const nodes: Record<string, NodeSpec> = {
  doc: { content: "block+" },
  paragraph: {
    group: "block",
    content: "inline*",
    attrs: { ...bid },
    parseDOM: [{ tag: "p", getAttrs: bidOf }],
    toDOM: (node): DOMOutputSpec => ["p", { "data-bid": node.attrs.bid }, 0],
  },
  heading: {
    group: "block",
    content: "inline*",
    defining: true,
    attrs: { ...bid, level: { default: 1 } },
    parseDOM: [1, 2, 3, 4, 5, 6].map((level) => ({
      tag: `h${level}`,
      getAttrs: (dom: HTMLElement) => ({ ...bidOf(dom), level: Math.min(level, 4) }),
    })),
    toDOM: (node): DOMOutputSpec => [`h${node.attrs.level}`, { "data-bid": node.attrs.bid }, 0],
  },
  list: {
    group: "block",
    content: "list_item+",
    attrs: { ...bid, kind: { default: "bullet" } },
    parseDOM: [
      {
        tag: "ul",
        getAttrs: (dom: HTMLElement) => ({
          ...bidOf(dom),
          kind:
            dom.getAttribute("data-kind") === "check" || dom.getAttribute("data-type") === "taskList"
              ? "check"
              : "bullet",
        }),
      },
      { tag: "ol", getAttrs: (dom: HTMLElement) => ({ ...bidOf(dom), kind: "ordered" }) },
    ],
    toDOM: (node): DOMOutputSpec => [
      node.attrs.kind === "ordered" ? "ol" : "ul",
      { "data-kind": node.attrs.kind, "data-bid": node.attrs.bid },
      0,
    ],
  },
  list_item: {
    content: "block+",
    defining: true,
    attrs: { ...bid, checked: { default: null as boolean | null } },
    parseDOM: [
      {
        tag: "li",
        getAttrs: (dom: HTMLElement) => {
          const checked = dom.getAttribute("data-checked");
          return { ...bidOf(dom), checked: checked === null ? null : checked === "true" };
        },
      },
    ],
    toDOM: (node): DOMOutputSpec => [
      "li",
      {
        "data-bid": node.attrs.bid,
        "data-checked": node.attrs.checked === null ? undefined : String(node.attrs.checked),
      },
      0,
    ],
  },
  quote: {
    group: "block",
    content: "block+",
    defining: true,
    attrs: { ...bid },
    parseDOM: [{ tag: "blockquote", getAttrs: bidOf }],
    toDOM: (node): DOMOutputSpec => ["blockquote", { "data-bid": node.attrs.bid }, 0],
  },
  code_block: {
    group: "block",
    content: "text*",
    marks: "",
    code: true,
    defining: true,
    attrs: { ...bid, lang: { default: null as string | null } },
    parseDOM: [{ tag: "pre", preserveWhitespace: "full", getAttrs: bidOf }],
    toDOM: (node): DOMOutputSpec => ["pre", { "data-bid": node.attrs.bid }, ["code", 0]],
  },
  divider: {
    group: "block",
    attrs: { ...bid },
    parseDOM: [{ tag: "hr", getAttrs: bidOf }],
    toDOM: (node): DOMOutputSpec => ["hr", { "data-bid": node.attrs.bid }],
  },
  bookmark: {
    group: "block",
    atom: true,
    attrs: { ...bid, url: { default: "" }, title: { default: null as string | null } },
    parseDOM: [
      {
        tag: "div.bookmark",
        getAttrs: (dom: HTMLElement) => ({
          ...bidOf(dom),
          url: dom.getAttribute("data-url") ?? "",
          title: dom.textContent || null,
        }),
      },
    ],
    toDOM: (node): DOMOutputSpec => [
      "div",
      { class: "bookmark", "data-url": node.attrs.url, "data-bid": node.attrs.bid },
      ["a", { href: node.attrs.url, rel: "noopener noreferrer" }, node.attrs.title || node.attrs.url],
    ],
  },
  attachment: {
    group: "block",
    atom: true,
    attrs: {
      ...bid,
      blob: { default: "" },
      name: { default: "" },
      mime: { default: "application/octet-stream" },
      size: { default: 0 },
    },
    toDOM: (node): DOMOutputSpec => [
      "div",
      { class: "attachment", "data-bid": node.attrs.bid },
      `Attachment: ${node.attrs.name} (${node.attrs.mime}, ${node.attrs.size} bytes)`,
    ],
  },
  // Blocks the editor cannot show yet are kept verbatim (JSON in `raw`).
  unknown: {
    group: "block",
    atom: true,
    attrs: { ...bid, raw: { default: "{}" } },
    toDOM: (node): DOMOutputSpec => [
      "div",
      { class: "unknown", "data-bid": node.attrs.bid },
      `Kept unchanged: ${(JSON.parse(node.attrs.raw) as { t?: string }).t ?? "block"}`,
    ],
  },
  ...tables,
  text: { group: "inline" },
  hard_break: {
    inline: true,
    group: "inline",
    selectable: false,
    parseDOM: [{ tag: "br" }],
    toDOM: (): DOMOutputSpec => ["br"],
  },
};

const IGNORED_COLORS = new Set(["windowtext", "black", "#000000", "#000", "rgb(0, 0, 0)", "inherit", "auto"]);

function styleWeightIsBold(value: string): boolean {
  return value === "bold" || value === "bolder" || Number.parseInt(value, 10) >= 600;
}

// Mark order defines the order of marks in ADM spans.
const marks: Record<string, MarkSpec> = {
  link: {
    attrs: { href: {} },
    inclusive: false,
    parseDOM: [{ tag: "a[href]", getAttrs: (dom: HTMLElement) => ({ href: dom.getAttribute("href") }) }],
    toDOM: (mark): DOMOutputSpec => ["a", { href: mark.attrs.href, rel: "noopener noreferrer" }, 0],
  },
  bold: {
    parseDOM: [
      { tag: "strong" },
      // Google Docs wraps pasted content in <b style="font-weight:normal">.
      { tag: "b", getAttrs: (dom: HTMLElement) => (dom.style.fontWeight === "normal" ? false : null) },
      { style: "font-weight", getAttrs: (value: string) => (styleWeightIsBold(value) ? null : false) },
    ],
    toDOM: (): DOMOutputSpec => ["strong", 0],
  },
  italic: {
    parseDOM: [{ tag: "i" }, { tag: "em" }, { style: "font-style=italic" }],
    toDOM: (): DOMOutputSpec => ["em", 0],
  },
  underline: {
    parseDOM: [
      { tag: "u" },
      { style: "text-decoration", getAttrs: (value: string) => (value.includes("underline") ? null : false) },
    ],
    toDOM: (): DOMOutputSpec => ["u", 0],
  },
  strike: {
    parseDOM: [
      { tag: "s" },
      { tag: "del" },
      { tag: "strike" },
      { style: "text-decoration", getAttrs: (value: string) => (value.includes("line-through") ? null : false) },
    ],
    toDOM: (): DOMOutputSpec => ["s", 0],
  },
  code: {
    parseDOM: [{ tag: "code" }],
    toDOM: (): DOMOutputSpec => ["code", 0],
  },
  highlight: {
    attrs: { color: { default: null as string | null } },
    parseDOM: [
      { tag: "mark", getAttrs: (dom: HTMLElement) => ({ color: dom.getAttribute("data-color") }) },
      {
        style: "background-color",
        getAttrs: (value: string) =>
          value && !["transparent", "initial", "inherit", "white", "#ffffff", "rgb(255, 255, 255)"].includes(value)
            ? { color: value }
            : false,
      },
    ],
    toDOM: (mark): DOMOutputSpec => [
      "mark",
      { "data-color": mark.attrs.color ?? undefined, style: mark.attrs.color ? `background:${mark.attrs.color}` : undefined },
      0,
    ],
  },
  superscript: {
    excludes: "subscript",
    parseDOM: [{ tag: "sup" }, { style: "vertical-align=super" }],
    toDOM: (): DOMOutputSpec => ["sup", 0],
  },
  subscript: {
    excludes: "superscript",
    parseDOM: [{ tag: "sub" }, { style: "vertical-align=sub" }],
    toDOM: (): DOMOutputSpec => ["sub", 0],
  },
  color: {
    attrs: { color: {} },
    parseDOM: [
      {
        style: "color",
        getAttrs: (value: string) => (IGNORED_COLORS.has(value.trim().toLowerCase()) ? false : { color: value }),
      },
    ],
    toDOM: (mark): DOMOutputSpec => ["span", { style: `color:${mark.attrs.color}` }, 0],
  },
};

export const schema = new Schema({ nodes, marks });

/** Node types that carry a block ID. */
export const BLOCKS_WITH_ID = new Set(
  Object.entries(nodes)
    .filter(([, spec]) => spec.attrs && "bid" in spec.attrs)
    .map(([name]) => name),
);
