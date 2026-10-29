// The interface to the ALTAIS core. In the desktop app every call goes to a
// Tauri command (src-tauri/src/main.rs, which calls altais-core). In a plain
// browser (development, tests) an in-memory stand-in answers instead.

import type { AdmBlock } from "./editor/adm";

export interface Status {
  dir: string;
  exists: boolean;
  needsPassword: boolean;
  unlocked: boolean;
  hasKeystore: boolean;
  hasPassword: boolean;
}

export type Secret = { kind: "keyStore" } | { kind: "password"; value: string } | { kind: "recovery"; value: string };

export interface NotebookInfo {
  id: string;
  name: string;
  count: number;
}

export interface NoteInfo {
  id: string;
  title: string;
  notebook: string | null;
  updated: string;
}

export interface NoteData extends NoteInfo {
  tags: string[];
  created: string;
  content: AdmBlock[];
}

export interface ImportOutcome {
  file: string;
  notebook: string;
  stored: number;
  partial: { title: string; problems: string[] }[];
  attachments: number;
  stoppedEarly: string | null;
}

export interface AttachmentInfo {
  blob: string;
  name: string;
  mime: string;
  size: number;
}

/** An error from the core: `code` is stable and translated by the interface. */
export interface CoreError {
  code: string;
  message: string;
}

export function isCoreError(e: unknown): e is CoreError {
  return typeof e === "object" && e !== null && "code" in e && "message" in e;
}

export interface Backend {
  status(): Promise<Status>;
  create(password: string | null): Promise<string>;
  unlock(secret: Secret): Promise<Status>;
  lock(): Promise<void>;
  notebooks(): Promise<NotebookInfo[]>;
  createNotebook(name: string): Promise<NotebookInfo>;
  notes(notebook: string | null): Promise<NoteInfo[]>;
  search(query: string): Promise<NoteInfo[]>;
  note(id: string): Promise<NoteData>;
  newNote(notebook: string | null): Promise<NoteData>;
  saveNote(id: string, title: string, content: AdmBlock[]): Promise<NoteInfo>;
  trashNote(id: string): Promise<void>;
  /** Asks the user for ENEX files and imports them; null if cancelled. */
  importEnex(): Promise<ImportOutcome[] | null>;
  /** The content of an attachment. */
  attachment(blob: string): Promise<Uint8Array>;
  /** Asks where to save an attachment and writes it; false if cancelled. */
  saveAttachment(blob: string, name: string): Promise<boolean>;
  /** Asks for a file and stores it as an attachment; null if cancelled. */
  addAttachment(): Promise<AttachmentInfo | null>;
}

// ---------------- Tauri ----------------

async function tauriBackend(): Promise<Backend> {
  const { invoke } = await import("@tauri-apps/api/core");
  const { open, save } = await import("@tauri-apps/plugin-dialog");
  return {
    status: () => invoke("status"),
    create: (password) => invoke("create_archive", { password }),
    unlock: (secret) => invoke("unlock", { secret }),
    lock: () => invoke("lock"),
    notebooks: () => invoke("notebooks"),
    createNotebook: (name) => invoke("create_notebook", { name }),
    notes: (notebook) => invoke("notes", { notebook }),
    search: (query) => invoke("search", { query }),
    note: (id) => invoke("note", { id }),
    newNote: (notebook) => invoke("new_note", { notebook }),
    saveNote: (id, title, content) => invoke("save_note", { edit: { id, title, content } }),
    trashNote: (id) => invoke("trash_note", { id }),
    async importEnex() {
      const picked = await open({
        multiple: true,
        directory: false,
        filters: [{ name: "Evernote (ENEX)", extensions: ["enex"] }],
      });
      if (!picked || (Array.isArray(picked) && picked.length === 0)) return null;
      const paths = Array.isArray(picked) ? picked : [picked];
      return invoke("import_enex", { paths });
    },
    async attachment(blob) {
      return new Uint8Array(await invoke<ArrayBuffer>("attachment", { blob }));
    },
    async saveAttachment(blob, name) {
      const path = await save({ defaultPath: name });
      if (!path) return false;
      await invoke("save_attachment", { blob, path });
      return true;
    },
    async addAttachment() {
      const path = await open({ multiple: false, directory: false });
      if (!path || Array.isArray(path)) return null;
      return invoke("add_attachment", { path });
    },
  };
}

// ---------------- In-memory stand-in ----------------

function err(code: string, message: string): CoreError {
  return { code, message };
}

/** A backend without files or encryption, for the browser and tests. */
export function memoryBackend(): Backend {
  let exists = false;
  let unlocked = false;
  let password: string | null = null;
  let recovery = "";
  const notebooks = new Map<string, string>();
  const notes = new Map<string, NoteData & { trashed: boolean }>();
  const blobs = new Map<string, Uint8Array>();
  const saved: string[] = [];
  let counter = 0;
  const id = () => `0192b6e1-2d3c-7f40-8a51-${(++counter).toString(16).padStart(12, "0")}`;
  const now = () => new Date().toISOString();
  const guard = () => {
    if (!unlocked) throw err("locked", "the archive is not open");
  };
  const info = (n: NoteData): NoteInfo => ({ id: n.id, title: n.title, notebook: n.notebook, updated: n.updated });
  const fold = (s: string) => s.normalize("NFD").replace(/\p{M}/gu, "").replace(/ł/gi, "l").toLowerCase();
  const text = (blocks: unknown): string => JSON.stringify(blocks);
  const status = (): Status => ({
    dir: "(in memory)",
    exists,
    needsPassword: exists && password !== null,
    unlocked,
    hasKeystore: true,
    hasPassword: password !== null,
  });
  const add = (title: string, notebook: string | null, content: AdmBlock[]) => {
    const n = { id: id(), title, notebook, tags: [], created: now(), updated: now(), content, trashed: false };
    notes.set(n.id, n);
    return n;
  };
  return {
    status: async () => status(),
    async create(pw) {
      exists = true;
      unlocked = true;
      password = pw && pw.length ? pw : null;
      recovery = "ALTAIS-ABCDEF-GHJKMN-PQRSTV-WXYZ01-234567-89ABCD-EFGHJK-MNPQRS-TV0123";
      return recovery;
    },
    async unlock(secret) {
      if (secret.kind === "password" && secret.value !== password) throw err("wrong_password", "wrong password");
      if (secret.kind === "recovery" && secret.value.toUpperCase().replace(/\s/g, "") !== recovery)
        throw err("wrong_recovery_key", "wrong recovery key");
      if (secret.kind === "keyStore" && password !== null) throw err("key_not_on_this_computer", "needs password");
      unlocked = true;
      return status();
    },
    async lock() {
      unlocked = false;
    },
    async notebooks() {
      guard();
      return [...notebooks].map(([nid, name]) => ({
        id: nid,
        name,
        count: [...notes.values()].filter((n) => n.notebook === nid && !n.trashed).length,
      }))
        .sort((a, b) => a.name.localeCompare(b.name));
    },
    async createNotebook(name) {
      guard();
      if (!name.trim()) throw err("empty_name", "the notebook needs a name");
      const nid = id();
      notebooks.set(nid, name.trim());
      return { id: nid, name: name.trim(), count: 0 };
    },
    async notes(notebook) {
      guard();
      return [...notes.values()]
        .filter((n) => !n.trashed && (notebook === null || n.notebook === notebook))
        .sort((a, b) => b.updated.localeCompare(a.updated))
        .map(info);
    },
    async search(query) {
      guard();
      const words = fold(query).split(/\s+/).filter(Boolean);
      if (!words.length) return [];
      return [...notes.values()]
        .filter((n) => !n.trashed && words.every((w) => fold(n.title + " " + text(n.content)).includes(w)))
        .map(info);
    },
    async note(nid) {
      guard();
      const n = notes.get(nid);
      if (!n) throw err("not_found", "the note does not exist");
      return structuredClone(n);
    },
    async newNote(notebook) {
      guard();
      return structuredClone(add("", notebook, []));
    },
    async saveNote(nid, title, content) {
      guard();
      const n = notes.get(nid);
      if (!n) throw err("not_found", "the note does not exist");
      n.title = title.trim();
      n.content = structuredClone(content);
      n.updated = now();
      return info(n);
    },
    async trashNote(nid) {
      guard();
      const n = notes.get(nid);
      if (n) n.trashed = true;
    },
    async importEnex() {
      guard();
      const example = (await import("../../../spec/adm/examples/rich-note.json")).default as { title: string; content: AdmBlock[] };
      const nid = id();
      notebooks.set(nid, "Ukážka z Evernote");
      add(example.title, nid, example.content);
      add("Druhá poznámka", nid, [{ t: "paragraph", bid: "p0001", text: [{ s: "Zoznam úloh na víkend" }] }]);
      const png = Uint8Array.from(atob(SAMPLE_PNG), (c) => c.charCodeAt(0));
      const blob = "blake3:" + "ab".repeat(32);
      blobs.set(blob, png);
      add("Fotka z výletu", nid, [
        { t: "paragraph", bid: "p0001", text: [{ s: "Výhľad z Rysov:" }] },
        { t: "attachment", bid: "a0001", blob, name: "rysy.png", mime: "image/png", size: png.length },
        { t: "attachment", bid: "a0002", blob, name: "trasa.gpx", mime: "application/octet-stream", size: png.length },
      ]);
      return [{ file: "ukazka.enex", notebook: "Ukážka z Evernote", stored: 3, partial: [], attachments: 1, stoppedEarly: null }];
    },
    async attachment(blob) {
      guard();
      const data = blobs.get(blob);
      if (!data) throw err("missing_attachment", "the attachment is not in the archive");
      return data;
    },
    async saveAttachment(blob, name) {
      guard();
      saved.push(`${name}:${blob}`);
      (window as unknown as { savedAttachments: string[] }).savedAttachments = saved;
      return true;
    },
    async addAttachment() {
      guard();
      const data = new TextEncoder().encode("Zoznam vecí na výlet");
      const blob = "blake3:" + "cd".repeat(32);
      blobs.set(blob, data);
      return { blob, name: "zoznam.txt", mime: "text/plain", size: data.length };
    },
  };
}

let backendPromise: Promise<Backend> | null = null;

/** The backend for this environment (Tauri when running in the desktop app). */
export function backend(): Promise<Backend> {
  backendPromise ??= "__TAURI_INTERNALS__" in window ? tauriBackend() : Promise.resolve(memoryBackend());
  return backendPromise;
}

/** A small blue PNG used by the in-memory backend's sample import. */
const SAMPLE_PNG = "iVBORw0KGgoAAAANSUhEUgAAACgAAAAYCAIAAAAH5iiXAAAAK0lEQVR4nGPUj9/MMBCAaUBsHbV41OJRi0ctHrV41OJRi0ctHrV41GKaAABSywFxu8WDeQAAAABJRU5ErkJggg==";
