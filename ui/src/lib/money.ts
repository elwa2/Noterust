/**
 * Cash counting helpers.
 *
 * The counter works with Egyptian banknotes only (no coins): the change that
 * cannot be paid in notes is written in the "مبلغ فكة" field and added to the
 * total as it is. Every number is rendered with Latin digits.
 */

import { parseAmount } from "./format";

/** فئات الأوراق بالجنيه المصري. */
export const NOTE_VALS = [200, 100, 50, 20, 10, 5, 1];

/** عدد الورق في الباكو الواحد. */
export const NOTES_PER_PACK = 100;

/** عدد الباكوهات في الربطة الواحدة. */
export const PACKS_PER_BUNDLE = 5;

/** عدد الورق في الربطة الواحدة (5 باكوهات × 100 ورقة). */
export const NOTES_PER_BUNDLE = NOTES_PER_PACK * PACKS_PER_BUNDLE;

/** قيمة الرباط: الفئة × عدد الروابط × 100 × 5. */
export function bundleTotal(value: number, bundles: number): number {
  return value * bundles * NOTES_PER_PACK * PACKS_PER_BUNDLE;
}

/** قيمة الباكوهات: الفئة × عدد الباكوهات × 100. */
export function packTotal(value: number, packs: number): number {
  return value * packs * NOTES_PER_PACK;
}

/** قيمة الأوراق المفردة: الفئة × عدد الأوراق. */
export function notesTotal(value: number, notes: number): number {
  return value * notes;
}

export interface NoteRow {
  value: number;
  count: number;
  subtotal: number;
}

export interface Breakdown {
  rows: NoteRow[];
  /** The part of the amount that cannot be paid in notes. */
  left: number;
}

/** `1545` → `1,545.00` — Latin digits, always two decimals. */
export function formatMoney(value: number): string {
  return value.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

/** `1500` → `1,500` */
export function formatCount(value: number): string {
  return value.toLocaleString("en-US");
}

/** A typed note count: a whole, non-negative number (Arabic digits allowed). */
export function parseCount(input: string | null): number {
  const value = parseAmount(input);
  if (value === null || !Number.isFinite(value)) return 0;
  return Math.max(0, Math.floor(value));
}

/** A typed amount: never negative (Arabic digits and separators allowed). */
export function parseMoney(input: string | null): number {
  const value = parseAmount(input);
  if (value === null || !Number.isFinite(value)) return 0;
  return Math.max(0, value);
}

/** Greedy breakdown of an amount into banknotes. */
export function breakdownNotes(amount: number): Breakdown {
  let rest = Math.round(amount * 100);
  const rows: NoteRow[] = [];
  for (const value of NOTE_VALS) {
    const unit = value * 100;
    const count = Math.floor(rest / unit);
    if (count <= 0) continue;
    rest -= count * unit;
    rows.push({ value, count, subtotal: count * value });
  }
  const covered = rows.reduce((sum, row) => sum + row.subtotal, 0);
  return { rows, left: Math.round((amount - covered) * 100) / 100 };
}
