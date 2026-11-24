<script lang="ts">
  import { setView } from "../lib/store";
  import Icon from "./Icon.svelte";
</script>

<button class="back" title="الرجوع إلى الملاحظات (Esc)" on:click={() => void setView("notes")}>
  <Icon name="chevronRight" size={15} />
  <span>الملاحظات</span>
</button>

<style>
  .back {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 5px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-size: 0.8em;
    flex-shrink: 0;
  }

  .back:hover {
    color: var(--accent);
    border-color: var(--accent);
  }
</style>
