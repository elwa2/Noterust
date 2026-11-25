<script lang="ts">
  import {
    NOTE_VALS,
    breakdownNotes,
    formatCount,
    formatMoney,
    parseCount,
    parseMoney,
  } from "../lib/money";

  /** Typed note counts, in `NOTE_VALS` order. */
  let counts: string[] = [];
  /** Change amount, written directly and added to the total as it is. */
  let change = "";
  /** The amount to break down into banknotes. */
  let amount = "";

  $: rows = NOTE_VALS.map((value, index) => {
    const count = parseCount(counts[index] ?? "");
    return { value, count, subtotal: count * value };
  });
  $: changeValue = parseMoney(change);
  $: countedTotal = rows.reduce((sum, row) => sum + row.subtotal, 0) + changeValue;

  $: target = parseMoney(amount);
  $: breakdown = target > 0 ? breakdownNotes(target) : null;
  $: brokenTotal = breakdown ? breakdown.rows.reduce((sum, row) => sum + row.subtotal, 0) : 0;

  /** Moves the counted total into the breakdown field. */
  function useCountedTotal() {
    amount = String(countedTotal);
  }
</script>

<div class="money">
  <section>
    <h3>عدّ الملاحظات</h3>
    <p class="faint">اكتب عدد الملاحظات لكل فئة — أو اكتب مبلغ الفكة مباشرة في سطر «مبلغ فكة».</p>
    <table class="money-table">
      <thead>
        <tr>
          <th>الفئة</th>
          <th>العدد</th>
          <th>المجموع</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row, index}
          <tr>
            <td class="value">{row.value}</td>
            <td>
              <input
                class="num"
                type="text"
                inputmode="numeric"
                placeholder="0"
                dir="ltr"
                aria-label="عدد الملاحظات فئة {row.value}"
                bind:value={counts[index]}
              />
            </td>
            <td class="value" dir="ltr">{formatMoney(row.subtotal)}</td>
          </tr>
        {/each}
        <tr class="change">
          <td class="value">مبلغ فكة</td>
          <td>
            <input
              class="num"
              type="text"
              inputmode="decimal"
              placeholder="0"
              dir="ltr"
              aria-label="مبلغ الفكة"
              title="اكتب مبلغ الفكة مباشرة — بيتجمع على الإجمالي زي ما هو"
              bind:value={change}
            />
          </td>
          <td class="value" dir="ltr">{formatMoney(changeValue)}</td>
        </tr>
      </tbody>
      <tfoot>
        <tr>
          <td>الإجمالي</td>
          <td></td>
          <td class="value total" dir="ltr">{formatMoney(countedTotal)}</td>
        </tr>
      </tfoot>
    </table>
  </section>

  <section>
    <h3>فك مبلغ إلى أوراق</h3>
    <div class="row">
      <input
        class="num amount"
        type="text"
        inputmode="decimal"
        placeholder="0"
        dir="ltr"
        aria-label="المبلغ المراد فكه"
        bind:value={amount}
      />
      <button class="btn" on:click={useCountedTotal} disabled={countedTotal <= 0}>
        استخدم الإجمالي
      </button>
    </div>

    {#if breakdown && breakdown.rows.length > 0}
      <table class="money-table">
        <thead>
          <tr>
            <th>الفئة</th>
            <th>العدد</th>
            <th>المجموع</th>
          </tr>
        </thead>
        <tbody>
          {#each breakdown.rows as row}
            <tr>
              <td class="value">{row.value}</td>
              <td class="value" dir="ltr">{formatCount(row.count)}</td>
              <td class="value" dir="ltr">{formatMoney(row.subtotal)}</td>
            </tr>
          {/each}
        </tbody>
        <tfoot>
          <tr>
            <td>الإجمالي</td>
            <td></td>
            <td class="value total" dir="ltr">{formatMoney(brokenTotal)}</td>
          </tr>
        </tfoot>
      </table>
      <p class="faint" class:bad={breakdown.left > 0}>
        {#if breakdown.left > 0}
          الجزء الصغير غير قابل للفك بالأوراق: {formatMoney(breakdown.left)}
        {:else}
          تم فك {formatMoney(target)} جنيه بالكامل.
        {/if}
      </p>
    {:else if target > 0}
      <p class="faint">المبلغ أقل من قيمة أصغر فئة.</p>
    {/if}
  </section>
</div>

<style>
  .money {
    display: flex;
    flex-direction: column;
    gap: 18px;
    min-width: min(480px, 82vw);
  }

  h3 {
    margin: 0 0 4px;
    font-size: 0.95em;
  }

  .faint {
    margin: 0 0 10px;
    color: var(--text-faint);
    font-size: 0.76em;
  }

  .faint.bad {
    color: var(--danger, #e5484d);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 10px;
  }

  .num {
    width: 120px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 7px 10px;
    font-size: 0.9em;
    text-align: right;
  }

  .num:focus {
    outline: none;
    border-color: var(--accent);
  }

  .num.amount {
    width: 150px;
    font-size: 1.05em;
    font-weight: 600;
  }

  .money-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.88em;
  }

  .money-table th {
    text-align: right;
    font-weight: 600;
    color: var(--text-muted);
    font-size: 0.92em;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
  }

  .money-table td {
    padding: 3px 6px;
    border-bottom: 1px solid var(--border);
  }

  .money-table .value {
    white-space: nowrap;
  }

  .money-table .change td {
    background: var(--bg-elevated);
  }

  .money-table .change .value:first-child {
    color: var(--accent);
    font-weight: 700;
  }

  .money-table tfoot td {
    border-bottom: none;
    padding-top: 8px;
    font-weight: 700;
  }

  .money-table .total {
    color: var(--accent);
    font-weight: 700;
  }
</style>
