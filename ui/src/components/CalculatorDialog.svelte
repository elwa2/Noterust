<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";
  import CashCounter from "./CashCounter.svelte";
  import { evaluate, formatResult } from "../lib/calc";
  import * as store from "../lib/store";

  interface HistoryEntry {
    expr: string;
    value: number;
  }

  const dispatch = createEventDispatcher<{ close: void }>();
  const STORAGE_KEY = "noterust.calc.history";

  /** Which tool the dialog opens on. */
  export let initialTab: "calc" | "money" = "calc";

  let expression = "";
  let history: HistoryEntry[] = [];
  let inputEl: HTMLInputElement | null = null;
  /** Which tool the dialog shows: the arithmetic calculator or the cash counter. */
  let tab: "calc" | "money" = initialTab;

  $: result = evaluate(expression);
  $: valid = result !== null;

  onMount(() => {
    loadHistory();
    inputEl?.focus();
  });

  function loadHistory() {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      const parsed = raw ? (JSON.parse(raw) as HistoryEntry[]) : [];
      history = Array.isArray(parsed)
        ? parsed.filter((item) => typeof item?.expr === "string" && Number.isFinite(item?.value))
        : [];
    } catch {
      history = [];
    }
  }

  function persist() {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(history.slice(0, 40)));
  }

  function commit() {
    if (result === null) return;
    history = [{ expr: expression.trim(), value: result }, ...history].slice(0, 40);
    persist();
    expression = formatResult(result);
    inputEl?.focus();
    inputEl?.select();
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      commit();
    }
  }

  function reuse(entry: HistoryEntry) {
    expression = entry.expr;
    inputEl?.focus();
  }

  function clearHistory() {
    history = [];
    localStorage.removeItem(STORAGE_KEY);
  }

  async function copyResult() {
    if (result === null) return;
    try {
      await navigator.clipboard.writeText(formatResult(result));
      store.toast("تم نسخ النتيجة");
    } catch {
      store.toast("تعذر النسخ", "error");
    }
  }
</script>

<Modal title={tab === "calc" ? "الآلة الحاسبة" : "عدّ الفلوس"} on:close={() => dispatch("close")}>
  <div class="tabs">
    <button class="tab" class:active={tab === "calc"} on:click={() => (tab = "calc")}>
      <Icon name="calculator" size={14} /> حساب
    </button>
    <button class="tab" class:active={tab === "money"} on:click={() => (tab = "money")}>
      <Icon name="wallet" size={14} /> عدّ الفلوس
    </button>
  </div>

  {#if tab === "money"}
    <CashCounter />
  {:else}
  <div class="calc">
    <p class="faint">حساب سريع · سجل محفوظ</p>

    <input
      class="expr"
      bind:this={inputEl}
      bind:value={expression}
      on:keydown={onKeydown}
      placeholder="15+15+15+15+30"
      dir="ltr"
      spellcheck="false"
      inputmode="text"
      aria-label="التعبير الحسابي"
    />

    <div class="out">
      {#if expression.trim() === ""}
        <span class="result empty">—</span>
        <span class="hint">اكتب عملية حسابية بالأرقام العربية أو اللاتينية</span>
      {:else if valid}
        <span class="result" dir="ltr">{formatResult(result ?? 0)}</span>
        <span class="hint">النتيجة</span>
      {:else}
        <span class="result empty">؟</span>
        <span class="hint">تعبير غير صالح</span>
      {/if}
    </div>

    <div class="actions">
      <button class="btn primary" on:click={commit} disabled={!valid}>
        <Icon name="check" size={14} /> حساب
      </button>
      <button class="btn" on:click={copyResult} disabled={!valid}>
        <Icon name="copy" size={14} /> نسخ
      </button>
      <span class="spacer"></span>
      <button class="btn ghost" on:click={() => (expression = "")} disabled={!expression}>
        مسح
      </button>
    </div>

    <div class="history-head">
      <span>السجل</span>
      {#if history.length > 0}
        <button class="link" on:click={clearHistory}>مسح السجل</button>
      {/if}
    </div>

    {#if history.length === 0}
      <p class="faint">ستظهر عملياتك هنا بعد الحساب.</p>
    {:else}
      <ul class="history">
        {#each history as entry, index (entry.expr + entry.value + index)}
          <li>
            <button class="entry" on:click={() => reuse(entry)} title="استخدام هذا التعبير">
              <span class="entry-expr" dir="ltr">{entry.expr}</span>
              <span class="entry-value" dir="ltr">{formatResult(entry.value)}</span>
            </button>
          </li>
        {/each}
      </ul>
    {/if}

    <p class="faint keys">
      يدعم <bdi dir="ltr">+ − × ÷ ( )</bdi> والأرقام العربية، و<bdi>Enter</bdi> للحساب.
    </p>
  </div>
  {/if}
</Modal>

<style>
  .tabs {
    display: flex;
    gap: 6px;
    margin-bottom: 12px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 10px;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    border-radius: var(--radius-sm);
    background: transparent;
    border: 1px solid transparent;
    color: var(--text-muted);
    font-size: 0.86em;
  }

  .tab:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .tab.active {
    background: var(--bg-elevated);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .calc {
    display: flex;
    flex-direction: column;
    gap: 9px;
    min-width: min(520px, 82vw);
  }

  .faint {
    margin: 0;
    color: var(--text-faint);
    font-size: 0.76em;
  }

  .expr {
    background: var(--bg);
    border: 1.5px solid var(--accent);
    border-radius: var(--radius-sm);
    padding: 12px 14px;
    font-size: 1.45em;
    font-weight: 600;
    text-align: right;
    letter-spacing: 0.5px;
  }

  .out {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 2px;
    min-height: 62px;
    justify-content: center;
  }

  .result {
    font-size: 2.2em;
    font-weight: 700;
    line-height: 1.1;
  }

  .result.empty {
    color: var(--text-faint);
  }

  .hint {
    color: var(--text-faint);
    font-size: 0.74em;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spacer {
    flex: 1;
  }

  .history-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-top: 1px solid var(--border);
    padding-top: 10px;
    font-size: 0.82em;
    color: var(--text-muted);
  }

  .link {
    color: var(--accent);
    font-size: 0.86em;
  }

  .link:hover {
    text-decoration: underline;
  }

  .history {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 210px;
    overflow-y: auto;
  }

  .entry {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    background: var(--bg-elevated);
    font-size: 0.85em;
  }

  .entry:hover {
    background: var(--bg-hover);
  }

  .entry-expr {
    flex: 1;
    text-align: right;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .entry-value {
    font-weight: 600;
  }

  .keys {
    line-height: 1.9;
  }
</style>
