import { expect, type Page, test } from "@playwright/test";

async function createArchive(page: Page, password?: string) {
  await page.goto("/");
  await expect(page.getByTestId("welcome")).toBeVisible();
  if (password) {
    await page.getByLabel("Chrániť archív aj heslom").check();
    await page.getByLabel("Heslo", { exact: true }).fill(password);
    await page.getByLabel("Heslo ešte raz").fill(password);
  }
  await page.getByRole("button", { name: "Vytvoriť archív" }).click();
  await expect(page.getByTestId("recovery-key")).toContainText("ALTAIS-");
  const next = page.getByRole("button", { name: "Pokračovať" });
  await expect(next).toBeDisabled(); // the user must confirm the key is saved
  await page.getByLabel("Záchranný kľúč mám bezpečne uložený").check();
  await next.click();
  await expect(page.getByTestId("sidebar")).toBeVisible();
}

test("first run, Evernote import, editing, search and trash", async ({ page }) => {
  await createArchive(page);

  await page.getByTestId("import").click();
  const report = page.getByTestId("import-report");
  await expect(report).toContainText("3 poznámky a 1 príloha do zošita „Ukážka z Evernote“");
  await report.getByRole("button", { name: "Zavrieť" }).click();

  const sidebar = page.getByTestId("sidebar");
  await sidebar.getByRole("button", { name: /Ukážka z Evernote\s*3/ }).click();
  await page.getByTestId("note-list").getByRole("button", { name: /Plán výletu na Štrbské Pleso/ }).click();
  const editor = page.getByTestId("editor");
  await expect(editor.locator("h1")).toHaveText("Výlet");
  await expect(editor.locator("ul[data-kind='check'] > li")).toHaveCount(2);

  // Edit the title and the text; the note saves by itself.
  await page.getByTestId("title").fill("Výlet do Tatier");
  await editor.locator("blockquote p").click();
  await page.keyboard.press("End");
  await page.keyboard.type(" Bryndzové halušky!");
  await expect(page.getByTestId("save-state")).toHaveText("Uložené");
  await expect(page.getByTestId("note-list")).toContainText("Výlet do Tatier");

  // Search ignores diacritics and case.
  await page.getByTestId("search").fill("bryndzove");
  await expect(page.getByTestId("note-list").getByRole("button", { name: /Výlet do Tatier/ })).toBeVisible();
  await page.getByTestId("search").fill("");

  // A new note in the selected notebook.
  await page.getByTestId("new-note").click();
  await page.getByTestId("title").fill("Nákupný zoznam");
  await page.keyboard.press("Enter");
  await page.keyboard.type("mlieko, chlieb");
  await expect(page.getByTestId("save-state")).toHaveText("Uložené");
  await expect(sidebar.getByRole("button", { name: /Ukážka z Evernote\s*4/ })).toBeVisible();

  // Switching notes right after typing still saves the text.
  await page.keyboard.type(", syr");
  await page.getByTestId("note-list").getByRole("button", { name: /Druhá poznámka/ }).click();
  await page.getByTestId("search").fill("syr");
  await expect(page.getByTestId("note-list").getByRole("button", { name: /Nákupný zoznam/ })).toBeVisible();
  await page.getByTestId("note-list").getByRole("button", { name: /Nákupný zoznam/ }).click();
  await page.getByRole("button", { name: "Do koša" }).click();
  await expect(page.getByTestId("note-list")).toContainText("Nič sa nenašlo");
});

test("a password protects the archive; wrong passwords are refused", async ({ page }) => {
  await createArchive(page, "Hory volajú");
  await page.getByRole("button", { name: "Zamknúť" }).click();
  await expect(page.getByTestId("unlock")).toBeVisible();
  await page.getByLabel("Heslo").fill("hory volajú");
  await page.getByRole("button", { name: "Otvoriť" }).click();
  await expect(page.getByTestId("unlock")).toContainText("Nesprávne heslo.");
  await page.getByLabel("Heslo").fill("Hory volajú");
  await page.getByRole("button", { name: "Otvoriť" }).click();
  await expect(page.getByTestId("sidebar")).toBeVisible();
});

test("notebooks can be created", async ({ page }) => {
  await createArchive(page);
  await page.getByRole("button", { name: "+ Nový zošit" }).click();
  await page.getByPlaceholder("Názov zošita").fill("Recepty");
  await page.getByRole("button", { name: "Vytvoriť", exact: true }).click();
  await expect(page.getByTestId("sidebar").getByRole("button", { name: /Recepty\s*0/ })).toBeVisible();
});

test("pictures show in the note; attachments can be saved and added", async ({ page }) => {
  await createArchive(page);
  await page.getByTestId("import").click();
  await page.getByTestId("import-report").getByRole("button", { name: "Zavrieť" }).click();
  await page.getByTestId("note-list").getByRole("button", { name: /Fotka z výletu/ }).click();

  const editor = page.getByTestId("editor");
  const img = editor.locator(".attachment-image img");
  await expect(img).toHaveAttribute("alt", "rysy.png");
  await expect.poll(() => img.evaluate((i: HTMLImageElement) => i.naturalWidth)).toBe(40);
  const gpx = editor.locator(".attachment-file", { hasText: "trasa.gpx" });
  await expect(gpx).toBeVisible();

  await gpx.getByRole("button", { name: "Uložiť ako…" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { savedAttachments?: string[] }).savedAttachments?.[0] ?? "")).toContain("trasa.gpx");

  // Opening in the default program; programs can only be saved.
  await gpx.getByRole("button", { name: "Otvoriť" }).click();
  await expect.poll(() => page.evaluate(() => (window as unknown as { openedAttachments?: string[] }).openedAttachments?.[0] ?? "")).toBe("trasa.gpx");
  const exe = editor.locator(".attachment-file", { hasText: "instalacia.exe" });
  await expect(exe.getByRole("button", { name: "Uložiť ako…" })).toBeVisible();
  await expect(exe.getByRole("button", { name: "Otvoriť" })).toHaveCount(0);

  // Add a file at the end of the note; it saves with the note.
  await editor.locator("p").first().click();
  await page.keyboard.press("End");
  await page.getByTestId("add-attachment").click();
  await expect(editor.locator(".attachment-file", { hasText: "zoznam.txt" })).toBeVisible();
  await expect(page.getByTestId("save-state")).toHaveText("Uložené");
});

test("settings: set, change and remove the password; replace the recovery key", async ({ page }) => {
  await createArchive(page);
  await page.getByTestId("settings").click();
  const dialog = page.getByTestId("settings-dialog");
  await expect(dialog.getByTestId("password-state")).toContainText("Heslo nie je nastavené");

  await dialog.getByRole("button", { name: "Nastaviť heslo" }).click();
  await dialog.getByLabel("Nové heslo").fill("Lomnický štít");
  await dialog.getByLabel("Heslo ešte raz").fill("Lomnicky stit");
  await dialog.getByRole("button", { name: "Uložiť heslo" }).click();
  await expect(dialog).toContainText("Heslá sa nezhodujú.");
  await dialog.getByLabel("Heslo ešte raz").fill("Lomnický štít");
  await dialog.getByRole("button", { name: "Uložiť heslo" }).click();
  await expect(dialog.getByTestId("settings-message")).toHaveText("Heslo je nastavené.");
  await expect(dialog.getByTestId("password-state")).toContainText("chránený heslom");

  // A new recovery key must be confirmed as saved.
  await dialog.getByRole("button", { name: "Vytvoriť nový záchranný kľúč" }).click();
  await expect(dialog.getByTestId("new-recovery-key")).toContainText("ALTAIS-");
  const next = dialog.getByRole("button", { name: "Pokračovať" });
  await expect(next).toBeDisabled();
  await dialog.getByLabel("Záchranný kľúč mám bezpečne uložený").check();
  await next.click();

  await dialog.getByRole("button", { name: "Zavrieť" }).click();
  await page.getByRole("button", { name: "Zamknúť" }).click();
  await page.getByLabel("Heslo").fill("Lomnický štít");
  await page.getByRole("button", { name: "Otvoriť" }).click();
  await expect(page.getByTestId("sidebar")).toBeVisible();

  await page.getByTestId("settings").click();
  await dialog.getByRole("button", { name: "Zrušiť heslo" }).click();
  await expect(dialog.getByTestId("password-state")).toContainText("Heslo nie je nastavené");
});

test("demo data: try ALTAIS, then remove the demo completely and start", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("try-demo").click();
  await expect(page.getByTestId("demo-banner")).toContainText("Ukážkové dáta");
  const sidebar = page.getByTestId("sidebar");
  for (const name of ["Začíname s ALTAIS", "Cestovanie", "Recepty", "Práca", "Jazyky"]) {
    await expect(sidebar.getByRole("button", { name: new RegExp(name) })).toBeVisible();
  }
  // Own data and key settings are not offered in the demo.
  await expect(page.getByTestId("import")).toHaveCount(0);
  await expect(page.getByTestId("settings")).toHaveCount(0);

  await page.getByTestId("search").fill("笔记");
  await expect(page.getByTestId("note-list").getByRole("button", { name: /中文/ })).toBeVisible();
  await page.getByTestId("search").fill("strbske");
  await page.getByTestId("note-list").getByRole("button", { name: /Štrbské Pleso/ }).click();
  await expect.poll(() => page.getByTestId("editor").locator(".attachment-image img").evaluate((i: HTMLImageElement) => i.naturalWidth)).toBe(320);

  await page.getByTestId("remove-demo").click();
  await page.getByTestId("confirm-remove-demo").getByRole("button", { name: "Odstrániť ukážku" }).click();
  await expect(page.getByTestId("welcome")).toBeVisible();

  // The own archive starts empty: nothing of the demo is left.
  await page.getByRole("button", { name: "Vytvoriť archív" }).click();
  await page.getByLabel("Záchranný kľúč mám bezpečne uložený").check();
  await page.getByRole("button", { name: "Pokračovať" }).click();
  await expect(page.getByTestId("note-list")).toContainText("Žiadne poznámky");
  await expect(sidebar.getByRole("button", { name: /Cestovanie/ })).toHaveCount(0);
});

test("the interface language can be chosen in Settings", async ({ page }) => {
  await createArchive(page);
  await page.getByTestId("settings").click();
  const dialog = page.getByTestId("settings-dialog");
  await dialog.getByTestId("language").selectOption("hu");
  await expect(dialog.getByTestId("settings-message")).toHaveText("Zmena sa prejaví po opätovnom spustení ALTAIS.");
  await page.reload();
  await expect(page.getByRole("heading", { name: "Üdvözöl az ALTAIS" })).toBeVisible();
  await expect(page.locator("html")).toHaveAttribute("lang", "hu");
});

test.describe("system language", () => {
  test.use({ locale: "ru-RU" });
  test("follows the system language", async ({ page }) => {
    await page.goto("/");
    await expect(page.getByRole("heading", { name: "Добро пожаловать в ALTAIS" })).toBeVisible();
  });
});
