"""Generates the synthetic ENEX test fixtures in this directory.

All content is invented for ALTAIS and released under CC0. The structures
follow what Evernote 10/11 and older versions write. Run from this directory:

    python3 generate.py
"""
import base64
import hashlib

# A 1x1 transparent PNG and a minimal one-page PDF, both created for these tests.
PNG = base64.b64decode(
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYAAAAAYAAjCB0C8AAAAASUVORK5CYII="
)
PDF = (
    b"%PDF-1.4\n1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
    b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
    b"3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 200 200]>>endobj\n"
    b"trailer<</Root 1 0 R>>\n%%EOF\n"
)
TEXT = "Toto je vymyslená príloha pre testy ALTAIS.\n".encode("utf-8")


def md5(data):
    return hashlib.md5(data).hexdigest()


def b64(data):
    encoded = base64.b64encode(data).decode("ascii")
    # Evernote wraps base64 lines; the importer must ignore the line breaks.
    return "\n".join(encoded[i : i + 76] for i in range(0, len(encoded), 76))


HEADER = (
    '<?xml version="1.0" encoding="UTF-8"?>\n'
    '<!DOCTYPE en-export SYSTEM "http://xml.evernote.com/pub/evernote-export4.dtd">\n'
)
ENML_DOCTYPE = (
    '<?xml version="1.0" encoding="UTF-8" standalone="no"?>'
    '<!DOCTYPE en-note SYSTEM "http://xml.evernote.com/pub/enml2.dtd">'
)


def note(title, body, created="20261020T080000Z", updated="20261021T193000Z", extra=""):
    return (
        "  <note>\n"
        f"    <title>{title}</title>\n"
        f"    <created>{created}</created>\n"
        f"    <updated>{updated}</updated>\n"
        f"{extra}"
        f"    <content><![CDATA[{ENML_DOCTYPE}<en-note>{body}</en-note>]]></content>\n"
    )


def resource(data, mime, name, extra_attrs=""):
    return (
        "    <resource>\n"
        f"      <data encoding=\"base64\">\n{b64(data)}\n      </data>\n"
        f"      <mime>{mime}</mime>\n"
        "      <resource-attributes>\n"
        f"        <file-name>{name}</file-name>\n"
        f"{extra_attrs}"
        "      </resource-attributes>\n"
        "    </resource>\n"
    )


HIDDEN_SETTINGS = '<div style="display:none;--en-chs:&quot;eyJoMSI6e319&quot;"></div>'

# Evernote 11 structures, as observed in a real export.
evernote11 = HEADER + '<en-export export-date="20261022T120000Z" application="Evernote" version="11.39.2">\n'
evernote11 += note(
    "Formátovanie",
    HIDDEN_SETTINGS
    + "<div><br/></div>"
    + '<h1 style="--en-nodeId:11111111-2222-4333-8444-555555555555;"><span style="font-size: 25px;">Výlet&nbsp;do hôr</span></h1>'
    + "<div><i>kurzíva</i> a "
    + '<span style="color:rgb(229, 158, 37);--inversion-type-color:simple;">farebný text</span>, '
    + "<u>podčiarknuté</u>, "
    + '<span style="--en-highlight:purple;background-color: #edf0ff;">zvýraznené</span>, '
    + '<a href="https://example.com/trasa" rev="en_rl_none">odkaz</a>, '
    + "<s>prečiarknuté</s> a <b>tučné</b>.</div>"
    + "<div>H<sub>2</sub>O a m<sup>2</sup></div>"
    + '<div style="--en-richlink:true; --en-href:https://example.com/video; --en-viewAs:youtube-video-small;">'
    + '<a href="https://example.com/video" rev="en_rl_small">Video o trase</a></div>'
    + '<div style="--en-codeblock:true;"><div>{</div><div>  "km": 12</div><div>}</div></div>',
) + "  </note>\n"
evernote11 += note(
    "Zoznamy",
    '<ul style="--en-todo:true;">'
    + '<li style="--en-checked:true;"><div>Kúpiť lístky</div></li>'
    + '<li style="--en-checked:false;"><div>Zbaliť batoh</div></li>'
    + "</ul>"
    + '<ul style="--en-todo:true;"><ul style="--en-todo:true;"><li style="--en-checked:false;"><h3>Odsadená úloha</h3></li></ul></ul>'
    + "<ol><li><div>Prvý</div></li><li><div>Druhý</div><ul><li>vnorený</li></ul></li></ol>",
) + "  </note>\n"
evernote11 += note(
    "Tabuľka a prílohy",
    '<table width="400px" style="border-collapse:collapse;"><colgroup><col style="width: 200px;" /><col style="width: 200px;" /></colgroup>'
    + "<tbody><tr><td><div>Čas</div></td><td><div><b>Miesto</b></div></td></tr>"
    + '<tr><td colspan="2"><div>celý deň voľno</div></td></tr></tbody></table>'
    + f'<en-media hash="{md5(PNG)}" type="image/png" style="--en-naturalWidth:1; --en-naturalHeight:1;" />'
    + f'<div>Leták: <en-media style="--en-viewAs:pdf-pageByPage;" hash="{md5(PDF)}" type="application/pdf" /> koniec.</div>',
)
evernote11 += resource(
    PNG,
    "image/png",
    "bod.png",
    '        <source-url>https://example.com/bod.png</source-url>\n'
    '        <application-data key="evernote.caption">Priehľadný bod</application-data>\n',
)
evernote11 += resource(PDF, "application/pdf", "letak.pdf")
evernote11 += "  </note>\n</en-export>\n"

# Older Evernote structures and edge cases.
legacy = HEADER + '<en-export export-date="20150101T000000Z" application="Evernote/Windows" version="6.x">\n'
legacy += note(
    "Staršie prvky",
    '<div><en-todo checked="true"/>Hotová úloha</div>'
    + "<div><en-todo/>Otvorená úloha</div>"
    + "<blockquote><div>Citát</div></blockquote>"
    + "<hr/>"
    + "<pre>let x = 1;\nlet y = 2;</pre>"
    + '<en-crypt cipher="AES" length="128" hint="obvyklé">RU5DMI1mnQ7fKjBkvymT8g==</en-crypt>'
    + '<div><font color="#FF0000">červené</font> a <a href="evernote:///view/1/s1/abc/abc/">interný odkaz</a></div>'
    + "<div>Neznámy znak &foo; ostane.</div>"
    + "<div><blink>neznámy prvok</blink></div>"
    + '<en-media hash="0123456789abcdef0123456789abcdef" type="image/png" />',
    created="20150101T000000Z",
    updated="20150102T000000Z",
    extra=(
        "    <tag>práca</tag>\n"
        "    <tag>projekt</tag>\n"
        "    <tag>práca</tag>\n"
        "    <note-attributes>\n"
        "      <author>Ján Vzorový</author>\n"
        "      <source-url>https://example.sk/clanok</source-url>\n"
        "      <reminder-time>20150105T090000Z</reminder-time>\n"
        "    </note-attributes>\n"
        "    <task>\n"
        "      <title>Zavolať do servisu</title>\n"
        "      <status>open</status>\n"
        "      <dueDate>20150110T090000Z</dueDate>\n"
        "    </task>\n"
    ),
)
legacy += resource(TEXT, "text/plain", "poznamka.txt")
legacy += "  </note>\n</en-export>\n"

# A valid note, a note with a damaged body, and a file cut off in the third note.
damaged = HEADER + '<en-export export-date="20261022T120000Z" application="Evernote" version="11.39.2">\n'
damaged += note("V poriadku", "<div>Táto poznámka je v poriadku.</div>") + "  </note>\n"
damaged += note("Poškodené telo", "<div>Neuzavretý <b>prvok</div>") + "  </note>\n"
damaged += "  <note>\n    <title>Odrezaná</title>\n    <content><![CDATA[<en-note><div>začiatok"

for name, text in [
    ("evernote11.enex", evernote11),
    ("legacy.enex", legacy),
    ("damaged.enex", damaged),
]:
    with open(name, "w", encoding="utf-8", newline="\n") as f:
        f.write(text)
    print("wrote", name)
