// How attachments look in a note: pictures are shown, other files as a card.
// Both offer "Save as…"; files that are not programs also "Open" (in their
// default program, from a temporary copy). The content is loaded from the archive on demand.

import type { Node as PMNode } from "prosemirror-model";
import type { NodeView } from "prosemirror-view";

export interface AttachmentHandler {
  load(blob: string): Promise<Uint8Array>;
  save(blob: string, name: string): Promise<boolean>;
  open(blob: string, name: string): Promise<void>;
  /** Text for an error from `open` or `save`. */
  describeError(e: unknown): string;
  labels: { saveAs: string; open: string; cannotLoad: string };
}

/** Extensions that would run as programs; such files are only saved. The
 * core refuses them too (`open_copy::safe_to_open`), this only hides the button. */
const RUNNABLE = new Set(
  ("ade adp app appx appxbundle application appref-ms bat cab cer chm cmd com cpl crt der diagcab exe gadget " +
    "hta inf ins iso img jar jnlp js jse lnk msc msh msi msix msp mst msu pif pl ps1 ps1xml ps2 psc1 psc2 psd1 " +
    "py pyc pyw pyz rb reg scf scr sct settingcontent-ms sh shb shs theme url vb vbe vbs vhd vhdx ws wsb wsc " +
    "wsf wsh xbap xll library-ms search-ms searchconnector-ms").split(" "),
);

export function canOpen(name: string): boolean {
  const clean = name.replace(/[. ]+$/, "").toLowerCase();
  const dot = clean.lastIndexOf(".");
  return dot > 0 && !RUNNABLE.has(clean.slice(dot + 1));
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
  const problem = el("div", "attachment-problem");
  problem.hidden = true;
  const report = (e: unknown) => {
    problem.textContent = handler.describeError(e);
    problem.hidden = false;
  };
  const actions = el("span", "attachment-actions");
  if (canOpen(name)) {
    const openButton = el("button", "attachment-open", handler.labels.open);
    openButton.type = "button";
    openButton.addEventListener("click", () => {
      problem.hidden = true;
      handler.open(blob, name).catch(report);
    });
    actions.append(openButton);
  }
  const save = el("button", "attachment-save", handler.labels.saveAs);
  save.type = "button";
  save.addEventListener("click", () => {
    problem.hidden = true;
    handler.save(blob, name).catch(report);
  });
  actions.append(save);
  caption.append(actions);
  dom.append(caption, problem);

  return {
    dom,
    stopEvent: (event) => (event.target as HTMLElement).closest?.("button") !== null,
    ignoreMutation: () => true,
    destroy() {
      if (url) URL.revokeObjectURL(url);
    },
  };
}
