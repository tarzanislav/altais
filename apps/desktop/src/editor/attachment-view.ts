// How attachments look in a note: pictures are shown, other files as a card.
// Both offer "Save as…". The content is loaded from the archive on demand.

import type { Node as PMNode } from "prosemirror-model";
import type { NodeView } from "prosemirror-view";

export interface AttachmentHandler {
  load(blob: string): Promise<Uint8Array>;
  save(blob: string, name: string): Promise<boolean>;
  labels: { saveAs: string; cannotLoad: string };
}

/** Image types the WebView shows directly. */
const IMAGE_TYPES = new Set(["image/png", "image/jpeg", "image/gif", "image/webp", "image/bmp", "image/svg+xml"]);

export function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} kB`;
  return `${bytes} B`;
}

function el<K extends keyof HTMLElementTagNameMap>(tag: K, cls?: string, text?: string): HTMLElementTagNameMap[K] {
  const e = document.createElement(tag);
  if (cls) e.className = cls;
  if (text !== undefined) e.textContent = text;
  return e;
}

export function attachmentView(node: PMNode, handler: AttachmentHandler): NodeView {
  const { blob, name, mime, size } = node.attrs as { blob: string; name: string; mime: string; size: number };
  const isImage = IMAGE_TYPES.has(mime);
  const dom = el("div", isImage ? "attachment attachment-image" : "attachment attachment-file");
  dom.contentEditable = "false";
  dom.dataset.bid = node.attrs.bid;
  let url: string | null = null;

  if (isImage) {
    const img = el("img");
    img.alt = name;
    dom.append(img);
    handler
      .load(blob)
      .then((data) => {
        url = URL.createObjectURL(new Blob([data as BlobPart], { type: mime }));
        img.src = url;
      })
      .catch(() => {
        img.replaceWith(el("div", "attachment-missing", handler.labels.cannotLoad));
      });
  } else {
    dom.append(el("span", "attachment-icon", "📄"));
  }

  const caption = el("div", "attachment-caption");
  caption.append(el("span", "attachment-name", name), el("span", "attachment-size", formatSize(size)));
  const save = el("button", "attachment-save", handler.labels.saveAs);
  save.type = "button";
  save.addEventListener("click", () => void handler.save(blob, name));
  caption.append(save);
  dom.append(caption);

  return {
    dom,
    stopEvent: (event) => (event.target as HTMLElement).closest?.("button") !== null,
    ignoreMutation: () => true,
    destroy() {
      if (url) URL.revokeObjectURL(url);
    },
  };
}
