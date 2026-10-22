import { expect, type Page, test } from "@playwright/test";

type Block = {
  t: string;
  bid?: string;
  text?: { s: string; marks?: unknown[] }[];
  content?: Block[];
  [key: string]: unknown;
};

const adm = (page: Page, which: "a" | "b") => page.evaluate((w) => window.spike.adm(w), which) as Promise<Block[]>;

function allBids(blocks: unknown): string[] {
  const out: string[] = [];
  const walk = (value: unknown) => {
    if (Array.isArray(value)) value.forEach(walk);
    else if (value && typeof value === "object") {
      const record = value as Record<string, unknown>;
      if (typeof record.bid === "string") out.push(record.bid);
      Object.values(record).forEach(walk);
    }
  };
  walk(blocks);
  return out;
}

async function waitForSync(page: Page) {
  await expect(page.getByRole("status")).toHaveText("In sync");
}

const heading = (blocks: Block[]) => blocks[0].text;
const quoteText = (blocks: Block[]) => blocks.find((b) => b.t === "quote")?.content?.[0].text;

/**
 * Waits until both devices show the expected value. Keyboard input reaches
 * ProseMirror asynchronously, so a single read right after typing can be stale.
 */
async function expectOnBoth(page: Page, pick: (blocks: Block[]) => unknown, expected: unknown) {
  for (const which of ["a", "b"] as const) {
    await expect.poll(async () => pick(await adm(page, which))).toEqual(expected);
  }
}

const consoleLines = new WeakMap<Page, string[]>();

test.beforeEach(async ({ page }) => {
  const errors: string[] = [];
  const lines: string[] = [];
  consoleLines.set(page, lines);
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => lines.push(`${message.type()}: ${message.text()}`));
  page.on("load", () => lines.push("page loaded"));
  await page.goto("/");
  await expect(page.getByTestId("editor-a").locator("h1")).toHaveText("Výlet");
  await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet");
  await waitForSync(page);
  expect(errors).toEqual([]);
});

// On failure, print what the page logged; CI keeps only this output.
test.afterEach(async ({ page }, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus) return;
  const lines = consoleLines.get(page) ?? [];
  const state = await page.evaluate(() => {
    const focus = document.activeElement?.closest("[data-testid]")?.getAttribute("data-testid") ?? document.activeElement?.tagName;
    const describe = (which: "a" | "b") => {
      const view = window.spike[which].view;
      const { from, to } = view.state.selection;
      return `${which}: selection ${from}-${to} of ${view.state.doc.content.size}; text ${JSON.stringify(view.state.doc.textContent)}`;
    };
    return `focus ${focus}\n${describe("a")}\n${describe("b")}`;
  });
  // Thrown so that the GitHub reporter shows it as an annotation.
  throw new Error(`Diagnostics (browser ${page.context().browser()?.version()}):\n${state}\n${lines.slice(-30).join("\n")}`);
});

/**
 * Lets loro-prosemirror finish restoring cursors after remote changes. It
 * restores them in a timer, and a restore that fires after a click moves the
 * cursor away from where the user clicked (a known issue, see README). The
 * two nested timers run after any restore timer already queued.
 */
async function settle(page: Page) {
  await page.evaluate(() => new Promise<void>((resolve) => setTimeout(() => setTimeout(resolve, 0), 0)));
}

/** Clicks into an editor and checks that it really has the keyboard focus. */
async function clickInto(page: Page, which: "a" | "b", selector: string) {
  await settle(page);
  const editor = page.getByTestId(`editor-${which}`);
  await editor.locator(selector).click();
  await expect(editor.locator(".ProseMirror")).toBeFocused();
}

test("shows every block type of the example note", async ({ page }) => {
  const editor = page.getByTestId("editor-a");
  await expect(editor.locator("sub")).toHaveText("2");
  await expect(editor.locator("sup")).toHaveText("2");
  await expect(editor.locator("ul[data-kind='check'] > li")).toHaveCount(2);
  await expect(editor.locator("ol ul li")).toHaveText("raňajky");
  await expect(editor.locator("table th")).toHaveCount(2);
  await expect(editor.locator("td[colspan='2']")).toHaveText("celý deň voľno");
  await expect(editor.locator("blockquote")).toHaveText("Hory volajú.");
  await expect(editor.locator("pre")).toContainText('"km": 12');
  await expect(editor.locator("hr")).toHaveCount(1);
  await expect(editor.locator(".bookmark a")).toHaveAttribute("href", "https://example.com/video");
  await expect(editor.locator(".unknown")).toContainText("evernote_encrypted");
});

test("typing on one device appears on the other", async ({ page }) => {
  await clickInto(page, "a", "h1");
  await page.keyboard.press("End");
  await page.keyboard.type(" do Tatier");
  await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet do Tatier");
  await waitForSync(page);
});

test("Enter creates a new block with its own ID on both devices", async ({ page }) => {
  await clickInto(page, "a", "h1");
  await page.keyboard.press("End");
  await page.keyboard.press("Enter");
  await page.keyboard.type("Nový odsek");
  await expectOnBoth(page, (blocks) => [blocks[1].t, blocks[1].text], ["paragraph", [{ s: "Nový odsek" }]]);
  const blocks = await adm(page, "b");
  const bids = allBids(blocks);
  expect(new Set(bids).size).toBe(bids.length);
});

test("bold shortcut produces a bold span", async ({ page }) => {
  await clickInto(page, "a", "blockquote p");
  await page.keyboard.press("End");
  await page.keyboard.press("ControlOrMeta+b");
  await page.keyboard.type(" Áno!");
  await expectOnBoth(page, quoteText, [{ s: "Hory volajú." }, { s: " Áno!", marks: ["bold"] }]);
});

test("clicking a checkbox ticks the item on both devices", async ({ page }) => {
  const second = page.getByTestId("editor-a").locator("ul[data-kind='check'] > li").nth(1);
  await expect(second).toHaveAttribute("data-checked", "false");
  const box = await second.boundingBox();
  if (!box) throw new Error("no box");
  await page.mouse.click(box.x - 16, box.y + box.height / 2);
  await expect(second).toHaveAttribute("data-checked", "true");
  await expect(page.getByTestId("editor-b").locator("ul[data-kind='check'] > li").nth(1)).toHaveAttribute(
    "data-checked",
    "true",
  );
});

test("concurrent offline edits merge when back online", async ({ page }) => {
  await page.getByLabel("Online").uncheck();

  await clickInto(page, "a", "h1");
  await page.keyboard.press("End");
  await page.keyboard.type(" A");

  await clickInto(page, "b", "blockquote p");
  await page.keyboard.press("End");
  await page.keyboard.type(" B");
  await expect.poll(async () => quoteText(await adm(page, "b"))).toEqual([{ s: "Hory volajú. B" }]);

  await expect(page.getByRole("status")).toHaveText("Offline: devices differ");
  expect(heading(await adm(page, "b"))).toEqual([{ s: "Výlet" }]); // A's edit has not arrived
  await page.getByLabel("Online").check();
  await waitForSync(page);

  await expectOnBoth(page, heading, [{ s: "Výlet A" }]);
  await expectOnBoth(page, quoteText, [{ s: "Hory volajú. B" }]);
});

test("undo reverts only this device's change", async ({ page }) => {
  await clickInto(page, "b", "blockquote p");
  await page.keyboard.press("End");
  await page.keyboard.type(" (B)");
  await clickInto(page, "a", "h1");
  await page.keyboard.press("End");
  await page.keyboard.type("!");
  await expectOnBoth(page, heading, [{ s: "Výlet!" }]);
  await page.keyboard.press("ControlOrMeta+z");
  await expectOnBoth(page, heading, [{ s: "Výlet" }]);
  await expectOnBoth(page, quoteText, [{ s: "Hory volajú. (B)" }]);
});

test("Word paste becomes a real list and drops Office markup", async ({ page }) => {
  const html = `<html xmlns:o="urn:schemas-microsoft-com:office:office"><head><style>p.MsoNormal{margin:0}</style></head><body>
<p class="MsoNormal">Úvod<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level1 lfo1"><span style="mso-list:Ignore">·<span>&nbsp;&nbsp;</span></span>prvý<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level2 lfo1"><span style="mso-list:Ignore">o<span>&nbsp;</span></span>vnorený<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level1 lfo1"><span style="mso-list:Ignore">·<span>&nbsp;</span></span><b>druhý</b><o:p></o:p></p>
<p class="MsoNormal"><span style="color:windowtext">Koniec</span></p>
</body></html>`;
  const editor = page.getByTestId("editor-a");
  await editor.locator("hr").click(); // select the divider; the paste replaces it
  await page.evaluate((pasted) => {
    const target = document.querySelector("[data-testid='editor-a'] .ProseMirror") as HTMLElement;
    const data = new DataTransfer();
    data.setData("text/html", pasted);
    data.setData("text/plain", "Úvod prvý vnorený druhý Koniec");
    target.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
  }, html);
  await expectOnBoth(page, (blocks) => blocks.some((b) => b.text?.[0]?.s === "Koniec"), true);

  const blocks = await adm(page, "b");
  const intro = blocks.findIndex((b) => b.t === "paragraph" && b.text?.[0]?.s === "Úvod");
  expect(intro).toBeGreaterThan(-1);
  const list = blocks[intro + 1] as { t: string; kind: string; items: { content: Block[] }[] };
  expect(list).toMatchObject({ t: "list", kind: "bullet" });
  expect(list.items).toHaveLength(2);
  expect(list.items[0].content[0].text).toEqual([{ s: "prvý" }]);
  expect(list.items[0].content[1]).toMatchObject({ t: "list", items: [{ content: [{ text: [{ s: "vnorený" }] }] }] });
  expect(list.items[1].content[0].text).toEqual([{ s: "druhý", marks: ["bold"] }]);
  expect(blocks[intro + 2]).toMatchObject({ t: "paragraph", text: [{ s: "Koniec" }] });
  expect(JSON.stringify(blocks)).not.toMatch(/mso|Ignore|·/);
});

test("the other device keeps its cursor when the same paragraph changes", async ({ page }) => {
  // The cursors are placed through the editor API: where a mouse click lands
  // differs between Chromium versions, and this test is about merging.
  await page.evaluate(() => window.spike.placeCursor("b", "Hory volajú.", 4)); // after "Hory"
  await page.evaluate(() => window.spike.placeCursor("a", "Hory volajú.", 12)); // at the end
  await page.keyboard.type(" A");
  await expectOnBoth(page, quoteText, [{ s: "Hory volajú. A" }]);
  await settle(page);

  await page.getByTestId("editor-b").locator(".ProseMirror").focus();
  await page.keyboard.type("X");
  await expectOnBoth(page, quoteText, [{ s: "HoryX volajú. A" }]);
});
