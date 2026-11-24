/* Arabic-Indic formatting helpers. `ar-EG` gives ٠١٢٣ digits natively. */

const dateTime = new Intl.DateTimeFormat("ar-EG", {
  day: "numeric",
  month: "long",
  year: "numeric",
});

const timeOnly = new Intl.DateTimeFormat("ar-EG", {
  hour: "2-digit",
  minute: "2-digit",
  hour12: false,
});

const shortDate = new Intl.DateTimeFormat("ar-EG", {
  day: "numeric",
  month: "short",
});

/** `٢٣ نوفمبر ٢٠٢٦ في ٢:١١ م` */
export function arabicDateTime(millis: number): string {
  if (!millis) return "";
  const date = new Date(millis);
  const time = timeOnly.format(date).replace(/\s/g, " ");
  return `${dateTime.format(date)} في ${time}`;
}

export function shortDay(millis: number): string {
  if (!millis) return "";
  const date = new Date(millis);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) return timeOnly.format(date);
  return shortDate.format(date);
}

/** Bucket label used by the note list grouping. */
export function dateGroup(millis: number): string {
  const date = new Date(millis);
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const startOfYesterday = startOfToday - 86_400_000;
  if (millis >= startOfToday) return "اليوم";
  if (millis >= startOfYesterday) return "أمس";
  if (millis >= startOfToday - 7 * 86_400_000) return "آخر ٧ أيام";
  return "أقدم";
}

/** Converts ASCII digits to Arabic-Indic digits. */
export function arabicDigits(value: string | number): string {
  return String(value).replace(/[0-9]/g, (digit) => "٠١٢٣٤٥٦٧٨٩"[Number(digit)]);
}

const moneyFormatters = new Map<string, Intl.NumberFormat>();

/** `٥٠٠٫٠٠ ج.م.‏` from 50000 piastres. */
export function money(amountMinor: number, currency = "EGP"): string {
  const key = currency;
  let formatter = moneyFormatters.get(key);
  if (!formatter) {
    formatter = new Intl.NumberFormat("ar-EG", {
      style: "currency",
      currency,
      maximumFractionDigits: 2,
    });
    moneyFormatters.set(key, formatter);
  }
  return formatter.format(amountMinor / 100);
}

/** Plain amount without the currency symbol. */
export function amount(amountMinor: number): string {
  return new Intl.NumberFormat("ar-EG", { maximumFractionDigits: 2 }).format(amountMinor / 100);
}

export function fileSize(bytes: number): string {
  if (bytes < 1024) return `${arabicDigits(bytes)} بايت`;
  if (bytes < 1024 * 1024) return `${arabicDigits((bytes / 1024).toFixed(1))} ك.ب`;
  return `${arabicDigits((bytes / 1024 / 1024).toFixed(1))} م.ب`;
}

/** `yyyy-mm-dd` for `<input type="date">`. */
export function toInputDate(millis: number | null): string {
  if (!millis) return "";
  const date = new Date(millis);
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

/** Parses a `<input type="date">` value into local midnight millis. */
export function fromInputDate(value: string): number | null {
  if (!value) return null;
  const [year, month, day] = value.split("-").map(Number);
  if (!year || !month || !day) return null;
  return new Date(year, month - 1, day, 12, 0, 0).getTime();
}

export function todayStamp(): string {
  return arabicDateTime(Date.now());
}

const intFormatter = new Intl.NumberFormat("ar-EG", { maximumFractionDigits: 0 });

/** `١٬٢٥٠` — grouped integer with Arabic-Indic digits. */
export function fmtInt(value: number): string {
  return intFormatter.format(value);
}

const weekdayFormatter = new Intl.DateTimeFormat("ar-EG", { weekday: "long" });

/** `اليوم` / `أمس` / weekday within a week / short date. */
export function relativeDay(millis: number): string {
  if (!millis) return "";
  const date = new Date(millis);
  const now = new Date();
  const startOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const target = new Date(date.getFullYear(), date.getMonth(), date.getDate()).getTime();
  const diff = Math.round((target - startOfToday) / 86_400_000);
  if (diff === 0) return "اليوم";
  if (diff === -1) return "أمس";
  if (diff === 1) return "غدًا";
  if (diff > 1 && diff < 7) return weekdayFormatter.format(date);
  if (diff < -1 && diff > -7) return weekdayFormatter.format(date);
  return shortDate.format(date);
}

/** Compact timestamp for list rows: time today, «أمس», weekday, then date. */
export function listTime(millis: number): string {
  if (!millis) return "";
  const date = new Date(millis);
  const now = new Date();
  const sameDay =
    date.getFullYear() === now.getFullYear() &&
    date.getMonth() === now.getMonth() &&
    date.getDate() === now.getDate();
  if (sameDay) return timeOnly.format(date);
  return relativeDay(millis);
}

export interface PluralForms {
  zero: string;
  one: string;
  two: string;
  few: string;
  many: string;
  other: string;
}

const pluralRules = new Intl.PluralRules("ar");

/** Arabic plural agreement: `plural(3, NOUNS.note) → «٣ ملاحظات»`. */
export function plural(
  count: number,
  forms: PluralForms,
  format: (value: number) => string = fmtInt,
): string {
  const category = pluralRules.select(count);
  const template = forms[category as keyof PluralForms] ?? forms.other;
  return template.replace("{n}", format(count));
}

export const NOUNS = {
  note: {
    zero: "لا توجد ملاحظات",
    one: "ملاحظة واحدة",
    two: "ملاحظتان",
    few: "{n} ملاحظات",
    many: "{n} ملاحظة",
    other: "{n} ملاحظة",
  },
  task: {
    zero: "لا توجد مهام",
    one: "مهمة واحدة",
    two: "مهمتان",
    few: "{n} مهام",
    many: "{n} مهمة",
    other: "{n} مهمة",
  },
  entry: {
    zero: "لا توجد معاملات",
    one: "معاملة واحدة",
    two: "معاملتان",
    few: "{n} معاملات",
    many: "{n} معاملة",
    other: "{n} معاملة",
  },
  snippet: {
    zero: "لا يوجد محفوظ",
    one: "عنصر واحد",
    two: "عنصران",
    few: "{n} عناصر",
    many: "{n} عنصراً",
    other: "{n} عنصر",
  },
  day: {
    zero: "{n} يوم",
    one: "يوم واحد",
    two: "يومان",
    few: "{n} أيام",
    many: "{n} يومًا",
    other: "{n} يوم",
  },
  file: {
    zero: "لا ملفات",
    one: "ملف واحد",
    two: "ملفان",
    few: "{n} ملفات",
    many: "{n} ملفًا",
    other: "{n} ملف",
  },
};

/** Currencies offered in settings and the finance view. */
export const CURRENCIES: { code: string; symbol: string; name: string }[] = [
  { code: "EGP", symbol: "ج.م", name: "جنيه مصري" },
  { code: "SAR", symbol: "ر.س", name: "ريال سعودي" },
  { code: "AED", symbol: "د.إ", name: "درهم إماراتي" },
  { code: "KWD", symbol: "د.ك", name: "دينار كويتي" },
  { code: "QAR", symbol: "ر.ق", name: "ريال قطري" },
  { code: "BHD", symbol: "د.ب", name: "دينار بحريني" },
  { code: "OMR", symbol: "ر.ع", name: "ريال عماني" },
  { code: "JOD", symbol: "د.أ", name: "دينار أردني" },
  { code: "MAD", symbol: "د.م", name: "درهم مغربي" },
  { code: "USD", symbol: "$", name: "دولار أمريكي" },
  { code: "EUR", symbol: "€", name: "يورو" },
  { code: "GBP", symbol: "£", name: "جنيه إسترليني" },
];

/** Converts Arabic-Indic and Persian digits to ASCII. */
export function toLatinDigits(value: string): string {
  return value
    .replace(/[٠-٩]/g, (digit) => String(digit.charCodeAt(0) - 0x0660))
    .replace(/[۰-۹]/g, (digit) => String(digit.charCodeAt(0) - 0x06f0));
}

/**
 * Parses a typed amount: accepts Arabic digits, `٫` decimal and `٬` grouping,
 * comma/dot separators and stray spaces. Returns a plain number or null.
 */
export function parseAmount(input: string | null): number | null {
  if (input == null) return null;
  let text = toLatinDigits(String(input))
    .replace(/[\s\u00a0\u200e\u200f\u061c]/g, "")
    .replace(/٫/g, ".")
    .replace(/٬/g, ",")
    .replace(/[^\d.,-]/g, "");
  if (!/\d/.test(text)) return null;
  const negative = text.startsWith("-");
  text = text.replace(/-/g, "");
  const lastDot = text.lastIndexOf(".");
  const lastComma = text.lastIndexOf(",");
  if (lastDot > -1 && lastComma > -1) {
    const decimal = lastDot > lastComma ? "." : ",";
    const grouping = decimal === "." ? "," : ".";
    text = text.split(grouping).join("");
    if (decimal === ",") text = text.replace(",", ".");
  } else if (lastComma > -1) {
    const parts = text.split(",");
    text = parts.length === 2 && parts[1].length >= 1 && parts[1].length <= 2
      ? parts.join(".")
      : parts.join("");
  } else if (text.split(".").length > 2) {
    text = text.split(".").join("");
  }
  const value = Number(text);
  if (!Number.isFinite(value)) return null;
  const rounded = Math.round(value * 1000) / 1000;
  return negative ? -rounded : rounded;
}
