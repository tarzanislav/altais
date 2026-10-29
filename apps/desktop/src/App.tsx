import { type FormEvent, useCallback, useEffect, useRef, useState } from "react";
import {
  type Backend,
  type ImportOutcome,
  type NotebookInfo,
  type NoteInfo,
  type Status,
  backend,
} from "./api";
import { type CommandName, createNoteEditor, type NoteEditor } from "./editor/editor";
import { errorText, locale, t } from "./i18n";

type Screen =
  | { kind: "loading" }
  | { kind: "welcome"; status: Status }
  | { kind: "recovery"; key: string }
  | { kind: "unlock"; status: Status; error?: string }
  | { kind: "main"; demo?: boolean };

export function App() {
  const [api, setApi] = useState<Backend | null>(null);
  const [screen, setScreen] = useState<Screen>({ kind: "loading" });

  const start = useCallback(async (b: Backend) => {
    const status = await b.status();
    if (status.unlocked) return setScreen({ kind: "main", demo: status.demo });
    if (!status.exists) {
      // Someone trying ALTAIS with the demo comes back to the demo.
      if (status.demoExists) {
        try {
          await b.openDemo();
          return setScreen({ kind: "main", demo: true });
        } catch {
          // fall through to the welcome screen
        }
      }
      return setScreen({ kind: "welcome", status });
    }
    if (!status.needsPassword) {
      try {
        await b.unlock({ kind: "keyStore" });
        return setScreen({ kind: "main" });
      } catch (e) {
        return setScreen({ kind: "unlock", status, error: errorText(e) });
      }
    }
    setScreen({ kind: "unlock", status });
  }, []);

  useEffect(() => {
    backend().then((b) => {
      setApi(b);
      void start(b);
    });
  }, [start]);

  if (!api || screen.kind === "loading") return <div className="center muted">{t.loading}</div>;
  switch (screen.kind) {
    case "welcome":
      return (
        <Welcome
          api={api}
          status={screen.status}
          onCreated={(key) => setScreen({ kind: "recovery", key })}
          onDemo={() => setScreen({ kind: "main", demo: true })}
        />
      );
    case "recovery":
      return <RecoveryKeyScreen recoveryKey={screen.key} onDone={() => setScreen({ kind: "main" })} />;
    case "unlock":
      return <Unlock api={api} initialError={screen.error} onUnlocked={() => setScreen({ kind: "main" })} />;
    case "main":
      return (
        <Main
          key={screen.demo ? "demo" : "own"}
          api={api}
          demo={!!screen.demo}
          onLock={async () => {
            await api.lock();
            void start(api);
          }}
          onDemoRemoved={() => void start(api)}
        />
      );
  }
}

// ---------------- First run ----------------

function Welcome({
  api,
  status,
  onCreated,
  onDemo,
}: {
  api: Backend;
  status: Status;
  onCreated: (key: string) => void;
  onDemo: () => void;
}) {
  const [usePassword, setUsePassword] = useState(!status.hasKeystore);
  const [password, setPassword] = useState("");
  const [again, setAgain] = useState("");
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    if (usePassword && password !== again) return setError(t.passwordsDiffer);
    if (usePassword && !password) return setError(t.errors.empty_password);
    setBusy(true);
    try {
      onCreated(await api.create(usePassword ? password : null));
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  }

  return (
    <form className="card" onSubmit={submit} data-testid="welcome">
      <h1>{t.welcomeTitle}</h1>
      <p>{t.welcomeText}</p>
      <p className="muted small">
        {t.archiveFolder}: <code>{status.dir}</code>
      </p>
      {status.hasKeystore ? (
        <label className="check">
          <input type="checkbox" checked={usePassword} onChange={(e) => setUsePassword(e.target.checked)} />
          {t.usePassword}
        </label>
      ) : (
        <p className="muted">{t.passwordRequired}</p>
      )}
      {usePassword && (
        <>
          <label>
            {t.password}
            <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoFocus />
          </label>
          <label>
            {t.passwordAgain}
            <input type="password" value={again} onChange={(e) => setAgain(e.target.value)} />
          </label>
        </>
      )}
      {error && <p className="error">{error}</p>}
      <button className="primary" disabled={busy}>
        {t.createArchive}
      </button>
      <div className="or">{t.or}</div>
      <button
        type="button"
        disabled={busy}
        data-testid="try-demo"
        onClick={async () => {
          setBusy(true);
          try {
            await api.loadDemo(locale.startsWith("sk") ? "sk" : "en");
            onDemo();
          } catch (err) {
            setError(errorText(err));
            setBusy(false);
          }
        }}
      >
        {t.tryDemo}
      </button>
      <p className="muted small">{t.tryDemoHint}</p>
    </form>
  );
}

function RecoveryKeyScreen({ recoveryKey, onDone }: { recoveryKey: string; onDone: () => void }) {
  const [saved, setSaved] = useState(false);
  const [copied, setCopied] = useState(false);
  return (
    <div className="card" data-testid="recovery">
      <h1>{t.recoveryTitle}</h1>
      <p>{t.recoveryText}</p>
      <p className="recovery-key" data-testid="recovery-key">
        {recoveryKey}
      </p>
      <button
        type="button"
        onClick={async () => {
          await navigator.clipboard.writeText(recoveryKey).catch(() => undefined);
          setCopied(true);
        }}
      >
        {copied ? t.copied : t.copy}
      </button>
      <label className="check">
        <input type="checkbox" checked={saved} onChange={(e) => setSaved(e.target.checked)} />
        {t.recoverySaved}
      </label>
      <button className="primary" disabled={!saved} onClick={onDone}>
        {t.continue}
      </button>
    </div>
  );
}

function Unlock({ api, initialError, onUnlocked }: { api: Backend; initialError?: string; onUnlocked: () => void }) {
  const [mode, setMode] = useState<"password" | "recovery">("password");
  const [value, setValue] = useState("");
  const [error, setError] = useState(initialError ?? "");
  const [busy, setBusy] = useState(false);

  async function submit(e: FormEvent) {
    e.preventDefault();
    setBusy(true);
    try {
      await api.unlock({ kind: mode, value });
      onUnlocked();
    } catch (err) {
      setError(errorText(err));
      setBusy(false);
    }
  }

  return (
    <form className="card" onSubmit={submit} data-testid="unlock">
      <h1>{t.unlockTitle}</h1>
      <div className="tabs" role="tablist">
        {(["password", "recovery"] as const).map((m) => (
          <button
            key={m}
            type="button"
            role="tab"
            aria-selected={mode === m}
            onClick={() => {
              setMode(m);
              setValue("");
            }}
          >
            {m === "password" ? t.unlockWithPassword : t.unlockWithRecovery}
          </button>
        ))}
      </div>
      <label>
        {mode === "password" ? t.password : t.recoveryKey}
        <input
          type={mode === "password" ? "password" : "text"}
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoFocus
          spellCheck={false}
        />
      </label>
      {error && <p className="error">{error}</p>}
      <button className="primary" disabled={busy || !value}>
        {t.open}
      </button>
    </form>
  );
}

// ---------------- Main window ----------------

const dateFormat = new Intl.DateTimeFormat(locale, { day: "numeric", month: "numeric", year: "numeric" });

function Main({
  api,
  demo,
  onLock,
  onDemoRemoved,
}: {
  api: Backend;
  demo: boolean;
  onLock: () => void;
  onDemoRemoved: () => void;
}) {
  const [confirmRemove, setConfirmRemove] = useState(false);
  const [notebooks, setNotebooks] = useState<NotebookInfo[]>([]);
  const [notebook, setNotebook] = useState<string | null>(null);
  const [query, setQuery] = useState("");
  const [list, setList] = useState<NoteInfo[]>([]);
  const [noteId, setNoteId] = useState<string | null>(null);
  const [addingNotebook, setAddingNotebook] = useState(false);
  const [importing, setImporting] = useState(false);
  const [outcomes, setOutcomes] = useState<ImportOutcome[] | null>(null);
  const [error, setError] = useState("");
  const [reload, setReload] = useState(0);
  const [showSettings, setShowSettings] = useState(false);

  const refreshNotebooks = useCallback(() => api.notebooks().then(setNotebooks), [api]);
  useEffect(() => {
    void refreshNotebooks();
  }, [refreshNotebooks, reload]);

  useEffect(() => {
    let cancelled = false;
    const timer = setTimeout(
      async () => {
        const result = query.trim() ? await api.search(query) : await api.notes(notebook);
        if (!cancelled) setList(result);
      },
      query.trim() ? 200 : 0,
    );
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [api, notebook, query, reload]);

  async function newNote() {
    const note = await api.newNote(notebook);
    setQuery("");
    setReload((r) => r + 1);
    setNoteId(note.id);
  }

  async function runImport() {
    setImporting(true);
    setError("");
    try {
      const result = await api.importEnex();
      if (result) {
        setOutcomes(result);
        setReload((r) => r + 1);
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      setImporting(false);
    }
  }

  const onSaved = useCallback((info: NoteInfo) => {
    setList((items) => {
      const rest = items.filter((i) => i.id !== info.id);
      return items.some((i) => i.id === info.id) ? [info, ...rest] : items;
    });
  }, []);

  return (
    <div className="layout">
      <aside className="sidebar" data-testid="sidebar">
        <div className="brand">{t.appName}</div>
        <button className="primary block" onClick={newNote} data-testid="new-note">
          + {t.newNote}
        </button>
        <nav>
          <button
            className={notebook === null && !query ? "nav active" : "nav"}
            onClick={() => {
              setNotebook(null);
              setQuery("");
            }}
          >
            {t.allNotes}
          </button>
          <div className="nav-heading">{t.notebooks}</div>
          {notebooks.map((nb) => (
            <button
              key={nb.id}
              className={notebook === nb.id && !query ? "nav active" : "nav"}
              onClick={() => {
                setNotebook(nb.id);
                setQuery("");
              }}
            >
              <span>{nb.name}</span>
              <span className="count">{nb.count}</span>
            </button>
          ))}
          {addingNotebook ? (
            <NewNotebook
              onCancel={() => setAddingNotebook(false)}
              onCreate={async (name) => {
                const nb = await api.createNotebook(name);
                setAddingNotebook(false);
                setNotebook(nb.id);
                setReload((r) => r + 1);
              }}
            />
          ) : (
            <button className="nav subtle" onClick={() => setAddingNotebook(true)}>
              + {t.newNotebook}
            </button>
          )}
        </nav>
        <div className="sidebar-bottom">
          {demo ? (
            <div className="demo-banner" data-testid="demo-banner">
              <strong>{t.demoTitle}</strong>
              <p className="small">{t.demoText}</p>
              <button className="primary block" onClick={() => setConfirmRemove(true)} data-testid="remove-demo">
                {t.removeDemo}
              </button>
            </div>
          ) : (
            <>
              <button className="block" onClick={runImport} disabled={importing} data-testid="import">
                {importing ? t.importing : t.importEvernote}
              </button>
              <button className="block subtle" onClick={() => setShowSettings(true)} data-testid="settings">
                {t.settings}
              </button>
            </>
          )}
          <button className="block subtle" onClick={onLock}>
            {t.lock}
          </button>
          {error && <p className="error small">{error}</p>}
        </div>
      </aside>

      <section className="list" data-testid="note-list">
        <input
          className="search"
          type="search"
          placeholder={t.search}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          data-testid="search"
        />
        {list.length === 0 && <p className="muted center-text">{query ? t.noResults : t.noNotes}</p>}
        <ul>
          {list.map((n) => (
            <li key={n.id}>
              <button className={n.id === noteId ? "note active" : "note"} onClick={() => setNoteId(n.id)}>
                <span className="note-title">{n.title || t.untitled}</span>
                <span className="note-date">{dateFormat.format(new Date(n.updated))}</span>
              </button>
            </li>
          ))}
        </ul>
      </section>

      <main className="editor-pane">
        {noteId ? (
          <NoteView
            key={noteId}
            api={api}
            noteId={noteId}
            onSaved={onSaved}
            onTrashed={() => {
              setNoteId(null);
              setReload((r) => r + 1);
            }}
          />
        ) : (
          <p className="muted center-text">{t.selectNote}</p>
        )}
      </main>

      {outcomes && <ImportReport outcomes={outcomes} onClose={() => setOutcomes(null)} />}
      {showSettings && <Settings api={api} onClose={() => setShowSettings(false)} />}
      {confirmRemove && (
        <div className="modal-backdrop" role="dialog" aria-modal="true" data-testid="confirm-remove-demo">
          <div className="modal">
            <h2>{t.removeDemoTitle}</h2>
            <p>{t.removeDemoText}</p>
            <div className="row">
              <button
                className="primary"
                onClick={async () => {
                  try {
                    await api.removeDemo();
                    onDemoRemoved();
                  } catch (e) {
                    setConfirmRemove(false);
                    setError(errorText(e));
                  }
                }}
              >
                {t.removeDemoConfirm}
              </button>
              <button onClick={() => setConfirmRemove(false)}>{t.cancel}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

function Settings({ api, onClose }: { api: Backend; onClose: () => void }) {
  const [status, setStatus] = useState<Status | null>(null);
  const [editing, setEditing] = useState(false);
  const [password, setPassword] = useState("");
  const [again, setAgain] = useState("");
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [recoveryKey, setRecoveryKey] = useState<string | null>(null);
  const [savedKey, setSavedKey] = useState(false);

  useEffect(() => {
    void api.status().then(setStatus);
  }, [api]);

  async function run(action: () => Promise<Status | void>, done: string) {
    setError("");
    setMessage("");
    try {
      const next = await action();
      if (next) setStatus(next);
      setMessage(done);
    } catch (e) {
      setError(errorText(e));
    }
  }

  if (recoveryKey) {
    return (
      <div className="modal-backdrop" role="dialog" aria-modal="true" data-testid="settings-dialog">
        <div className="modal">
          <h2>{t.newRecoveryKeyTitle}</h2>
          <p>{t.newRecoveryKeyText}</p>
          <p className="recovery-key" data-testid="new-recovery-key">
            {recoveryKey}
          </p>
          <label className="check">
            <input type="checkbox" checked={savedKey} onChange={(e) => setSavedKey(e.target.checked)} />
            {t.recoverySaved}
          </label>
          <button className="primary" disabled={!savedKey} onClick={() => setRecoveryKey(null)}>
            {t.continue}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" data-testid="settings-dialog">
      <div className="modal settings">
        <h2>{t.settings}</h2>
        {status && (
          <>
            <p className="muted small">
              {t.archiveFolder}: <code>{status.dir}</code>
            </p>
            <h3>{t.password}</h3>
            <p data-testid="password-state">{status.hasPassword ? t.passwordOn : t.passwordOff}</p>
            {status.hasPassword && !status.hasKeystore && <p className="muted small">{t.passwordRequired}</p>}
            {editing ? (
              <form
                className="stack"
                onSubmit={async (e) => {
                  e.preventDefault();
                  if (password !== again) return setError(t.passwordsDiffer);
                  await run(async () => {
                    const next = await api.setPassword(password);
                    setEditing(false);
                    setPassword("");
                    setAgain("");
                    return next;
                  }, t.passwordSet);
                }}
              >
                <label>
                  {t.newPassword}
                  <input type="password" value={password} onChange={(e) => setPassword(e.target.value)} autoFocus />
                </label>
                <label>
                  {t.passwordAgain}
                  <input type="password" value={again} onChange={(e) => setAgain(e.target.value)} />
                </label>
                <div className="row">
                  <button className="primary" disabled={!password}>
                    {t.savePassword}
                  </button>
                  <button type="button" onClick={() => setEditing(false)}>
                    {t.cancel}
                  </button>
                </div>
              </form>
            ) : (
              <div className="row">
                <button onClick={() => setEditing(true)}>{status.hasPassword ? t.changePassword : t.setPassword}</button>
                {status.hasPassword && status.hasKeystore && (
                  <button onClick={() => run(() => api.removePassword(), t.passwordRemoved)}>{t.removePassword}</button>
                )}
              </div>
            )}
            <h3>{t.recoveryKey}</h3>
            <p className="muted small">{t.recoveryKeyHint}</p>
            <div className="row">
              <button
                onClick={async () => {
                  setError("");
                  try {
                    setRecoveryKey(await api.newRecoveryKey());
                    setSavedKey(false);
                    setMessage(t.recoveryReplaced);
                  } catch (e) {
                    setError(errorText(e));
                  }
                }}
              >
                {t.createNewRecoveryKey}
              </button>
            </div>
          </>
        )}
        {message && (
          <p className="ok" data-testid="settings-message">
            {message}
          </p>
        )}
        {error && <p className="error">{error}</p>}
        <button className="primary" onClick={onClose}>
          {t.close}
        </button>
      </div>
    </div>
  );
}

function NewNotebook({ onCreate, onCancel }: { onCreate: (name: string) => Promise<void>; onCancel: () => void }) {
  const [name, setName] = useState("");
  const [error, setError] = useState("");
  return (
    <form
      className="new-notebook"
      onSubmit={async (e) => {
        e.preventDefault();
        try {
          await onCreate(name);
        } catch (err) {
          setError(errorText(err));
        }
      }}
    >
      <input autoFocus placeholder={t.notebookName} value={name} onChange={(e) => setName(e.target.value)} />
      <div className="row">
        <button className="primary small">{t.create}</button>
        <button type="button" className="small" onClick={onCancel}>
          {t.cancel}
        </button>
      </div>
      {error && <p className="error small">{error}</p>}
    </form>
  );
}

function ImportReport({ outcomes, onClose }: { outcomes: ImportOutcome[]; onClose: () => void }) {
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true" data-testid="import-report">
      <div className="modal">
        <h2>{t.importDone}</h2>
        {outcomes.map((o) => (
          <div key={o.file} className="outcome">
            <p>
              <strong>{o.file.split(/[\\/]/).pop()}</strong>: {t.importSummary(o.stored, o.attachments, o.notebook)}
            </p>
            {o.partial.length > 0 && (
              <details>
                <summary>
                  {t.partlyImported} {o.partial.length}
                </summary>
                <ul>
                  {o.partial.map((p, i) => (
                    <li key={i}>
                      {p.title || t.untitled}
                      <ul className="muted small">
                        {p.problems.map((problem, j) => (
                          <li key={j}>{problem}</li>
                        ))}
                      </ul>
                    </li>
                  ))}
                </ul>
              </details>
            )}
            {o.stoppedEarly && (
              <p className="error small">
                {t.stoppedEarly} {o.stoppedEarly}
              </p>
            )}
          </div>
        ))}
        <button className="primary" onClick={onClose} autoFocus>
          {t.close}
        </button>
      </div>
    </div>
  );
}

// ---------------- Editor ----------------

const TOOLS: { name: CommandName; label: string }[] = [
  { name: "bold", label: "B" },
  { name: "italic", label: "I" },
  { name: "underline", label: "U" },
  { name: "strike", label: "S" },
  { name: "heading1", label: "H1" },
  { name: "heading2", label: "H2" },
  { name: "paragraph", label: "¶" },
  { name: "bulletList", label: "•" },
  { name: "orderedList", label: "1." },
  { name: "checkList", label: "☑" },
  { name: "code", label: "</>" },
  { name: "undo", label: "↶" },
  { name: "redo", label: "↷" },
];

/** How long after the last change a note is saved. */
const SAVE_DELAY_MS = 600;

function NoteView({
  api,
  noteId,
  onSaved,
  onTrashed,
}: {
  api: Backend;
  noteId: string;
  onSaved: (info: NoteInfo) => void;
  onTrashed: () => void;
}) {
  const mount = useRef<HTMLDivElement>(null);
  const editor = useRef<NoteEditor | null>(null);
  const title = useRef("");
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const saveNow = useRef<() => Promise<void>>(async () => undefined);
  const [titleValue, setTitleValue] = useState("");
  const [loaded, setLoaded] = useState(false);
  const [saveState, setSaveState] = useState<"saved" | "pending" | "saving" | "error">("saved");

  useEffect(() => {
    let cancelled = false;
    saveNow.current = async () => {
      clearTimeout(timer.current);
      timer.current = undefined;
      const content = editor.current?.content();
      if (!content) return;
      setSaveState("saving");
      try {
        onSaved(await api.saveNote(noteId, title.current, content));
        if (!cancelled && timer.current === undefined) setSaveState("saved");
      } catch {
        if (!cancelled) setSaveState("error");
      }
    };
    const schedule = () => {
      setSaveState("pending");
      clearTimeout(timer.current);
      timer.current = setTimeout(() => void saveNow.current(), SAVE_DELAY_MS);
    };
    api.note(noteId).then((note) => {
      if (cancelled || !mount.current) return;
      title.current = note.title;
      setTitleValue(note.title);
      editor.current = createNoteEditor(mount.current, note.content, schedule, {
        load: (blob) => api.attachment(blob),
        save: (blob, name) => api.saveAttachment(blob, name),
        labels: { saveAs: t.saveAs, cannotLoad: t.cannotLoadAttachment },
      });
      setLoaded(true);
    });
    return () => {
      // Leaving the note: save what is still pending.
      if (timer.current !== undefined) void saveNow.current();
      cancelled = true;
      editor.current?.destroy();
      editor.current = null;
    };
  }, [api, noteId, onSaved]);

  const status = { saved: t.saved, pending: "", saving: t.saving, error: t.saveFailed }[saveState];

  return (
    <div className="note-view" data-testid="note-view">
      <div className="note-toolbar">
        <div className="tools" role="toolbar">
          {TOOLS.map((tool) => (
            <button
              key={tool.name}
              title={t.tools[tool.name]}
              aria-label={t.tools[tool.name]}
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => editor.current?.run(tool.name)}
            >
              {tool.label}
            </button>
          ))}
        </div>
        <button
          title={t.addAttachment}
          aria-label={t.addAttachment}
          onMouseDown={(e) => e.preventDefault()}
          onClick={async () => {
            const info = await api.addAttachment();
            if (info) editor.current?.insertAttachment(info);
          }}
          data-testid="add-attachment"
        >
          📎
        </button>
        <span className={saveState === "error" ? "save-state error" : "save-state"} data-testid="save-state">
          {status}
        </span>
        <button
          className="subtle"
          onClick={async () => {
            clearTimeout(timer.current);
            timer.current = undefined;
            await api.trashNote(noteId);
            onTrashed();
          }}
        >
          {t.trash}
        </button>
      </div>
      <input
        className="note-title-input"
        placeholder={t.titlePlaceholder}
        value={titleValue}
        disabled={!loaded}
        data-testid="title"
        onChange={(e) => {
          title.current = e.target.value;
          setTitleValue(e.target.value);
          setSaveState("pending");
          clearTimeout(timer.current);
          timer.current = setTimeout(() => void saveNow.current(), SAVE_DELAY_MS);
        }}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            editor.current?.view.focus();
          }
        }}
      />
      <div className="note-body" ref={mount} data-testid="editor" />
    </div>
  );
}
