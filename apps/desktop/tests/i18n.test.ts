import { FluentResource } from "@fluent/bundle";
import { describe, expect, it } from "vitest";

import { LANGUAGES, makeBundles, makeTexts, pickLanguage, SOURCES } from "../src/i18n";

function ids(source: string): string[] {
  const resource = new FluentResource(source);
  return resource.body.map((m) => m.id).sort();
}

describe("interface texts", () => {
  it("every language has exactly the Slovak messages and parses cleanly", () => {
    const expected = ids(SOURCES.sk);
    for (const { code } of LANGUAGES) {
      const source = SOURCES[code];
      expect(source, code).toBeTruthy();
      // Junk entries (syntax errors) have no id and would show up here.
      expect(ids(source), code).toEqual(expected);
      expect(source.includes("�"), code).toBe(false);
    }
  });

  it("no message is missing or empty in any language", () => {
    for (const { code } of LANGUAGES) {
      const [own] = makeBundles(code);
      for (const id of ids(SOURCES.sk)) {
        const message = own.getMessage(id);
        expect(message?.value, `${code} ${id}`).toBeTruthy();
        expect(own.formatPattern(message!.value!, { notes: 2, attachments: 2, notebook: "X" }).trim(), `${code} ${id}`).not.toBe("");
      }
      const t = makeTexts(code);
      expect(t.errors.wrong_password, code).toBeTruthy();
      expect(t.tools.bold, code).toBeTruthy();
      expect(t.errors.no_such_code, code).toBeUndefined();
    }
  });

  it("numbers take the right form", () => {
    expect(makeTexts("sk").importSummary(1, 2, "A")).toBe("1 poznámka a 2 prílohy do zošita „A“");
    expect(makeTexts("sk").importSummary(5, 0, "A")).toBe("5 poznámok a 0 príloh do zošita „A“");
    expect(makeTexts("cs").importSummary(3, 1, "B")).toBe("3 poznámky a 1 příloha do sešitu „B“");
    expect(makeTexts("pl").importSummary(22, 5, "C")).toBe("22 notatki i 5 załączników do notesu „C”");
    expect(makeTexts("ru").importSummary(21, 3, "D")).toBe("21 заметка и 3 вложения в блокнот «D»");
    expect(makeTexts("en").importSummary(1, 1, "E")).toBe("1 note and 1 attachment into “E”");
  });

  it("the system language picks the interface language", () => {
    expect(pickLanguage(["sk-SK"])).toBe("sk");
    expect(pickLanguage(["cs-CZ", "en-US"])).toBe("cs");
    expect(pickLanguage(["hu-HU"])).toBe("hu");
    expect(pickLanguage(["pl"])).toBe("pl");
    expect(pickLanguage(["ru-RU"])).toBe("ru");
    expect(pickLanguage(["zh-CN"])).toBe("zh-CN");
    expect(pickLanguage(["zh"])).toBe("zh-CN");
    expect(pickLanguage(["de-DE"])).toBe("en");
    expect(pickLanguage(["de-DE", "sk"])).toBe("sk");
  });
});
