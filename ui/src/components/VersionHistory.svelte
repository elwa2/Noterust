<script lang="ts">
  import { createEventDispatcher, onMount } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import { arabicDateTime, arabicDigits } from "../lib/format";
  import Modal from "./Modal.svelte";
  import type { Note, NoteVersion } from "../lib/types";

  export let noteId: number;

  const dispatch = createEventDispatcher<{ close: void; restored: Note }>();

  let versions: NoteVersion[] = [];
  let selected: NoteVersion | null = null;
  let loading = true;
  let error = "";

  onMount(async () => {
    try {
      versions = await api.noteVersions(noteId);
      selected = versions[0] ?? null;
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      loading = false;
    }
  });

  async function restore(version: NoteVersion) {
    if (!confirm("استرجاع هذه النسخة؟ سيُحفظ النص الحالي كنسخة أولاً.")) return;
    try {
      const note = await api.restoreVersion(version.id);
      store.toast("تم استرجاع النسخة");
      dispatch("restored", note);
      dispatch("close");
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }
</script>

<Modal title="تاريخ الإصدارات" wide on:close={() => dispatch("close")}>
  {#if loading}
    <p class="muted">جارٍ التحميل…</p>
  {:else if error}
    <p class="error">{error}</p>
  {:else if versions.length === 0}
    <p class="muted">لا توجد نسخ محفوظة بعد. تُنشأ النسخة عند الحفظ بعد تعديل كبير.</p>
  {:else}
    <div class="history">
      <div class="list scroll">
        {#each versions as version (version.id)}
          <button
            class="version"
            class:active={selected?.id === version.id}
            on:click={() => (selected = version)}
          >
            <span class="when">{arabicDateTime(version.savedAt)}</span>
            <span class="meta">{arabicDigits(version.content.length)} حرف</span>
          </button>
        {/each}
      </div>
      <div class="preview">
        {#if selected}
          <div class="preview-head">
            <span class="muted">{arabicDateTime(selected.savedAt)}</span>
            <span class="spacer"></span>
            <button class="btn small" on:click={() => restore(selected!)}>استرجاع هذه النسخة</button>
          </div>
          <pre class="scroll" dir="auto">{selected.content || "—"}</pre>
        {/if}
      </div>
    </div>
  {/if}
</Modal>

<style>
  .history {
    display: grid;
    grid-template-columns: 210px 1fr;
    gap: 14px;
    min-height: 340px;
  }

  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: 52vh;
    overflow-y: auto;
  }

  .version {
    display: flex;
    flex-direction: column;
    gap: 2px;
    align-items: flex-start;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    background: var(--bg-raised);
    border: 1px solid transparent;
    text-align: right;
  }

  .version:hover {
    background: var(--bg-hover);
  }

  .version.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .when {
    font-size: 0.82em;
  }

  .meta {
    font-size: 0.72em;
    color: var(--text-faint);
  }

  .preview {
    display: flex;
    flex-direction: column;
    gap: 8px;
    min-width: 0;
  }

  .preview-head {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8em;
  }

  pre {
    margin: 0;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px;
    max-height: 46vh;
    overflow: auto;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--font-family);
    font-size: 0.88em;
    line-height: 1.8;
  }

  .btn.small {
    font-size: 0.8em;
    padding: 4px 10px;
  }

  .error {
    color: var(--danger);
  }
</style>
