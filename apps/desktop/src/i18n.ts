// Interface texts with Project Fluent (ADR-0004). The texts are in
// `locales/<language>.ftl`; Slovak is the source, every other file has the
// same messages (checked by `i18n.test.ts`). A message missing in a language
// falls back to English, then to Slovak.

import { FluentBundle, FluentResource, type FluentVariable } from "@fluent/bundle";
import { negotiateLanguages } from "@fluent/langneg";

import cs from "./locales/cs.ftl?raw";
import en from "./locales/en.ftl?raw";
import hu from "./locales/hu.ftl?raw";
import pl from "./locales/pl.ftl?raw";
import ru from "./locales/ru.ftl?raw";
import sk from "./locales/sk.ftl?raw";
import zh from "./locales/zh-CN.ftl?raw";

export const SOURCES: Record<string, string> = { sk, cs, en, hu, pl, ru, "zh-CN": zh };

/** Languages of the interface, each named in itself. */
export const LANGUAGES: { code: string; name: string }[] = [
  { code: "sk", name: "Slovenčina" },
  { code: "cs", name: "Čeština" },
  { code: "en", name: "English" },
  { code: "hu", name: "Magyar" },
  { code: "pl", name: "Polski" },
  { code: "ru", name: "Русский" },
  { code: "zh-CN", name: "简体中文" },
];

/** Locale for dates and sorting. */
const LOCALES: Record<string, string> = {
  sk: "sk-SK",
  cs: "cs-CZ",
  en: "en-GB",
  hu: "hu-HU",
  pl: "pl-PL",
  ru: "ru-RU",
  "zh-CN": "zh-CN",
};

/** Messages without variables, read as `t.<id>`. */
const MESSAGES = [
  "appName", "loading", "welcomeTitle", "welcomeText", "archiveFolder", "usePassword", "passwordRequired",
  "password", "passwordAgain", "passwordsDiffer", "createArchive", "recoveryTitle", "recoveryText", "copy",
  "copied", "recoverySaved", "continue", "unlockTitle", "unlockWithPassword", "unlockWithRecovery",
  "recoveryKey", "open", "allNotes", "notebooks", "newNotebook", "notebookName", "create", "cancel",
  "importEvernote", "importing", "importDone", "partlyImported", "stoppedEarly", "close", "search", "noNotes",
  "noResults", "newNote", "untitled", "titlePlaceholder", "saving", "saved", "saveFailed", "trash",
  "selectNote", "settings", "passwordOn", "passwordOff", "setPassword", "changePassword", "removePassword",
  "newPassword", "savePassword", "passwordSet", "passwordRemoved", "recoveryKeyHint", "createNewRecoveryKey",
  "newRecoveryKeyTitle", "newRecoveryKeyText", "recoveryReplaced", "or", "tryDemo", "tryDemoHint",
  "demoTitle", "demoText", "removeDemo", "removeDemoTitle", "removeDemoText", "removeDemoConfirm", "lock",
  "saveAs", "openAttachment", "addAttachment", "cannotLoadAttachment", "language", "languageAuto",
  "languageHint", "trashTitle", "trashEmpty", "emptyTrash", "emptyTrashTitle", "inTrash", "restore",
  "deleteForever", "deleteForeverTitle", "deleteForeverText",
] as const;

export type Message = (typeof MESSAGES)[number];

export type Texts = Record<Message, string> & {
  importSummary(notes: number, attachments: number, notebook: string): string;
  emptyTrashText(count: number): string;
  /** Editor toolbar, by tool name. */
  tools: Record<string, string>;
  /** Error messages, by error code from the core. */
  errors: Record<string, string>;
};

const STORAGE_KEY = "altais.language";

/** The language chosen in Settings, or null to follow the system. */
export function chosenLanguage(): string | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value && value in SOURCES ? value : null;
  } catch {
    return null;
  }
}

/** Remembers the language (null: follow the system); applies after reload. */
export function chooseLanguage(code: string | null): void {
  try {
    if (code) localStorage.setItem(STORAGE_KEY, code);
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Without storage the system language is used.
  }
}

/** The interface language for the system's preferred languages. Czech and
 * Slovak users understand each other's language, so each falls back to the
 * other before English. */
export function pickLanguage(requested: readonly string[]): string {
  const [best] = negotiateLanguages(requested, Object.keys(SOURCES), {
    strategy: "matching",
    defaultLocale: "en",
  });
  return best ?? "en";
}

export function makeBundles(lang: string): FluentBundle[] {
  const chain = [lang, lang === "cs" ? "sk" : lang === "sk" ? "cs" : "en", "en", "sk"];
  return [...new Set(chain)].map((code) => {
    // No bidirectional isolation marks: none of the languages is written right to left.
    const bundle = new FluentBundle(LOCALES[code], { useIsolating: false });
    bundle.addResource(new FluentResource(SOURCES[code]));
    return bundle;
  });
}

export function format(bundles: FluentBundle[], id: string, args?: Record<string, FluentVariable>): string | undefined {
  for (const bundle of bundles) {
    const message = bundle.getMessage(id);
    if (message?.value) return bundle.formatPattern(message.value, args);
  }
  return undefined;
}

export function makeTexts(lang: string): Texts {
  const bundles = makeBundles(lang);
  const texts = {} as Record<string, unknown>;
  for (const id of MESSAGES) texts[id] = format(bundles, id) ?? id;
  texts.importSummary = (notes: number, attachments: number, notebook: string) =>
    format(bundles, "importSummary", { notes, attachments, notebook }) ?? "";
  texts.emptyTrashText = (count: number) => format(bundles, "emptyTrashText", { count }) ?? "";
  const lookup = (prefix: string) =>
    new Proxy({} as Record<string, string>, {
      get: (_, key) => (typeof key === "string" ? format(bundles, `${prefix}-${key.replace(/_/g, "-")}`) : undefined),
    });
  texts.tools = lookup("tool");
  texts.errors = lookup("error");
  return texts as Texts;
}

const systemLanguages = typeof navigator !== "undefined" ? navigator.languages ?? [navigator.language] : [];

/** The interface language in use. */
export const language = chosenLanguage() ?? pickLanguage(systemLanguages.length ? systemLanguages : ["sk"]);
export const t: Texts = makeTexts(language);
export const locale = LOCALES[language];
if (typeof document !== "undefined") document.documentElement.lang = language;

export function errorText(e: unknown): string {
  if (typeof e === "object" && e !== null && "code" in e) {
    const code = String((e as { code: unknown }).code);
    return t.errors[code] ?? `${t.errors.unknown} (${(e as { message?: string }).message ?? code})`;
  }
  return `${t.errors.unknown} (${String(e)})`;
}
