/* Turns the editable Markdown preview back into Markdown source.
   Only the elements produced by `renderMarkdown` are understood; anything
   else falls back to its text content so no note can be corrupted. */

function escapeText(text: string): string {
  return text.replace(/([\\`*_[\]])/g, "\\$1");
}

function inline(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return escapeText((node.textContent ?? "").replace(/\u00a0/g, " "));
  }
  if (node.nodeType !== Node.ELEMENT_NODE) return "";
  const el = node as HTMLElement;
  const tag = el.tagName.toLowerCase();
  const children = (): string => Array.from(el.childNodes).map(inline).join("");
  switch (tag) {
    case "br":
      return "\n";
    case "strong":
    case "b":
      return `**${children()}**`;
    case "em":
    case "i":
      return `*${children()}*`;
    case "del":
    case "s":
      return `~~${children()}~~`;
    case "code":
      return `\`${el.textContent ?? ""}\``;
    case "a": {
      const href = el.getAttribute("href") ?? "";
      return `[${children()}](${href})`;
    }
    case "img": {
      const alt = el.getAttribute("alt") ?? "";
      return `![${alt}](${el.getAttribute("src") ?? ""})`;
    }
    case "span": {
      if (el.classList.contains("md-image")) {
        const image = el.querySelector("img");
        const source = el.dataset.src ?? image?.getAttribute("src") ?? "";
        const alt = image?.getAttribute("alt") ?? "";
        const width = Number(el.dataset.w ?? "0");
        const rotation = Number(el.dataset.r ?? "0");
        const options: string[] = [];
        if (width > 0) options.push(`w=${Math.round(width)}`);
        if (rotation !== 0) options.push(`r=${((rotation % 360) + 360) % 360}`);
        const suffix = options.length > 0 ? `{${options.join(" ")}}` : "";
        return `![${alt}](${source})${suffix}`;
      }
      if (el.classList.contains("md-check")) return "";
      return children();
    }
    default:
      return children();
  }
}

function listBlock(list: Element): string {
  const ordered = list.tagName.toLowerCase() === "ol";
  const items: string[] = [];
  let counter = 1;
  for (const item of Array.from(list.children)) {
    if (item.tagName.toLowerCase() !== "li") continue;
    const indent = item.classList.contains("md-nested") ? "  " : "";
    const marker = ordered ? `${counter}.` : "-";
    counter += 1;
    let body: string;
    if (item.classList.contains("md-task")) {
      const check = item.querySelector(".md-check");
      const done = !!check && check.classList.contains("on");
      const text = item.querySelector(".md-task-text") ?? item;
      body = `[${done ? "x" : " "}] ${inline(text).trim()}`;
    } else {
      body = inline(item).trim();
    }
    items.push(`${indent}${marker} ${body}`);
  }
  return items.join("\n");
}

function tableBlock(table: Element): string {
  const rows = Array.from(table.querySelectorAll("tr")).map((row) =>
    Array.from(row.children).map((cell) => inline(cell).trim().replace(/\|/g, "\\|")),
  );
  const header = rows[0];
  if (!header || header.length === 0) return "";
  const separator = header.map(() => "---");
  const lines = [`| ${header.join(" | ")} |`, `| ${separator.join(" | ")} |`];
  for (const row of rows.slice(1)) lines.push(`| ${row.join(" | ")} |`);
  return lines.join("\n");
}

function block(el: Element): string {
  const tag = el.tagName.toLowerCase();
  if (/^h[1-6]$/.test(tag)) {
    return `${"#".repeat(Number(tag[1]))} ${inline(el).trim()}`;
  }
  switch (tag) {
    case "hr":
      return "---";
    case "pre": {
      const lang = el.getAttribute("data-lang") ?? "";
      return `\`\`\`${lang}\n${el.textContent ?? ""}\n\`\`\``;
    }
    case "blockquote":
      return inline(el)
        .split("\n")
        .map((line) => `> ${line}`)
        .join("\n");
    case "ul":
    case "ol":
      return listBlock(el);
    case "table":
      return tableBlock(el);
    case "p":
      return inline(el).replace(/^\n+|\n+$/g, "");
    default: {
      // Tables and code blocks are wrapped in a container by the renderer.
      const table = el.querySelector("table");
      if (table) return tableBlock(table);
      const pre = el.querySelector("pre");
      if (pre) return block(pre);
      return inline(el).trim();
    }
  }
}

/** Converts the rendered preview back to Markdown. */
export function htmlToMarkdown(root: Element): string {
  const blocks: string[] = [];
  for (const node of Array.from(root.childNodes)) {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = (node.textContent ?? "").trim();
      if (text) blocks.push(escapeText(text));
      continue;
    }
    if (node.nodeType !== Node.ELEMENT_NODE) continue;
    if ((node as Element).getAttribute("contenteditable") === "false") continue;
    const value = block(node as Element).replace(/\s+$/, "");
    if (value.trim()) blocks.push(value);
  }
  return blocks.join("\n\n");
}
