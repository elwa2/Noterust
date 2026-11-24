<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import {
    amount,
    arabicDigits,
    fromInputDate,
    money,
    parseAmount,
    plural,
    NOUNS,
    toInputDate,
  } from "../lib/format";
  import { settings } from "../lib/store";
  import DonutChart from "./DonutChart.svelte";
  import BackButton from "./BackButton.svelte";
  import Icon from "./Icon.svelte";
  import type { Category, MonthlySummary, TransactionItem } from "../lib/types";

  let year = new Date().getFullYear();
  let month = new Date().getMonth() + 1;
  let summary: MonthlySummary | null = null;
  let transactions: TransactionItem[] = [];
  let categories: Category[] = [];
  let kindFilter: "" | "income" | "expense" = "";
  let loading = true;

  let kind: "income" | "expense" = "expense";
  let amountText = "";
  let categoryId: string = "";
  let note = "";
  let date = toInputDate(Date.now());
  let saving = false;
  let error = "";

  let categoryOpen = false;
  let newCategoryName = "";
  let newCategoryKind: "income" | "expense" = "expense";
  let newCategoryColor = "#e8b830";

  const monthNames = new Intl.DateTimeFormat("ar-EG", { month: "long", year: "numeric" });

  $: monthLabel = monthNames.format(new Date(year, month - 1, 1));
  $: monthCategories = categories.filter((category) => category.kind === kind);
  $: currency = summary?.currency ?? ($settings["default_currency"] ?? "EGP");
  $: categorySegments = (summary?.byCategory ?? [])
    .filter((row) => row.kind === "expense")
    .sort((a, b) => b.totalMinor - a.totalMinor)
    .map((row) => ({
      key: String(row.categoryId ?? row.name),
      label: row.name,
      value: row.totalMinor,
      color: row.color,
    }));
  $: maxDay = Math.max(
    1,
    ...(summary?.byDay ?? []).map((day) => Math.max(day.incomeMinor, day.expenseMinor)),
  );
  $: savingsRate =
    summary && summary.incomeMinor > 0
      ? Math.round((summary.balanceMinor / summary.incomeMinor) * 100)
      : null;
  $: topExpense =
    (summary?.byCategory ?? [])
      .filter((row) => row.kind === "expense")
      .sort((a, b) => b.totalMinor - a.totalMinor)[0] ?? null;
  $: activeDays = new Set((summary?.byDay ?? []).map((day) => day.day)).size;
  $: incomeCategories = (summary?.byCategory ?? []).filter((row) => row.kind === "income").length;
  $: expenseCategories = (summary?.byCategory ?? []).filter((row) => row.kind === "expense").length;

  onMount(() => void load());

  async function load() {
    loading = true;
    try {
      const from = new Date(year, month - 1, 1).getTime();
      const to = new Date(year, month, 1).getTime();
      [summary, transactions, categories] = await Promise.all([
        api.monthlySummary(year, month),
        api.listTransactions(from, to, kindFilter || undefined),
        api.listCategories(),
      ]);
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    } finally {
      loading = false;
    }
  }

  function shiftMonth(delta: number) {
    const date = new Date(year, month - 1 + delta, 1);
    year = date.getFullYear();
    month = date.getMonth() + 1;
    void load();
  }

  async function addTransaction() {
    const value = parseAmount(amountText);
    if (value === null || !Number.isFinite(value) || value <= 0) {
      error = "أدخل مبلغاً صحيحاً (يمكنك استخدام الأرقام العربية)";
      return;
    }
    saving = true;
    error = "";
    try {
      await api.addTransaction({
        kind,
        amountMinor: Math.round(value * 100),
        categoryId: categoryId ? Number(categoryId) : null,
        note: note.trim() || null,
        occurredAt: fromInputDate(date) ?? Date.now(),
      });
      amountText = "";
      note = "";
      await load();
      store.toast("تمت إضافة المعاملة");
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      saving = false;
    }
  }

  async function removeTransaction(item: TransactionItem) {
    if (!confirm("حذف هذه المعاملة؟")) return;
    try {
      await api.deleteTransaction(item.id);
      await load();
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  async function addCategory() {
    const name = newCategoryName.trim();
    if (!name) return;
    try {
      await api.createCategory(name, newCategoryKind, newCategoryColor);
      newCategoryName = "";
      categoryOpen = false;
      categories = await api.listCategories();
      store.toast("تمت إضافة الفئة");
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  async function exportCsv() {
    try {
      const path = await api.exportTransactionsCsv(year, month);
      if (path) store.toast(`تم التصدير إلى ${path}`);
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  function dayOf(millis: number): string {
    return arabicDigits(new Date(millis).getDate());
  }
</script>

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>الدخل والمصروفات</h1>
      <p class="muted">سجّل معاملاتك وتابع ملخص الشهر بالجنيه المصري.</p>
    </div>
    <div class="month-nav">
      <button class="icon-btn" title="الشهر السابق" on:click={() => shiftMonth(-1)}>›</button>
      <span class="month">{monthLabel}</span>
      <button class="icon-btn" title="الشهر التالي" on:click={() => shiftMonth(1)}>‹</button>
    </div>
  </header>

  <div class="scroll content">
    {#if loading && !summary}
      <p class="muted">جارٍ التحميل…</p>
    {:else if summary}
      <div class="cards">
        <div class="stat income">
          <span class="stat-label"><Icon name="importIcon" size={13} /> الدخل</span>
          <strong>{money(summary.incomeMinor, currency)}</strong>
          <span class="stat-hint">
            {plural(incomeCategories, {
              zero: "لا توجد فئات دخل",
              one: "فئة واحدة",
              two: "فئتان",
              few: "{n} فئات",
              many: "{n} فئة",
              other: "{n} فئة",
            })}
          </span>
        </div>
        <div class="stat expense">
          <span class="stat-label"><Icon name="exportIcon" size={13} /> المصروفات</span>
          <strong>{money(summary.expenseMinor, currency)}</strong>
          <span class="stat-hint">
            {plural(expenseCategories, {
              zero: "لا توجد فئات مصروفات",
              one: "فئة واحدة",
              two: "فئتان",
              few: "{n} فئات",
              many: "{n} فئة",
              other: "{n} فئة",
            })}
          </span>
        </div>
        <div class="stat" class:negative={summary.balanceMinor < 0}>
          <span class="stat-label"><Icon name="wallet" size={13} /> الصافي</span>
          <strong>{money(summary.balanceMinor, currency)}</strong>
          <span class="stat-hint">
            {summary.balanceMinor < 0 ? "الشهر منتهٍ بعجز" : "دخل ناقص مصروفات"}
          </span>
        </div>
        <div class="stat" class:negative={(savingsRate ?? 0) < 0}>
          <span class="stat-label"><Icon name="sparkle" size={13} /> معدل الادخار</span>
          <strong>{savingsRate === null ? "—" : `${arabicDigits(savingsRate)}٪`}</strong>
          <span class="stat-hint">
            {savingsRate === null ? "لا يوجد دخل هذا الشهر" : "من إجمالي الدخل"}
          </span>
        </div>
      </div>

      <div class="grid">
        <div class="card">
          <h3>الحركة اليومية</h3>
          {#if summary.byDay.length === 0}
            <p class="faint">لا معاملات هذا الشهر.</p>
          {:else}
            <div class="chart">
              {#each summary.byDay as day (day.day)}
                <div class="day-col" title={`يوم ${day.day}`}>
                  <div class="bars">
                    <div class="bar income" style="height:{(day.incomeMinor / maxDay) * 100}%"></div>
                    <div class="bar expense" style="height:{(day.expenseMinor / maxDay) * 100}%"></div>
                  </div>
                  <span class="day-label">{arabicDigits(day.day)}</span>
                </div>
              {/each}
            </div>
            <div class="legend">
              <span><i class="dot income"></i> دخل</span>
              <span><i class="dot expense"></i> مصروف</span>
            </div>
          {/if}
        </div>

        <div class="card">
          <div class="card-head">
            <h3>حسب الفئة</h3>
            {#if topExpense}
              <span class="top-cat" title="أكبر فئة مصروفات">
                الأعلى: {topExpense.name} · {money(topExpense.totalMinor, currency)}
              </span>
            {/if}
          </div>
          {#if summary.byCategory.length === 0}
            <p class="faint">لا بيانات بعد.</p>
          {:else}
            <div class="donut-row">
              <DonutChart
                segments={categorySegments}
                centerLabel="مصروفات"
                centerValue={amount(summary.expenseMinor)}
                size={148}
                thickness={20}
                emptyText="لا مصروفات"
              />
              <ul class="breakdown">
                {#each summary.byCategory as row (row.categoryId ?? row.name)}
                  <li>
                    <span class="dot" style="background:{row.color}"></span>
                    <span class="cat-name">{row.name}</span>
                    <span class="cat-kind">{row.kind === "income" ? "دخل" : "مصروف"}</span>
                    <span class="cat-total">{money(row.totalMinor, currency)}</span>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      </div>

      <div class="card add-card">
        <h3>معاملة جديدة</h3>
        <div class="add-row">
          <div class="kind-toggle">
            <button class:on={kind === "expense"} on:click={() => { kind = "expense"; categoryId = ""; }}>مصروف</button>
            <button class:on={kind === "income"} on:click={() => { kind = "income"; categoryId = ""; }}>دخل</button>
          </div>
          <input
            class="amount"
            placeholder="المبلغ"
            bind:value={amountText}
            inputmode="decimal"
            dir="auto"
          />
          <select bind:value={categoryId} title="الفئة">
            <option value="">بدون فئة</option>
            {#each monthCategories as category (category.id)}
              <option value={category.id}>{category.name}</option>
            {/each}
          </select>
          <input type="date" bind:value={date} title="التاريخ" />
          <input class="grow" placeholder="ملاحظة (اختياري)" bind:value={note} />
          <button class="btn primary" on:click={addTransaction} disabled={saving}>
            <Icon name="plus" size={14} /> إضافة
          </button>
        </div>
        {#if error}<p class="error">{error}</p>{/if}
        <button class="link" on:click={() => (categoryOpen = !categoryOpen)}>
          {categoryOpen ? "إخفاء" : "إضافة فئة جديدة"}
        </button>
        {#if categoryOpen}
          <div class="add-row category-row">
            <input placeholder="اسم الفئة" bind:value={newCategoryName} />
            <select bind:value={newCategoryKind}>
              <option value="expense">مصروف</option>
              <option value="income">دخل</option>
            </select>
            <input type="color" bind:value={newCategoryColor} title="اللون" />
            <button class="btn" on:click={addCategory}>إضافة الفئة</button>
          </div>
        {/if}
      </div>

      <div class="card">
                  <div class="list-head">
          <h3>المعاملات</h3>
          <span class="faint tx-count">
            {activeDays === 0 ? "لا أيام نشطة" : `${arabicDigits(activeDays)} يوم نشط`}
          </span>
          <select bind:value={kindFilter} on:change={() => load()}>
            <option value="">الكل</option>
            <option value="income">دخل</option>
            <option value="expense">مصروف</option>
          </select>
          <button class="btn ghost small" on:click={exportCsv}>
            <Icon name="exportIcon" size={14} /> CSV
          </button>
        </div>

        {#if transactions.length === 0}
          <p class="faint">لا معاملات في هذا الشهر.</p>
        {:else}
          <p class="faint tx-count">{plural(transactions.length, NOUNS.entry)}</p>
          <ul class="txns">
            {#each transactions as item (item.id)}
              <li>
                <span class="txn-day">{dayOf(item.occurredAt)}</span>
                <span class="dot" style="background:{item.categoryColor ?? 'var(--border-strong)'}"></span>
                <span class="txn-title">
                  {item.categoryName ?? "بدون فئة"}
                  {#if item.note}<span class="txn-note">— {item.note}</span>{/if}
                </span>
                <span class="txn-amount" class:income={item.kind === "income"}>
                  {item.kind === "income" ? "+" : "−"}{amount(item.amountMinor)}
                </span>
                <button class="icon-btn" title="حذف" on:click={() => removeTransaction(item)}>
                  <Icon name="trash" size={15} />
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/if}
  </div>
</section>

<style>
  .view-page {
    grid-column: 3 / -1;
    grid-row: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }

  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px 26px 14px;
    border-bottom: 1px solid var(--border);
  }

  h1 {
    margin: 0;
    font-size: 1.28em;
  }

  .page-head p {
    margin: 2px 0 0;
    font-size: 0.82em;
  }

  .month-nav {
    margin-inline-start: auto;
    display: flex;
    align-items: center;
    gap: 4px;
  }

  .month {
    min-width: 130px;
    text-align: center;
    font-size: 0.86em;
    font-weight: 600;
  }

  .content {
    flex: 1;
    min-height: 0;
    padding: 18px 26px 30px;
    overflow-y: auto;
  }

  .cards {
    display: flex;
    gap: 10px;
    flex-wrap: wrap;
    align-items: stretch;
    margin-bottom: 14px;
  }

  .stat {
    flex: 1;
    min-width: 150px;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .stat-label {
    font-size: 0.74em;
    color: var(--text-faint);
  }

  .stat strong {
    font-size: 1.15em;
  }

  .stat-label {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }

  .stat-hint {
    font-size: 0.7em;
    color: var(--text-faint);
  }

  .card-head {
    display: flex;
    align-items: baseline;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }

  .card-head h3 {
    margin: 0;
  }

  .top-cat {
    font-size: 0.74em;
    color: var(--text-faint);
    margin-inline-start: auto;
  }

  .btn.ghost.small {
    font-size: 0.78em;
    padding: 4px 10px;
  }

  .stat.income strong {
    color: var(--success);
  }

  .stat.expense strong {
    color: var(--danger);
  }

  .stat.negative strong {
    color: var(--danger);
  }

  .grid {
    display: grid;
    grid-template-columns: 1.4fr 1fr;
    gap: 12px;
    margin-bottom: 14px;
  }

  .card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    margin-bottom: 12px;
  }

  .grid .card {
    margin-bottom: 0;
  }

  h3 {
    margin: 0 0 12px;
    font-size: 0.92em;
  }

  .chart {
    display: flex;
    align-items: flex-end;
    gap: 4px;
    height: 150px;
    padding-top: 6px;
  }

  .day-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
    height: 100%;
    min-width: 0;
  }

  .bars {
    flex: 1;
    display: flex;
    align-items: flex-end;
    gap: 2px;
    width: 100%;
    height: 100%;
  }

  .bar {
    flex: 1;
    border-radius: 3px 3px 0 0;
    min-height: 1px;
  }

  .bar.income {
    background: var(--success);
  }

  .bar.expense {
    background: var(--danger);
  }

  .day-label {
    font-size: 0.62em;
    color: var(--text-faint);
  }

  .legend {
    display: flex;
    gap: 12px;
    margin-top: 8px;
    font-size: 0.72em;
    color: var(--text-muted);
  }

  .legend span {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    display: inline-block;
    flex-shrink: 0;
  }

  .dot.income {
    background: var(--success);
  }

  .dot.expense {
    background: var(--danger);
  }

  .donut-row {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
  }

  .donut-row .breakdown {
    flex: 1;
    min-width: 160px;
  }

  .breakdown {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 9px;
  }

  .tx-count {
    margin: 0 0 8px;
  }

  .breakdown li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.82em;
  }

  .cat-name {
    flex: 1;
  }

  .cat-kind {
    font-size: 0.85em;
    color: var(--text-faint);
  }

  .cat-total {
    font-weight: 600;
  }

  .add-row {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
  }

  .grow {
    flex: 1;
    min-width: 130px;
  }

  .amount {
    width: 110px;
  }

  .kind-toggle {
    display: flex;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .kind-toggle button {
    padding: 6px 13px;
    font-size: 0.82em;
    color: var(--text-muted);
  }

  .kind-toggle button.on {
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 600;
  }

  .link {
    margin-top: 10px;
    font-size: 0.78em;
    color: var(--accent);
  }

  .category-row {
    margin-top: 10px;
  }

  .list-head {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 8px;
  }

  .list-head h3 {
    margin: 0;
    flex: 1;
  }

  .txns {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .txns li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 4px;
    border-bottom: 1px solid var(--border);
    font-size: 0.85em;
  }

  .txns li:last-child {
    border-bottom: none;
  }

  .txn-day {
    width: 24px;
    color: var(--text-faint);
    font-size: 0.9em;
  }

  .txn-title {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .txn-note {
    color: var(--text-faint);
    font-size: 0.9em;
  }

  .txn-amount {
    font-weight: 600;
    color: var(--danger);
  }

  .txn-amount.income {
    color: var(--success);
  }

  .error {
    margin: 8px 0 0;
    color: var(--danger);
    font-size: 0.8em;
  }
</style>
