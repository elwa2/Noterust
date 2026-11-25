<script lang="ts">
  import {
    NOTE_VALS,
    breakdownNotes,
    bundleTotal,
    formatCount,
    formatMoney,
    notesTotal,
    packTotal,
    parseCount,
    parseMoney,
  } from "../lib/money";

  /** Typed bundle counts, in `NOTE_VALS` order — one رابطة = 5 باكوهات. */
  let bundleCounts: string[] = [];
  /** Typed pack counts, in `NOTE_VALS` order — one باكو = 100 ورقة. */
  let packCounts: string[] = [];
  /** Typed loose note counts, in `NOTE_VALS` order. */
  let noteCounts: string[] = [];
  /** Change amount, written directly and added to the total as it is. */
  let change = "";
  /** The amount to break down into banknotes. */
  let amount = "";

  $: rows = NOTE_VALS.map((value, index) => {
    const bundles = parseCount(bundleCounts[index] ?? "");
    const packs = parseCount(packCounts[index] ?? "");
    const notes = parseCount(noteCounts[index] ?? "");
    const bundleValue = bundleTotal(value, bundles);
    const packValue = packTotal(value, packs);
    const noteValue = notesTotal(value, notes);
    return {
      value,
      bundles,
      packs,
      notes,
      bundleValue,
      packValue,
      noteValue,
      subtotal: bundleValue + packValue + noteValue,
    };
  });
  $: changeValue = parseMoney(change);
  $: countedTotal = rows.reduce((sum, row) => sum + row.subtotal, 0) + changeValue;
  $: bundlesTotal = rows.reduce((sum, row) => sum + row.bundleValue, 0);
  $: packsTotal = rows.reduce((sum, row) => sum + row.packValue, 0);
  $: notesTotalSum = rows.reduce((sum, row) => sum + row.noteValue, 0);

  $: target = parseMoney(amount);
  $: breakdown = target > 0 ? breakdownNotes(target) : null;
  $: brokenTotal = breakdown ? breakdown.rows.reduce((sum, row) => sum + row.subtotal, 0) : 0;

  /**
   * Enter moves to the cell right below the current one, exactly like a
   * spreadsheet. The row under the last note is the «مبلغ فكة» field, and the
   * one under that is the amount to break down.
   */
  function onEnter(event: KeyboardEvent, column: "bundles" | "packs" | "notes", index: number) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    const next = document.getElementById(`cc-${column}-${index + 1}`);
    if (next instanceof HTMLInputElement) {
      next.focus();
      next.select();
      return;
    }
    const fallback =
      column === "notes" ? document.getElementById("cc-change") : document.getElementById("cc-amount");
    if (fallback instanceof HTMLInputElement) {
      fallback.focus();
      fallback.select();
    }
  }

  /** Moves the counted total into the breakdown field. */
  function useCountedTotal() {
    amount = String(countedTotal);
  }
</script>

<div class="money">
  <section>
    <h3>عدّ الملاحظات</h3>
    <p class="faint">
      اكتب عدد الروابط والباكوهات والأوراق لكل فئة — 1 رابطة = 5 باكوهات = 500 ورقة، و1 باكو = 100 ورقة.
      اضغط Enter في أي مربع عشان تنزل للمربع اللي تحته.
    </p>
    <table class="money-table">
      <thead>
        <tr>
          <th>الفئة</th>
          <th>روابط</th>
          <th>باكوهات</th>
          <th>أوراق</th>
          <th title="الفئة × عدد الروابط × 100 × 5">قيمة الروابط</th>
          <th title="الفئة × عدد الباكوهات × 100">قيمة الباكوهات</th>
          <th>المجموع</th>
        </tr>
      </thead>
      <tbody>
        {#each rows as row, index}
          <tr>
            <td class="value">{row.value}</td>
            <td>
              <input
                id="cc-bundles-{index}"
                class="num"
                type="text"
                inputmode="numeric"
                placeholder="0"
                dir="ltr"
                aria-label="عدد الروابط فئة {row.value}"
                bind:value={bundleCounts[index]}
                on:keydown={(event) => onEnter(event, "bundles", index)}
              />
            </td>
            <td>
              <input
                id="cc-packs-{index}"
                class="num"
                type="text"
                inputmode="numeric"
                placeholder="0"
                dir="ltr"
                aria-label="عدد الباكوهات فئة {row.value}"
                bind:value={packCounts[index]}
                on:keydown={(event) => onEnter(event, "packs", index)}
              />
            </td>
            <td>
              <input
                id="cc-notes-{index}"
                class="num"
                type="text"
                inputmode="numeric"
                placeholder="0"
                dir="ltr"
                aria-label="عدد الأوراق فئة {row.value}"
                bind:value={noteCounts[index]}
                on:keydown={(event) => onEnter(event, "notes", index)}
              />
            </td>
            <td class="value" dir="ltr">{formatMoney(row.bundleValue)}</td>
            <td class="value" dir="ltr">{formatMoney(row.packValue)}</td>
            <td class="value" dir="ltr">{formatMoney(row.subtotal)}</td>
          </tr>
        {/each}
        <tr class="change">
          <td colspan="3" class="value">مبلغ فكة</td>
          <td>
            <input
              id="cc-change"
              class="num"
              type="text"
              inputmode="decimal"
              placeholder="0"
              dir="ltr"
              aria-label="مبلغ الفكة"
              title="اكتب مبلغ الفكة مباشرة — بيتجمع على الإجمالي زي ما هو"
              bind:value={change}
              on:keydown={(event) => {
                if (event.key !== "Enter") return;
                event.preventDefault();
                const next = document.getElementById("cc-amount");
                if (next instanceof HTMLInputElement) {
                  next.focus();
                  next.select();
                }
              }}
            />
          </td>
          <td></td>
          <td></td>
          <td class="value" dir="ltr">{formatMoney(changeValue)}</td>
        </tr>
      </tbody>
      <tfoot>
        <tr>
          <td colspan="4">الإجمالي</td>
          <td class="value" dir="ltr">{formatMoney(bundlesTotal)}</td>
          <td class="value" dir="ltr">{formatMoney(packsTotal)}</td>
          <td class="value total" dir="ltr">{formatMoney(countedTotal)}</td>
        </tr>
      </tfoot>
    </table>
    <p class="faint">
      مجموع الأوراق المفردة: {formatMoney(notesTotalSum)} — والمجموع النهائي {formatMoney(countedTotal)}.
    </p>
  </section>

  <section>
    <h3>فك مبلغ إلى أوراق</h3>
    <div class="row">
      <input
        id="cc-amount"
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
    min-width: min(760px, 88vw);
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
    width: 72px;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 6px 8px;
    font-size: 0.88em;
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
    font-size: 0.85em;
  }

  .money-table th {
    text-align: right;
    font-weight: 600;
    color: var(--text-muted);
    font-size: 0.9em;
    padding: 4px 6px;
    border-bottom: 1px solid var(--border);
    white-space: nowrap;
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
