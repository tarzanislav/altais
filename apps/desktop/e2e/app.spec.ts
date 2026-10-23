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
  await expect(report).toContainText("2 poznámok a 0 príloh do zošita „Ukážka z Evernote“");
  await report.getByRole("button", { name: "Zavrieť" }).click();

  const sidebar = page.getByTestId("sidebar");
  await sidebar.getByRole("button", { name: /Ukážka z Evernote\s*2/ }).click();
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
  await expect(sidebar.getByRole("button", { name: /Ukážka z Evernote\s*3/ })).toBeVisible();

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
