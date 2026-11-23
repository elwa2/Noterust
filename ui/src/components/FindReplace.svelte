<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let find = "";
  export let replace = "";
  export let showReplace = false;
  export let useRegex = false;
  export let caseSensitive = false;
  export let matchCount = 0;
  export let error = "";

  const dispatch = createEventDispatcher<{
    findNext: void;
    findPrev: void;
    replaceOne: void;
    replaceAll: void;
    close: void;
    changed: void;
  }>();

  function changed() {
    dispatch("changed");
  }
</script>

<div class="find">
  <div class="row">
    <input
      placeholder="بحث…"
      bind:value={find}
      on:input={changed}
      on:keydown={(event) => {
        if (event.key === "Enter") {
          event.preventDefault();
          dispatch(event.shiftKey ? "findPrev" : "findNext");
        }
        if (event.key === "Escape") dispatch("close");
      }}
    />
    <span class="count">{matchCount} نتيجة</span>
    <button class="icon-btn" title="السابق" on:click={() => dispatch("findPrev")}>↑</button>
    <button class="icon-btn" title="التالي" on:click={() => dispatch("findNext")}>↓</button>
    <button
      class="icon-btn"
      class:active={caseSensitive}
      title="حساسية الأحرف"
      on:click={() => {
        caseSensitive = !caseSensitive;
        changed();
      }}>Aa</button
    >
    <button
      class="icon-btn"
      class:active={useRegex}
      title="تعبير نمطي"
      on:click={() => {
        useRegex = !useRegex;
        changed();
      }}>.*</button
    >
    <button class="icon-btn" title="إغلاق" on:click={() => dispatch("close")}>✕</button>
  </div>

  {#if showReplace}
    <div class="row">
      <input
        placeholder="استبدال بـ…"
        bind:value={replace}
        on:input={changed}
        on:keydown={(event) => {
          if (event.key === "Enter") {
            event.preventDefault();
            dispatch("replaceOne");
          }
        }}
      />
      <button class="btn small" on:click={() => dispatch("replaceOne")}>استبدال</button>
      <button class="btn small" on:click={() => dispatch("replaceAll")}>الكل</button>
    </div>
  {/if}

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .find {
    border-top: 1px solid var(--border);
    background: var(--bg-elevated);
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  input {
    flex: 1;
    padding: 5px 9px;
    font-size: 0.85em;
    min-width: 0;
  }

  .count {
    font-size: 0.75em;
    color: var(--text-faint);
    white-space: nowrap;
  }

  .btn.small {
    font-size: 0.78em;
    padding: 4px 9px;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.76em;
  }
</style>
