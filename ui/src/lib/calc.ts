/* A tiny, dependency-free arithmetic evaluator for the calculator.
   Supports + - * / and parentheses, Arabic-Indic digits, × ÷ and Arabic
   decimal separators. Never uses `eval`, so pasted text cannot run code. */

const ARABIC_DIGITS = "٠١٢٣٤٥٦٧٨٩";
const PERSIAN_DIGITS = "۰۱۲۳۴۵۶۷۸۹";

function normalize(input: string): string {
  let out = "";
  for (const char of input) {
    const arabic = ARABIC_DIGITS.indexOf(char);
    const persian = PERSIAN_DIGITS.indexOf(char);
    if (arabic !== -1) out += String(arabic);
    else if (persian !== -1) out += String(persian);
    else if (char === "×" || char === "x" || char === "X") out += "*";
    else if (char === "÷") out += "/";
    else if (char === "٫") out += ".";
    else if (char === "٬" || char === "," || char === "،") out += "";
    else if (char === " ") out += " ";
    else out += char;
  }
  return out.trim().replace(/=+$/, "").trim();
}

/** Returns the numeric value of an expression, or `null` when it is invalid. */
export function evaluate(input: string): number | null {
  const src = normalize(input);
  if (!src) return null;

  let index = 0;

  const skipSpaces = () => {
    while (src[index] === " ") index += 1;
  };

  const parseNumber = (): number | null => {
    skipSpaces();
    const start = index;
    while (index < src.length && /[0-9.]/.test(src[index])) index += 1;
    if (start === index) return null;
    const value = Number(src.slice(start, index));
    return Number.isFinite(value) ? value : null;
  };

  const parseFactor = (): number | null => {
    skipSpaces();
    if (src[index] === "(") {
      index += 1;
      const value = parseExpression();
      skipSpaces();
      if (src[index] !== ")") return null;
      index += 1;
      return value;
    }
    if (src[index] === "-") {
      index += 1;
      const value = parseFactor();
      return value === null ? null : -value;
    }
    if (src[index] === "+") {
      index += 1;
      return parseFactor();
    }
    return parseNumber();
  };

  const parseTerm = (): number | null => {
    let left = parseFactor();
    if (left === null) return null;
    for (;;) {
      skipSpaces();
      const op = src[index];
      if (op !== "*" && op !== "/") break;
      index += 1;
      const right = parseFactor();
      if (right === null) return null;
      if (op === "/") {
        if (right === 0) return null;
        left /= right;
      } else {
        left *= right;
      }
    }
    return left;
  };

  const parseExpression = (): number | null => {
    let left = parseTerm();
    if (left === null) return null;
    for (;;) {
      skipSpaces();
      const op = src[index];
      if (op !== "+" && op !== "-") break;
      index += 1;
      const right = parseTerm();
      if (right === null) return null;
      left = op === "+" ? left + right : left - right;
    }
    return left;
  };

  const value = parseExpression();
  skipSpaces();
  if (value === null || index !== src.length) return null;
  return Math.abs(value) < 1e15 ? value : null;
}

const resultFormatter = new Intl.NumberFormat("ar-EG", { maximumFractionDigits: 8 });

/** `٩٠` or `١٥٫٥` — rounded to 8 decimals with Arabic-Indic digits. */
export function formatResult(value: number): string {
  const rounded = Math.round(value * 1e8) / 1e8;
  return resultFormatter.format(rounded);
}
