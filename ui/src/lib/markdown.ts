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

/* Images pasted into notes are referenced as `![alt](noteimg://file.png)` and
   stored outside the note text. Their files are loaded on demand and mapped
   from file name to `data:` URL in `imageAssets` before rendering. */
let imageAssets: Record<string, string> = {};
let imageIndex = 0;

/** `{w=320 r=90}` after an image adds a width in pixels and a rotation. */
export interface ImageOptions {
  width: number;
  rotation: number;
}

const IMAGE_RE = /!\[([^\]]*)\]\(([^)\s]+)(?:\s+&quot;[^&]*&quot;)?\)(?:\{([^}]*)\})?/g;
const ATTACHMENT_PREFIX = "noteimg://";

function parseImageOptions(raw: string | undefined): ImageOptions {
  const options: ImageOptions = { width: 0, rotation: 0 };
  if (!raw) return options;
  for (const part of raw.split(/[\s,;]+/)) {
    const [key, value] = part.split("=");
    const number = Math.round(Number(value));
    if (!Number.isFinite(number)) continue;
    if (key === "w" || key === "width") options.width = Math.max(40, Math.min(4000, number));
    if (key === "r" || key === "rotate") options.rotation = ((number % 360) + 360) % 360;
  }
  return options;
}

function imageMarkup(alt: string, src: string, source: string, options: ImageOptions): string {
  const { width, rotation } = options;
  const style = [
    width > 0 ? `width:${width}px` : "",
    rotation !== 0 ? `transform:rotate(${rotation}deg)` : "",
  ]
    .filter(Boolean)
    .join(";");
  const index = imageIndex;
  imageIndex += 1;
  return (
    `<span class="md-image" data-img="${index}" data-w="${width}" data-r="${rotation}" data-src="${source}">` +
    `<span class="md-image-frame"><img alt="${alt}" src="${src}" loading="lazy"${
      style ? ` style="${style}"` : ""
    } /></span>` +
    `<span class="md-image-tools" dir="ltr">` +
    `<button type="button" data-act="zoom-out" title="تصغير">−</button>` +
    `<span class="md-image-zoom">${width > 0 ? `${width}px` : "100%"}</span>` +
    `<button type="button" data-act="zoom-in" title="تكبير">+</button>` +
    `<button type="button" data-act="rotate-left" title="تدوير لليسار">↺</button>` +
    `<button type="button" data-act="rotate-right" title="تدوير لليمين">↻</button>` +
    `<button type="button" data-act="reset" title="إعادة الحجم والاتجاه">إعادة</button>` +
    `</span>` +
    `</span>`
  );
}

function inline(text: string): string {
  const codes: string[] = [];
  let value = text.replace(/`([^`]+)`/g, (_match, code: string) => {
    codes.push(escapeHtml(code));
    return `\u0000${codes.length - 1}\u0000`;
  });

  value = escapeHtml(value);
  value = value.replace(IMAGE_RE, (_match, alt: string, src: string, raw?: string) => {
    const name = src.startsWith(ATTACHMENT_PREFIX) ? src.slice(ATTACHMENT_PREFIX.length) : "";
    const resolved = name ? (imageAssets[name] ?? "") : src;
    return imageMarkup(alt, resolved, src, parseImageOptions(raw));
  });
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

/** Rewrites the `{w=… r=…}` suffix of the nth image in the note text. */
export function setImageOptions(source: string, index: number, options: ImageOptions): string {
  const parts: string[] = [];
  if (options.width > 0) parts.push(`w=${Math.round(options.width)}`);
  const rotation = ((Math.round(options.rotation) % 360) + 360) % 360;
  if (rotation !== 0) parts.push(`r=${rotation}`);
  const suffix = parts.length > 0 ? `{${parts.join(" ")}}` : "";
  let seen = -1;
  return source.replace(
    IMAGE_RE,
    (match, alt: string, src: string) => {
      seen += 1;
      if (seen !== index) return match;
      const quote = /\s+&quot;[^&]*&quot;/.exec(match)?.[0] ?? "";
      return `![${alt}](${src}${quote})${suffix}`;
    },
  );
}

/** Renders Markdown to safe HTML. */
export function renderMarkdown(
  source: string,
  images: Record<string, string> = {},
): string {
  imageAssets = images;
  imageIndex = 0;
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
      out.push(
        `<div class="md-code-block"><button type="button" class="md-copy" data-act="copy-code" title="نسخ الكود كاملاً" aria-label="نسخ الكود كاملاً">نسخ</button>` +
          `<pre class="md-code"${lang} dir="ltr"><code>${escapeHtml(body.join("\n"))}</code></pre></div>`,
      );
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
      out.push(`<blockquote>${inline(quoted.join("\n")).replace(/\n/g, "<br />")}</blockquote>`);
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
