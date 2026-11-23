<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { todayStamp } from "../lib/format";

  export let query = "";
  export let top = 0;
  export let left = 0;

  const dispatch = createEventDispatcher<{ choose: { template: string; caret: number } }>();

  interface Item {
    label: string;
    hint: string;
    template: string;
    caret: number;
  }

  const items: Item[] = [
    { label: "عنوان", hint: "عنوان كبير", template: "# عنوان\n", caret: 8 },
    { label: "قائمة نقطية", hint: "• عنصر", template: "- عنصر\n", caret: 7 },
    { label: "قائمة مرقمة", hint: "1. عنصر", template: "1. عنصر\n", caret: 8 },
    { label: "قائمة تحقق", hint: "☐ مهمة", template: "- [ ] مهمة\n", caret: 10 },
    { label: "كود", hint: "كتلة كود", template: "```\n\n```\n", caret: 4 },
    { label: "اقتباس", hint: "> نص", template: "> اقتباس\n", caret: 8 },
    { label: "فاصل", hint: "خط أفقي", template: "---\n", caret: 4 },
    {
      label: "جدول",
      hint: "جدول من عمودين",
      template: "| العمود الأول | العمود الثاني |\n| --- | --- |\n|  |  |\n",
      caret: 2,
    },
    { label: "تاريخ اليوم", hint: "إدراج التاريخ والوقت", template: `[[${todayStamp()}]]\n`, caret: 0 },
  ];

  let selected = 0;

  $: filtered = items.filter((item) => item.label.includes(query.trim()));
  $: visible = filtered.length > 0 ? filtered : items;
  $: if (selected >= visible.length) selected = 0;

  function choose(index: number) {
    const item = visible[index];
    if (!item) return;
    dispatch("choose", { template: item.template, caret: item.caret });
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = (selected + 1) % visible.length;
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = (selected - 1 + visible.length) % visible.length;
    } else if (event.key === "Enter") {
      event.preventDefault();
      choose(selected);
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div class="slash" style="top:{top}px; left:{left}px">
  <div class="slash-head">إدراج عنصر</div>
  {#each visible as item, index (item.label)}
    <button
      class="slash-item"
      class:selected={index === selected}
      on:mousemove={() => (selected = index)}
      on:click={() => choose(index)}
    >
      <span class="label">{item.label}</span>
      <span class="hint">{item.hint}</span>
    </button>
  {/each}
  {#if visible.length === 0}
    <div class="slash-empty">لا يوجد عنصر مطابق</div>
  {/if}
</div>

<style>
  .slash {
    position: absolute;
    z-index: 40;
    width: 260px;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    box-shadow: var(--shadow);
    padding: 5px;
    max-height: 320px;
    overflow-y: auto;
  }

  .slash-head {
    color: var(--text-faint);
    font-size: 0.72em;
    padding: 6px 8px 4px;
  }

  .slash-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 7px 9px;
    border-radius: 6px;
    text-align: right;
    font-size: 0.88em;
  }

  .slash-item:hover,
  .slash-item.selected {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .label {
    flex: 1;
  }

  .hint {
    color: var(--text-faint);
    font-size: 0.85em;
  }

  .slash-empty {
    padding: 10px;
    color: var(--text-faint);
    font-size: 0.85em;
  }
</style>
