#!/usr/bin/env python3
"""Generates the ALTAIS demo packages (ADR-0005): resources/demo/altais-demo-<lang>.json.

All texts and images are written for ALTAIS and released under CC0 (see
README.md here). Run from the repository root: python3 resources/demo/generate.py
The output is deterministic, so regenerating without changes gives identical files.
"""
import base64
import io
import json
import struct
import zlib
from pathlib import Path

try:
    from blake3 import blake3  # type: ignore
except ImportError:  # pragma: no cover
    blake3 = None

OUT = Path(__file__).resolve().parent


# ---------------- tiny PNG writer (no third-party libraries) ----------------

def png(width, height, pixel):
    """A PNG image; pixel(x, y) returns (r, g, b)."""
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(pixel(x, y))
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    return (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b""))


def mountains(x, y):
    """A simple landscape: sky gradient, two mountain ridges, a lake."""
    w, h = 320, 180
    sky = (int(120 + 100 * y / h), int(170 + 60 * y / h), 235)
    ridge1 = 110 - abs((x % 160) - 80) * 0.9
    ridge2 = 130 - abs(((x + 60) % 120) - 60) * 0.7
    if y > 150:
        return (40, 90, 150) if (x + y) % 7 else (90, 140, 200)
    if y > ridge2:
        return (60, 95, 70)
    if y > ridge1:
        return (110, 115, 125) if y > ridge1 + 12 else (240, 240, 245)
    return sky


IMAGE = png(320, 180, mountains)


def blob_ref(data):
    if blake3 is None:
        raise SystemExit("pip install blake3 (needed to compute content addresses)")
    return "blake3:" + blake3(data).hexdigest()


# ---------------- ADM helpers ----------------

class Note:
    def __init__(self):
        self.n = 0

    def bid(self):
        self.n += 1
        return f"d{self.n:04d}"


def spans(text):
    """Inline marks with a light syntax: **bold**, *italic*, `code`, ==highlight==."""
    out, i, plain = [], 0, ""
    marks = [("**", "bold"), ("==", "highlight"), ("`", "code"), ("*", "italic")]
    while i < len(text):
        for tok, mark in marks:
            if text.startswith(tok, i):
                end = text.find(tok, i + len(tok))
                if end > 0:
                    if plain:
                        out.append({"s": plain}); plain = ""
                    out.append({"s": text[i + len(tok):end], "marks": [mark]})
                    i = end + len(tok)
                    break
        else:
            plain += text[i]; i += 1
    if plain:
        out.append({"s": plain})
    return out


def p(n, text): return {"bid": n.bid(), "t": "paragraph", "text": spans(text)}
def h(n, level, text): return {"bid": n.bid(), "t": "heading", "level": level, "text": spans(text)}
def quote(n, text): return {"bid": n.bid(), "t": "quote", "content": [p(n, text)]}
def code(n, lang, text): return {"bid": n.bid(), "t": "code", "lang": lang, "text": text}
def divider(n): return {"bid": n.bid(), "t": "divider"}
def bookmark(n, url, title): return {"bid": n.bid(), "t": "bookmark", "url": url, "title": title}


def items(n, kind, entries):
    out = []
    for e in entries:
        if isinstance(e, tuple) and kind == "check":
            text, checked = e
            out.append({"bid": n.bid(), "checked": checked, "content": [p(n, text)]})
        elif isinstance(e, list):  # nested list under the previous item
            out[-1]["content"].append(items(n, "bullet", e))
        else:
            out.append({"bid": n.bid(), "content": [p(n, e)]})
    return {"bid": n.bid(), "t": "list", "kind": kind, "items": out}


def table(n, rows, header=True):
    out = []
    for r, row in enumerate(rows):
        cells = []
        for cell in row:
            c = {"content": [p(n, cell)]}
            if header and r == 0:
                c["header"] = True
            cells.append(c)
        out.append({"cells": cells})
    return {"bid": n.bid(), "t": "table", "rows": out}


def attachment(n, data, name, mime):
    return {"bid": n.bid(), "t": "attachment", "blob": blob_ref(data), "name": name, "mime": mime, "size": len(data)}


# ---------------- content ----------------
# Each entry: (notebook key, tags, age in days, {lang: (title, blocks-builder)})

TEXT_FILE = {
    "sk": "Zoznam vecí na výlet\n- pevná obuv\n- pršiplášť\n- mapa\n".encode(),
    "en": "Packing list\n- hiking boots\n- rain jacket\n- map\n".encode(),
}


def content(lang):
    sk = lang == "sk"
    T = (lambda s, e: s if sk else e)
    notebooks = [
        ("start", T("Začíname s ALTAIS", "Getting started")),
        ("travel", T("Cestovanie", "Travel")),
        ("food", T("Recepty", "Recipes")),
        ("work", T("Práca", "Work")),
        ("langs", T("Jazyky", "Languages")),
    ]
    notes = []

    def add(nb, tags, age, title, build):
        n = Note()
        notes.append({"notebook": nb, "title": title, "tags": tags, "ageDays": age, "content": build(n)})

    add("start", [T("návod", "guide")], 0, T("Vitaj v ALTAIS", "Welcome to ALTAIS"), lambda n: [
        h(n, 1, T("Vitaj!", "Welcome!")),
        p(n, T("Toto sú **ukážkové poznámky**. Môžeš ich upravovať, hľadať v nich a skúšať všetko, čo ALTAIS vie.",
               "These are **demo notes**. Edit them, search them and try everything ALTAIS can do.")),
        p(n, T("Keď budeš pripravený, zvoľ vľavo dole *Odstrániť ukážku* a začni s vlastnými poznámkami. Z ukážky v programe nič neostane.",
               "When you are ready, choose *Remove demo* at the bottom left and start with your own notes. Nothing of the demo stays in the app.")),
        h(n, 2, T("Čo vyskúšať", "Things to try")),
        items(n, "check", [(T("Otvoriť poznámku a upraviť ju", "Open a note and edit it"), True),
                           (T("Vyhľadať slovo bez diakritiky, napríklad „vylet“", "Search for a word, for example “lake”"), False),
                           (T("Vytvoriť nový zošit", "Create a new notebook"), False),
                           (T("Pridať prílohu tlačidlom 📎", "Add an attachment with the 📎 button"), False)]),
        quote(n, T("Poznámky sú uložené šifrovane na tvojom počítači.", "Notes are stored encrypted on your computer.")),
    ])
    add("start", [T("návod", "guide")], 0, T("Formátovanie textu", "Text formatting"), lambda n: [
        h(n, 1, T("Nadpis 1", "Heading 1")), h(n, 2, T("Nadpis 2", "Heading 2")), h(n, 3, T("Nadpis 3", "Heading 3")),
        p(n, T("Text môže byť **tučný**, *kurzíva*, ==zvýraznený== alebo `kód`.",
               "Text can be **bold**, *italic*, ==highlighted== or `code`.")),
        p(n, T("Klávesové skratky: Ctrl+B tučné, Ctrl+I kurzíva, Ctrl+U podčiarknuté, Ctrl+Z späť.",
               "Shortcuts: Ctrl+B bold, Ctrl+I italic, Ctrl+U underline, Ctrl+Z undo.")),
        divider(n),
        p(n, T("Emoji a symboly fungujú tiež: ☀️ 🏔️ ✓ € § ©", "Emoji and symbols work too: ☀️ 🏔️ ✓ € § ©")),
    ])
    add("start", [T("návod", "guide")], 1, T("Zoznamy a úlohy", "Lists and tasks"), lambda n: [
        p(n, T("Odrážky s vnorením:", "Bullets with nesting:")),
        items(n, "bullet", [T("Ovocie", "Fruit"), [T("jablká", "apples"), T("hrušky", "pears")], T("Zelenina", "Vegetables")]),
        p(n, T("Číslovaný zoznam:", "Numbered list:")),
        items(n, "ordered", [T("Prvý krok", "First step"), T("Druhý krok", "Second step"), T("Tretí krok", "Third step")]),
        p(n, T("Zoznam úloh – klikni na štvorček:", "Checklist – click the box:")),
        items(n, "check", [(T("Hotová úloha", "Done task"), True), (T("Úloha na neskôr", "Task for later"), False)]),
    ])
    add("start", [T("návod", "guide")], 1, T("Tabuľky a kód", "Tables and code"), lambda n: [
        table(n, [[T("Deň", "Day"), T("Plán", "Plan"), T("Poznámka", "Note")],
                  [T("Pondelok", "Monday"), T("Plávanie", "Swimming"), "18:00"],
                  [T("Streda", "Wednesday"), T("Jazyk", "Language course"), T("online", "online")]]),
        p(n, T("Blok kódu zachová medzery a odsadenie:", "A code block keeps spaces and indentation:")),
        code(n, "python", "def pozdrav(meno):\n    return f\"Ahoj, {meno}!\"" if sk else "def greet(name):\n    return f\"Hello, {name}!\""),
        code(n, "sql", "SELECT title FROM note WHERE notebook = 'travel';"),
    ])
    add("start", [T("návod", "guide")], 2, T("Hľadanie bez diakritiky", "Search tips"), lambda n: [
        p(n, T("Hľadanie ignoruje veľké písmená a diakritiku: „strbske pleso“ nájde „Štrbské Pleso“.",
               "Search ignores case and accents: “cafe” finds “Café”.")),
        p(n, T("Slová sa hľadajú ako začiatky slov: „vyl“ nájde „výlet“. Presnú frázu daj do úvodzoviek.",
               "Words match as prefixes: “moun” finds “mountain”. Put an exact phrase in quotes.")),
        p(n, T("Funguje to aj pre češtinu, maďarčinu, poľštinu, ruštinu a čínštinu – pozri zošit Jazyky.",
               "It works for Czech, Hungarian, Polish, Russian and Chinese too – see the Languages notebook.")),
    ])

    add("travel", [T("výlet", "trip"), T("hory", "mountains")], 3, T("Výlet na Štrbské Pleso", "Trip to the lake"), lambda n: [
        h(n, 1, T("Štrbské Pleso", "Mountain lake")),
        p(n, T("Odchod o 7:00, vlak z Popradu. Okruh okolo plesa trvá asi hodinu.",
               "Leave at 7:00 by train. The walk around the lake takes about an hour.")),
        attachment(n, IMAGE, T("vyhlad.png", "view.png"), "image/png"),
        items(n, "check", [(T("Kúpiť lístky", "Buy tickets"), True), (T("Zbaliť batoh", "Pack the backpack"), False)]),
        attachment(n, TEXT_FILE[lang], T("zoznam-veci.txt", "packing-list.txt"), "text/plain"),
    ])
    add("travel", [T("výlet", "trip")], 6, T("Plán dovolenky", "Holiday plan"), lambda n: [
        table(n, [[T("Deň", "Day"), T("Miesto", "Place"), T("Ubytovanie", "Stay")],
                  ["1", T("Bratislava", "Vienna"), T("hotel pri Dunaji", "hotel by the Danube")],
                  ["2", T("Banská Štiavnica", "Salzburg"), T("penzión", "guesthouse")],
                  ["3", T("Vysoké Tatry", "Hallstatt"), T("chata", "cabin")]]),
        bookmark(n, "https://example.com/map", T("Mapa trasy", "Route map")),
    ])
    add("travel", [T("výlet", "trip"), T("hory", "mountains")], 12, T("Výstup na Rysy", "Hike to the summit"), lambda n: [
        p(n, T("Prevýšenie 1 100 m, čas výstupu asi 4 hodiny. Chata pod Rysmi má najvyššie položenú reštauráciu.",
               "Elevation gain 1,100 m, about 4 hours up. The hut near the top serves hot tea.")),
        quote(n, T("Hory sa nikam nesťahujú. Počasie áno.", "The mountains are not going anywhere. The weather is.")),
    ])

    add("food", [T("recept", "recipe")], 4, T("Bryndzové halušky", "Potato dumplings with cheese"), lambda n: [
        h(n, 2, T("Suroviny", "Ingredients")),
        items(n, "bullet", [T("1 kg zemiakov", "1 kg potatoes"), T("300 g hladkej múky", "300 g flour"),
                            T("250 g bryndze", "250 g sheep cheese"), T("100 g slaniny", "100 g bacon")]),
        h(n, 2, T("Postup", "Steps")),
        items(n, "ordered", [T("Zemiaky ošúp a nastrúhaj.", "Peel and grate the potatoes."),
                             T("Primiešaj múku a soľ.", "Mix in the flour and salt."),
                             T("Halušky hádž do vriacej vody.", "Drop the dough into boiling water."),
                             T("Zmiešaj s bryndzou a posyp slaninkou.", "Mix with the cheese, top with bacon.")]),
    ])
    add("food", [T("recept", "recipe"), T("koláč", "cake")], 9, T("Jablkový koláč", "Apple cake"), lambda n: [
        p(n, T("Rýchly koláč na plech: **4 jablká**, 3 vajcia, 200 g cukru, 250 g múky, prášok do pečiva.",
               "A quick sheet cake: **4 apples**, 3 eggs, 200 g sugar, 250 g flour, baking powder.")),
        p(n, T("Pečieme 35 minút pri 180 °C.", "Bake 35 minutes at 180 °C.")),
    ])
    add("food", [T("nákup", "shopping")], 0, T("Nákupný zoznam", "Shopping list"), lambda n: [
        items(n, "check", [(T("mlieko", "milk"), False), (T("chlieb", "bread"), True), (T("vajcia", "eggs"), False),
                           (T("káva", "coffee"), False)]),
    ])

    add("work", [T("porada", "meeting")], 2, T("Porada tímu – zápis", "Team meeting – minutes"), lambda n: [
        h(n, 2, T("Prítomní", "Present")),
        p(n, T("Jana, Peter, Lucia", "Anna, Peter, Lucy")),
        h(n, 2, T("Rozhodnutia", "Decisions")),
        items(n, "bullet", [T("Nová verzia vyjde v piatok.", "The new version ships on Friday."),
                            T("Testovanie preberá Lucia.", "Lucy takes over testing.")]),
        h(n, 2, T("Úlohy", "Action items")),
        items(n, "check", [(T("Pripraviť poznámky k vydaniu", "Write release notes"), False),
                           (T("Aktualizovať dokumentáciu", "Update the documentation"), False)]),
    ])
    add("work", [T("projekt", "project")], 5, T("Projekt Atlas – prehľad", "Project Atlas – overview"), lambda n: [
        table(n, [[T("Fáza", "Phase"), T("Termín", "Due"), T("Stav", "Status")],
                  [T("Návrh", "Design"), "15. 3." if sk else "15 Mar", T("hotové", "done")],
                  [T("Vývoj", "Build"), "30. 6." if sk else "30 Jun", T("prebieha", "in progress")],
                  [T("Spustenie", "Launch"), "1. 9." if sk else "1 Sep", T("plán", "planned")]]),
        quote(n, T("Cieľ: jednoduchá aplikácia, ktorú ľudia radi používajú.", "Goal: a simple app people enjoy using.")),
    ])
    add("work", [T("nápady", "ideas")], 8, T("Nápady na zlepšenie", "Improvement ideas"), lambda n: [
        items(n, "bullet", [T("Tmavý režim pre prezentácie", "Dark mode for presentations"),
                            T("Kratšie porady – najviac 30 minút", "Shorter meetings – 30 minutes at most"),
                            T("Spoločný slovník pojmov", "A shared glossary")]),
    ])

    add("langs", ["čeština" if sk else "Czech"], 3, "Čeština", lambda n: [
        p(n, "Příliš žluťoučký kůň úpěl ďábelské ódy."),
        p(n, T("Skús hľadať „zlutoucky kun“.", "Try searching for “zlutoucky kun”.")),
    ])
    add("langs", ["magyar" if sk else "Hungarian"], 3, "Magyar", lambda n: [
        p(n, "Árvíztűrő tükörfúrógép."),
        p(n, T("Skús hľadať „arvizturo“.", "Try searching for “arvizturo”.")),
    ])
    add("langs", ["polski" if sk else "Polish"], 3, "Polski", lambda n: [
        p(n, "Zażółć gęślą jaźń. Spotkanie w Łodzi o 10:00."),
        p(n, T("Skús hľadať „lodzi“ – ł sa hľadá ako l.", "Try searching for “lodzi” – ł matches l.")),
    ])
    add("langs", ["русский" if sk else "Russian"], 3, "Русский", lambda n: [
        p(n, "Съешь же ещё этих мягких французских булок, да выпей чаю."),
        p(n, T("Skús hľadať „еще“ – ё sa hľadá ako е.", "Try searching for “еще” – ё matches е.")),
    ])
    add("langs", ["中文" if sk else "Chinese"], 3, "中文", lambda n: [
        p(n, "加密笔记很重要，同步也很方便。"),
        p(n, T("Skús hľadať „笔记“ – nájde sa aj uprostred vety.", "Try searching for “笔记” – found inside the sentence.")),
    ])
    add("langs", [], 4, T("Prázdna poznámka", "Empty note"), lambda n: [])

    return notebooks, notes


def build(lang):
    notebooks, notes = content(lang)
    blobs = {}
    for data, name, mime in [(IMAGE, "image", "image/png"), (TEXT_FILE[lang], "text", "text/plain")]:
        blobs[blob_ref(data)] = base64.b64encode(data).decode()
    package = {
        "format": "altais-demo",
        "version": 1,
        "lang": lang,
        "licence": "CC0-1.0",
        "notebooks": [{"key": k, "name": name} for k, name in notebooks],
        "notes": notes,
        "blobs": blobs,
    }
    path = OUT / f"altais-demo-{lang}.json"
    path.write_text(json.dumps(package, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"{path.name}: {len(notes)} notes, {len(blobs)} attachments, {path.stat().st_size // 1024} kB")


if __name__ == "__main__":
    for lang in ("sk", "en"):
        build(lang)
