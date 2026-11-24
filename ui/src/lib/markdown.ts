/* A small, dependency-free Markdown renderer for the note preview.
   Everything is escaped before any tag is emitted, so note text can never
   inject HTML (the preview is rendered with `{@html}`). */

function escapeHtml(text: string): string {
  return text.replace(/[&<>"']/g, (char) => {
    switch (char) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}

function inline(text: string): string {
  const codes: string[] = [];
  let value = text.replace(/`([^`]+)`/g, (_match, code: string) => {
    codes.push(escapeHtml(code));
    return `\u0000${codes.length - 1}\u0000`;
  });

  value = escapeHtml(value);
  value = value.replace(
    /!\[([^\]]*)\]\(([^)\s]+)(?:\s+&quot;[^&]*&quot;)?\)/g,
    '<img alt="$1" src="$2" loading="lazy" />',
  );
  value = value.replace(
    /\[([^\]]+)\]\(([^)\s]+)(?:\s+&quot;[^&]*&quot;)?\)/g,
    '<a href="$2" target="_blank" rel="noreferrer">$1</a>',
  );
  value = value.replace(/\*\*([^*]+)\*\*/g, "<strong>$1</strong>");
  value = value.replace(/__([^_]+)__/g, "<strong>$1</strong>");
  value = value.replace(/(^|[^*])\*([^*\n]+)\*/g, "$1<em>$2</em>");
  value = value.replace(/(^|[^_])_([^_\n]+)_/g, "$1<em>$2</em>");
  value = value.replace(/~~([^~]+)~~/g, "<del>$1</del>");
  value = value.replace(
    /\u0000(\d+)\u0000/g,
    (_match, index: string) => `<code>${codes[Number(index)] ?? ""}</code>`,
  );
  return value;
}

const isTableStart = (line: string, next: string | undefined): boolean =>
  line.includes("|") && !!next && /^\s*\|?[\s:-]*-[\s|:-]*\|?\s*$/.test(next) && next.includes("-");

const isBlockStart = (line: string, next?: string): boolean =>
  /^\s*$/.test(line) ||
  /^```/.test(line) ||
  /^#{1,6}\s/.test(line) ||
  /^\s*([-*+]|\d+[.)])\s+/.test(line) ||
  /^\s*>/.test(line) ||
  /^\s*(---+|\*\*\*+|___+)\s*$/.test(line) ||
  isTableStart(line, next);

function splitRow(line: string): string[] {
  return line
    .trim()
    .replace(/^\|/, "")
    .replace(/\|$/, "")
    .split("|")
    .map((cell) => cell.trim());
}

/** Renders Markdown to safe HTML. */
export function renderMarkdown(source: string): string {
  const lines = source.replace(/\r\n?/g, "\n").split("\n");
  const out: string[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i];

    if (/^\s*$/.test(line)) {
      i += 1;
      continue;
    }

    const fence = /^```(\S*)\s*$/.exec(line);
    if (fence) {
      const body: string[] = [];
      i += 1;
      while (i < lines.length && !/^```\s*$/.test(lines[i])) {
        body.push(lines[i]);
        i += 1;
      }
      i += 1;
      const lang = fence[1] ? ` data-lang="${escapeHtml(fence[1])}"` : "";
      out.push(`<pre class="md-code"${lang} dir="ltr"><code>${escapeHtml(body.join("\n"))}</code></pre>`);
      continue;
    }

    const heading = /^(#{1,6})\s+(.*)$/.exec(line);
    if (heading) {
      const level = heading[1].length;
      out.push(`<h${level}>${inline(heading[2].trim())}</h${level}>`);
      i += 1;
      continue;
    }

    if (/^\s*(---+|\*\*\*+|___+)\s*$/.test(line)) {
      out.push("<hr />");
      i += 1;
      continue;
    }

    if (isTableStart(line, lines[i + 1])) {
      const header = splitRow(line);
      const aligns = splitRow(lines[i + 1]).map((cell) => {
        const left = cell.startsWith(":");
        const right = cell.endsWith(":");
        if (left && right) return "center";
        if (right) return "right";
        return "start";
      });
      i += 2;
      const rows: string[][] = [];
      while (i < lines.length && lines[i].includes("|") && !/^\s*$/.test(lines[i])) {
        rows.push(splitRow(lines[i]));
        i += 1;
      }
      const head = header
        .map((cell, index) => `<th style="text-align:${aligns[index] ?? "start"}">${inline(cell)}</th>`)
        .join("");
      const body = rows
        .map(
          (row) =>
            `<tr>${row
              .map(
                (cell, index) =>
                  `<td style="text-align:${aligns[index] ?? "start"}">${inline(cell)}</td>`,
              )
              .join("")}</tr>`,
        )
        .join("");
      out.push(`<div class="md-table-wrap"><table><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table></div>`);
      continue;
    }

    if (/^\s*>/.test(line)) {
      const quoted: string[] = [];
      while (i < lines.length && /^\s*>/.test(lines[i])) {
        quoted.push(lines[i].replace(/^\s*>\s?/, ""));
        i += 1;
      }
      out.push(`<blockquote>${inline(quoted.join("<br />").replace(/<br \/>/g, "\n"))}</blockquote>`);
      continue;
    }

    const listMatch = /^\s*([-*+]|\d+[.)])\s+/.exec(line);
    if (listMatch) {
      const ordered = /\d/.test(listMatch[1]);
      const items: string[] = [];
      while (i < lines.length) {
        const item = /^(\s*)([-*+]|\d+[.)])\s+(.*)$/.exec(lines[i]);
        if (!item) break;
        if (/\d/.test(item[2]) !== ordered) break;
        const nested = item[1].length >= 2;
        const task = /^\[( |x|X)\]\s+(.*)$/.exec(item[3]);
        if (task) {
          const checked = task[1].toLowerCase() === "x";
          items.push(
            `<li class="md-task${nested ? " md-nested" : ""}"><span class="md-check${checked ? " on" : ""}">${checked ? "✓" : ""}</span><span class="md-task-text${checked ? " done" : ""}">${inline(task[2])}</span></li>`,
          );
        } else {
          items.push(`<li${nested ? ' class="md-nested"' : ""}>${inline(item[3])}</li>`);
        }
        i += 1;
      }
      const tag = ordered ? "ol" : "ul";
      out.push(`<${tag}>${items.join("")}</${tag}>`);
      continue;
    }

    const paragraph: string[] = [];
    while (i < lines.length && !isBlockStart(lines[i], lines[i + 1])) {
      paragraph.push(lines[i]);
      i += 1;
    }
    out.push(`<p>${inline(paragraph.join("\n")).replace(/\n/g, "<br />")}</p>`);
  }

  return out.join("\n");
}

/** First heading or first line, used for previews. */
export function markdownSummary(source: string, max = 120): string {
  const line = source
    .replace(/\r/g, "")
    .split("\n")
    .map((item) => item.replace(/^#+\s*/, "").trim())
    .find((item) => item.length > 0);
  return (line ?? "").slice(0, max);
}
