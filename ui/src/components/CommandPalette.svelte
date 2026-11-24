<script lang="ts">
  import { onMount } from "svelte";
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import type { NoteSummary } from "../lib/types";

  const dispatch = createEventDispatcher<{
    close: void;
    openFile: void;
    newFolder: void;
    help: void;
  }>();

  interface Command {
    label: string;
    hint: string;
    run: () => void | Promise<void>;
  }

  let query = "";
  let selected = 0;
  let input: HTMLInputElement | null = null;
  let results: NoteSummary[] = [];
  let searchTimer: ReturnType<typeof setTimeout> | undefined;

  const commands: Command[] = [
    { label: "ملاحظة جديدة", hint: "Ctrl+N", run: () => void store.createNote() },
    { label: "فتح ملف من القرص", hint: "Ctrl+O", run: () => dispatch("openFile") },
    { label: "مجلد جديد", hint: "Ctrl+Shift+N", run: () => dispatch("newFolder") },
    { label: "المهام", hint: "", run: () => void store.setView("tasks") },
    { label: "الدخل والمصروفات", hint: "", run: () => void store.setView("finance") },
    { label: "الأكواد والبرومبت", hint: "Ctrl+4", run: () => void store.setView("snippets") },
    {
      label: "الآلة الحاسبة",
      hint: "Ctrl+Shift+C",
      run: () => window.dispatchEvent(new CustomEvent("noterust:calc")),
    },
    { label: "الإعدادات", hint: "Ctrl+,", run: () => void store.setView("settings") },
    { label: "سلة المحذوفات", hint: "", run: () => void store.setView("trash") },
    { label: "دليل الاستخدام", hint: "F1", run: () => dispatch("help") },
    {
      label: "نسخة احتياطية الآن",
      hint: "",
      run: async () => {
        const path = await api.backupNow();
        store.toast(`تم إنشاء نسخة: ${path}`);
      },
    },
    {
      label: "تصدير كل البيانات JSON",
      hint: "",
      run: async () => {
        const path = await api.exportJson();
        if (path) store.toast(`تم التصدير إلى ${path}`);
      },
    },
    {
      label: "تصدير الملاحظات ZIP",
      hint: "",
      run: async () => {
        const path = await api.exportZip();
        if (path) store.toast(path);
      },
    },
    {
      label: "استيراد ملفات نصية",
      hint: "",
      run: async () => {
        const stats = await api.importDialog(null);
        if (stats) store.toast(`تم استيراد ${stats.notes} ملاحظة`);
        void store.refreshAll();
      },
    },
  ];

  $: visible = commands.filter((command) => command.label.includes(query.trim()));
  $: if (selected >= visible.length) selected = 0;

  onMount(() => {
    input?.focus();
  });

  function onInput() {
    clearTimeout(searchTimer);
    const text = query.trim();
    if (text.length < 2) {
      results = [];
      return;
    }
    searchTimer = setTimeout(async () => {
      try {
        results = await api.searchNotes(text, null);
      } catch {
        results = [];
      }
    }, 150);
  }

  function runCommand(index: number) {
    const command = visible[index];
    if (!command) return;
    dispatch("close");
    void Promise.resolve(command.run()).catch((error) => store.toast(store.errorMessage(error), "error"));
  }

  async function openResult(note: NoteSummary) {
    dispatch("close");
    await store.openNote(note.id);
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      dispatch("close");
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      selected = Math.min(visible.length - 1, selected + 1);
    } else if (event.key === "ArrowUp") {
      event.preventDefault();
      selected = Math.max(0, selected - 1);
    } else if (event.key === "Enter") {
      event.preventDefault();
      runCommand(selected);
    }
  }
</script>

<div class="palette-backdrop" role="presentation" on:mousedown|self={() => dispatch("close")}>
  <div class="palette" role="dialog" aria-modal="true" aria-label="لوحة الأوامر">
    <input
      bind:this={input}
      bind:value={query}
      placeholder="اكتب أمراً أو ابحث في الملاحظات…"
      on:input={onInput}
      on:keydown={onKeydown}
    />

    <div class="list scroll">
      {#if visible.length > 0}
        <div class="section">الأوامر</div>
        {#each visible as command, index (command.label)}
          <button
            class="entry"
            class:selected={index === selected}
            on:mousemove={() => (selected = index)}
            on:click={() => runCommand(index)}
          >
            <span class="label">{command.label}</span>
            {#if command.hint}<span class="hint">{command.hint}</span>{/if}
          </button>
        {/each}
      {/if}

      {#if results.length > 0}
        <div class="section">الملاحظات</div>
        {#each results.slice(0, 8) as result (result.id)}
          <button class="entry" on:click={() => openResult(result)}>
            <span class="label">{result.title || "بدون عنوان"}</span>
            <span class="hint">{result.preview.slice(0, 60)}</span>
          </button>
        {/each}
      {/if}

      {#if visible.length === 0 && results.length === 0}
        <p class="empty">لا نتائج</p>
      {/if}
    </div>

    <div class="foot">
      <span><kbd>↑</kbd><kbd>↓</kbd> تنقل</span>
      <span><kbd>Enter</kbd> تنفيذ</span>
      <span><kbd>Esc</kbd> إغلاق</span>
    </div>
  </div>
</div>

<style>
  .palette-backdrop {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.45);
    display: flex;
    justify-content: center;
    padding-top: 12vh;
    z-index: 75;
  }

  .palette {
    width: min(600px, 92vw);
    max-height: 64vh;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: 14px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .palette > input {
    border: none;
    border-bottom: 1px solid var(--border);
    border-radius: 0;
    padding: 14px 18px;
    font-size: 1em;
    background: transparent;
  }

  .list {
    flex: 1;
    padding: 6px;
    overflow-y: auto;
  }

  .section {
    color: var(--text-faint);
    font-size: 0.7em;
    padding: 8px 10px 4px;
  }

  .entry {
    display: flex;
    align-items: center;
    gap: 10px;
    width: 100%;
    padding: 8px 11px;
    border-radius: var(--radius-sm);
    text-align: right;
  }

  .entry:hover,
  .entry.selected {
    background: var(--accent-soft);
    color: var(--accent);
  }

  .label {
    flex: 1;
    font-size: 0.9em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hint {
    color: var(--text-faint);
    font-size: 0.76em;
    max-width: 55%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    color: var(--text-faint);
    text-align: center;
    font-size: 0.85em;
    padding: 18px;
  }

  .foot {
    display: flex;
    gap: 14px;
    padding: 8px 14px;
    border-top: 1px solid var(--border);
    color: var(--text-faint);
    font-size: 0.72em;
  }
</style>
