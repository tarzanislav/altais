import { expect, type Page, test } from "@playwright/test";

type Block = {
  t: string;
  bid?: string;
  text?: { s: string; marks?: unknown[] }[];
  content?: Block[];
  [key: string]: unknown;
};
type Which = "a" | "b";

const adm = (page: Page, which: Which) => page.evaluate((w) => window.spike.adm(w), which) as Promise<Block[]>;

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

const heading = (blocks: Block[]) => blocks[0].text;
const quoteText = (blocks: Block[]) => blocks.find((b) => b.t === "quote")?.content?.[0].text;

async function waitForSync(page: Page) {
  await expect(page.getByTestId("status")).toHaveText("In sync");
}

/** Waits until a device shows the expected value (typing reaches ProseMirror asynchronously). */
async function expectOn(page: Page, which: Which, pick: (blocks: Block[]) => unknown, expected: unknown) {
  await expect.poll(async () => pick(await adm(page, which))).toEqual(expected);
}

async function expectOnBoth(page: Page, pick: (blocks: Block[]) => unknown, expected: unknown) {
  await expectOn(page, "a", pick, expected);
  await expectOn(page, "b", pick, expected);
}

/** Clicks "Update now" on a device once changes are waiting there. */
async function update(page: Page, which: Which) {
  const bar = page.getByTestId(`syncbar-${which}`);
  await expect(bar).toBeVisible();
  await bar.getByRole("button", { name: "Update now" }).click();
  await expect(bar).toBeHidden();
}

/** Clicks into an editor and checks that it really has the keyboard focus. */
async function clickInto(page: Page, which: Which, selector: string) {
  const editor = page.getByTestId(`editor-${which}`);
  await editor.locator(selector).click();
  await expect(editor.locator(".ProseMirror")).toBeFocused();
}

const consoleLines = new WeakMap<Page, string[]>();

async function openPage(page: Page, idleMs: number) {
  const errors: string[] = [];
  const lines: string[] = [];
  consoleLines.set(page, lines);
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => lines.push(`${message.type()}: ${message.text()}`));
  await page.goto(`/?idle=${idleMs}`);
  await expect(page.getByTestId("editor-a").locator("h1")).toHaveText("Výlet");
  await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet");
  await waitForSync(page);
  expect(errors).toEqual([]);
}

// On failure, print the state and what the page logged; CI keeps only this output.
test.afterEach(async ({ page }, testInfo) => {
  if (testInfo.status === testInfo.expectedStatus) return;
  const lines = consoleLines.get(page) ?? [];
  const state = await page.evaluate(() => {
    const focus =
      document.activeElement?.closest("[data-testid]")?.getAttribute("data-testid") ?? document.activeElement?.tagName;
    const describe = (which: "a" | "b") => {
      const device = window.spike[which];
      const view = device.editor?.view;
      if (!view) return `${which}: closed, ${device.inbox.count} waiting`;
      const { from, to } = view.state.selection;
      return `${which}: selection ${from}-${to} of ${view.state.doc.content.size}; ${device.inbox.count} waiting; text ${JSON.stringify(view.state.doc.textContent)}`;
    };
    return `focus ${focus}\n${describe("a")}\n${describe("b")}`;
  });
  // Thrown so that the GitHub reporter shows it as an annotation.
  throw new Error(`Diagnostics (browser ${page.context().browser()?.version()}):\n${state}\n${lines.slice(-30).join("\n")}`);
});

test.describe("with manual updates", () => {
  // A long pause, so changes only apply when a test asks for it.
  test.beforeEach(async ({ page }) => openPage(page, 600_000));

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

  test("changes from the other device wait until Update now", async ({ page }) => {
    await clickInto(page, "a", "h1");
    await page.keyboard.press("End");
    await page.keyboard.type(" do Tatier");
    await expectOn(page, "a", heading, [{ s: "Výlet do Tatier" }]);

    const bar = page.getByTestId("syncbar-b");
    await expect(bar).toContainText("from another device waiting");
    await expect(page.getByTestId("status")).toHaveText("Changes waiting on a device");
    await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet"); // not changed yet

    await update(page, "b");
    await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet do Tatier");
    await waitForSync(page);
  });

  test("Enter creates a new block with its own ID on both devices", async ({ page }) => {
    await clickInto(page, "a", "h1");
    await page.keyboard.press("End");
    await page.keyboard.press("Enter");
    await page.keyboard.type("Nový odsek");
    await expectOn(page, "a", (blocks) => [blocks[1].t, blocks[1].text], ["paragraph", [{ s: "Nový odsek" }]]);
    await update(page, "b");
    await expectOn(page, "b", (blocks) => [blocks[1].t, blocks[1].text], ["paragraph", [{ s: "Nový odsek" }]]);
    const bids = allBids(await adm(page, "b"));
    expect(new Set(bids).size).toBe(bids.length);
    await waitForSync(page);
  });

  test("bold shortcut produces a bold span", async ({ page }) => {
    await clickInto(page, "a", "blockquote p");
    await page.keyboard.press("End");
    await page.keyboard.press("ControlOrMeta+b");
    await page.keyboard.type(" Áno!");
    await expectOn(page, "a", quoteText, [{ s: "Hory volajú." }, { s: " Áno!", marks: ["bold"] }]);
    await update(page, "b");
    await expectOn(page, "b", quoteText, [{ s: "Hory volajú." }, { s: " Áno!", marks: ["bold"] }]);
  });

  test("clicking a checkbox ticks the item on both devices", async ({ page }) => {
    const second = page.getByTestId("editor-a").locator("ul[data-kind='check'] > li").nth(1);
    await expect(second).toHaveAttribute("data-checked", "false");
    const box = await second.boundingBox();
    if (!box) throw new Error("no box");
    await page.mouse.click(box.x - 16, box.y + box.height / 2);
    await expect(second).toHaveAttribute("data-checked", "true");
    await update(page, "b");
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
    await expectOn(page, "a", heading, [{ s: "Výlet A" }]);

    await clickInto(page, "b", "blockquote p");
    await page.keyboard.press("End");
    await page.keyboard.type(" B");
    await expectOn(page, "b", quoteText, [{ s: "Hory volajú. B" }]);

    await expect(page.getByTestId("status")).toHaveText("Offline: changes in transit");
    await expect(page.getByTestId("syncbar-b")).toBeHidden(); // nothing has arrived
    await page.getByLabel("Online").check();
    await update(page, "a");
    await update(page, "b");
    await waitForSync(page);

    await expectOnBoth(page, heading, [{ s: "Výlet A" }]);
    await expectOnBoth(page, quoteText, [{ s: "Hory volajú. B" }]);
  });

  test("undo reverts only this device's change", async ({ page }) => {
    await clickInto(page, "b", "blockquote p");
    await page.keyboard.press("End");
    await page.keyboard.type(" (B)");
    await expectOn(page, "b", quoteText, [{ s: "Hory volajú. (B)" }]);
    await update(page, "a");

    await clickInto(page, "a", "h1");
    await page.keyboard.press("End");
    await page.keyboard.type("!");
    await expectOn(page, "a", heading, [{ s: "Výlet!" }]);
    await page.keyboard.press("ControlOrMeta+z");
    await expectOn(page, "a", heading, [{ s: "Výlet" }]);
    await expectOn(page, "a", quoteText, [{ s: "Hory volajú. (B)" }]);
    await update(page, "b");
    await waitForSync(page);
  });

  test("Word paste becomes a real list and drops Office markup", async ({ page }) => {
    const html = `<html xmlns:o="urn:schemas-microsoft-com:office:office"><head><style>p.MsoNormal{margin:0}</style></head><body>
<p class="MsoNormal">Úvod<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level1 lfo1"><span style="mso-list:Ignore">·<span>&nbsp;&nbsp;</span></span>prvý<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level2 lfo1"><span style="mso-list:Ignore">o<span>&nbsp;</span></span>vnorený<o:p></o:p></p>
<p class="MsoListParagraph" style="mso-list:l0 level1 lfo1"><span style="mso-list:Ignore">·<span>&nbsp;</span></span><b>druhý</b><o:p></o:p></p>
<p class="MsoNormal"><span style="color:windowtext">Koniec</span></p>
</body></html>`;
    await page.getByTestId("editor-a").locator("hr").click(); // select the divider; the paste replaces it
    await page.evaluate((pasted) => {
      const target = document.querySelector("[data-testid='editor-a'] .ProseMirror") as HTMLElement;
      const data = new DataTransfer();
      data.setData("text/html", pasted);
      data.setData("text/plain", "Úvod prvý vnorený druhý Koniec");
      target.dispatchEvent(new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true }));
    }, html);
    await update(page, "b");
    await expectOn(page, "b", (blocks) => blocks.some((b) => b.text?.[0]?.s === "Koniec"), true);

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

  test("writing continues undisturbed while changes wait, and the cursor stays after Update", async ({ page }) => {
    await page.evaluate(() => window.spike.placeCursor("b", "Hory volajú.", 4)); // after "Hory"
    await page.evaluate(() => window.spike.placeCursor("a", "Hory volajú.", 12)); // at the end
    await page.keyboard.type(" A");
    await expectOn(page, "a", quoteText, [{ s: "Hory volajú. A" }]);

    // B keeps writing in the same paragraph; A's change waits.
    await page.getByTestId("editor-b").locator(".ProseMirror").focus();
    await page.keyboard.type("X");
    await expectOn(page, "b", quoteText, [{ s: "HoryX volajú." }]);

    await update(page, "b");
    await expectOn(page, "b", quoteText, [{ s: "HoryX volajú. A" }]);
    await page.keyboard.type("Y"); // the editor has the focus back, cursor still after X
    await expectOn(page, "b", quoteText, [{ s: "HoryXY volajú. A" }]);

    await update(page, "a");
    await expectOnBoth(page, quoteText, [{ s: "HoryXY volajú. A" }]);
    await waitForSync(page);
  });

  test("a click right after Update stays where it was made", async ({ page }) => {
    await clickInto(page, "a", "blockquote p");
    await page.keyboard.press("End");
    await page.keyboard.type(" A");
    await expectOn(page, "a", quoteText, [{ s: "Hory volajú. A" }]);

    await page.getByTestId("syncbar-b").getByRole("button", { name: "Update now" }).click();
    // No waiting: click into the heading immediately and type.
    await clickInto(page, "b", "h1");
    await page.keyboard.press("End");
    await page.keyboard.type("!");
    await expectOn(page, "b", heading, [{ s: "Výlet!" }]);
    await expectOn(page, "b", quoteText, [{ s: "Hory volajú. A" }]);
  });
});

test.describe("automatic updates", () => {
  test("waiting changes apply after a pause in typing, not during it", async ({ page }) => {
    await openPage(page, 2_500);
    // B starts writing.
    await clickInto(page, "b", "blockquote p");
    await page.keyboard.press("End");
    await page.keyboard.type("0");

    // Meanwhile A's user (on another computer) edits the heading. Done through
    // the editor API: the test has one keyboard, and typing into A would be
    // a pause for B.
    await page.evaluate(() => {
      const view = window.spike.a.editor!.view;
      view.dispatch(view.state.tr.insertText(" A", 1 + "Výlet".length));
    });
    await expect(page.getByTestId("syncbar-b")).toBeVisible();

    // B keeps typing steadily for about 3 s, so the pause never comes.
    for (let i = 1; i <= 8; i++) {
      await page.keyboard.type(String(i));
      await page.waitForTimeout(400);
    }
    await expect(page.getByTestId("syncbar-b")).toBeVisible();
    expect(heading(await adm(page, "b"))).toEqual([{ s: "Výlet" }]);

    // Then B stops typing: the change applies by itself.
    await expect(page.getByTestId("syncbar-b")).toBeHidden({ timeout: 6_000 });
    await expectOn(page, "b", heading, [{ s: "Výlet A" }]);
    await expectOn(page, "b", quoteText, [{ s: "Hory volajú.012345678" }]);
    expect(await page.evaluate(() => window.spike.b.applied.map((a) => a.reason))).toEqual(["idle"]);
  });

  test("closing the app applies waiting changes; opening it synchronizes what came meanwhile", async ({ page }) => {
    await openPage(page, 600_000);
    await clickInto(page, "a", "h1");
    await page.keyboard.press("End");
    await page.keyboard.type(" 1");
    await expect(page.getByTestId("syncbar-b")).toBeVisible();

    await page.getByTestId("power-b").click(); // close B
    await expect(page.getByTestId("notice-b")).toContainText("Device B is closed");
    await expectOn(page, "b", heading, [{ s: "Výlet 1" }]); // applied on close

    await clickInto(page, "a", "h1");
    await page.keyboard.press("End");
    await page.keyboard.type("2");
    await expectOn(page, "b", heading, [{ s: "Výlet 1" }]); // B is closed; the change waits

    await page.getByTestId("power-b").click(); // open B
    await expect(page.getByTestId("notice-b")).toHaveText("Synchronized 1 change from other devices.");
    await expect(page.getByTestId("editor-b").locator("h1")).toHaveText("Výlet 12");
    expect(await page.evaluate(() => window.spike.b.applied.map((a) => a.reason))).toEqual(["close", "open"]);
    await waitForSync(page);
  });
});
