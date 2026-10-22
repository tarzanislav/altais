// Cleans HTML pasted from Microsoft Word before ProseMirror parses it.
// Word writes lists as paragraphs with `mso-list` styles and a hidden bullet
// span; this turns them into real <ul>/<ol> lists and removes Office-only markup.

export function cleanPastedHtml(html: string): string {
  if (!/urn:schemas-microsoft-com|mso-|MsoNormal|MsoListParagraph/i.test(html)) return html;

  const parser = new DOMParser();
  const doc = parser.parseFromString(html, "text/html");
  doc.querySelectorAll("style, meta, link, title, xml").forEach((el) => el.remove());

  // Remove Office namespace elements such as <o:p>, keeping their text.
  for (const el of Array.from(doc.body.querySelectorAll("*"))) {
    if (el.tagName.includes(":")) el.replaceWith(...Array.from(el.childNodes));
  }

  const paragraphs = Array.from(doc.body.querySelectorAll("p"));
  let current: { level: number; list: HTMLElement }[] = [];
  let previous: Element | null = null;

  for (const p of paragraphs) {
    const style = p.getAttribute("style") ?? "";
    const match = /mso-list:\s*l\d+\s+level(\d+)/i.exec(style);
    if (!match) {
      current = [];
      previous = null;
      continue;
    }
    const level = Number(match[1]);
    // The bullet or number Word puts in front of the text.
    const marker = p.querySelector("span[style*='mso-list:Ignore'], span[style*='mso-list: Ignore']");
    const markerText = marker?.textContent?.trim() ?? "";
    marker?.remove();
    const ordered = /^[0-9a-zA-Z]{1,3}[.)]$/.test(markerText);

    const startsNewList = previous === null || p.previousElementSibling !== previous;
    if (startsNewList) current = [];

    while (current.length && current[current.length - 1].level > level) current.pop();
    let top = current[current.length - 1];
    if (!top || top.level < level) {
      const list = doc.createElement(ordered ? "ol" : "ul");
      if (top) {
        const parentItem = top.list.lastElementChild ?? top.list.appendChild(doc.createElement("li"));
        parentItem.appendChild(list);
      } else {
        p.before(list);
      }
      top = { level, list };
      current.push(top);
    }
    const li = doc.createElement("li");
    li.append(...Array.from(p.childNodes));
    top.list.appendChild(li);
    previous = p;
    // Keep a placeholder so the next paragraph can see it directly follows a list item.
    const placeholder = doc.createElement("p");
    placeholder.setAttribute("data-mso-list-placeholder", "");
    p.replaceWith(placeholder);
    previous = placeholder;
  }
  doc.body.querySelectorAll("p[data-mso-list-placeholder]").forEach((el) => el.remove());
  return doc.body.innerHTML;
}
