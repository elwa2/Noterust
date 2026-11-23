<script lang="ts">
  import { arabicDigits, fileSize } from "../lib/format";
  import type { Note } from "../lib/types";

  export let content = "";
  export let note: Note | null = null;
  export let wrap = true;

  $: words = countWords(content);
  $: chars = content.length;
  $: lines = content.length === 0 ? 1 : content.split("\n").length;

  function countWords(text: string): number {
    const trimmed = text.trim();
    if (!trimmed) return 0;
    return trimmed.split(/\s+/).length;
  }
</script>

<footer class="status">
  <span title="عدد الكلمات">{arabicDigits(words)} كلمة</span>
  <span title="عدد الأحرف">{arabicDigits(chars)} حرف</span>
  <span title="عدد الأسطر">{arabicDigits(lines)} سطر</span>
  <span class="spacer"></span>
  {#if note?.sourcePath}
    <span class="chip" title={note.sourcePath}>📄 {note.encoding ?? "UTF-8"}</span>
    <span class="chip">{note.lineEnding ?? "LF"}</span>
  {:else}
    <span class="chip">Markdown</span>
  {/if}
  <span class="chip">{wrap ? "التفاف الأسطر" : "بدون التفاف"}</span>
  <span class="chip">{note?.direction === "ltr" ? "LTR" : note?.direction === "rtl" ? "RTL" : "تلقائي"}</span>
</footer>

<style>
  .status {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 6px 16px;
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-faint);
    font-size: 0.72em;
    flex-wrap: wrap;
  }

  .chip {
    background: var(--bg-raised);
    border-radius: 5px;
    padding: 1px 7px;
    white-space: nowrap;
  }
</style>
