<script lang="ts">
  import { createEventDispatcher } from "svelte";

  export let title = "";
  export let wide = false;

  const dispatch = createEventDispatcher<{ close: void }>();

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      dispatch("close");
    }
  }
</script>

<svelte:window on:keydown={onKeydown} />

<div
  class="modal-backdrop"
  role="presentation"
  on:mousedown|self={() => dispatch("close")}
>
  <div class="modal" class:wide role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <span>{title}</span>
      <span class="spacer"></span>
      <button class="icon-btn" title="إغلاق" on:click={() => dispatch("close")}>✕</button>
    </header>
    <div class="body">
      <slot />
    </div>
    {#if $$slots.footer}
      <footer>
        <slot name="footer" />
      </footer>
    {/if}
  </div>
</div>

<style>
  .modal.wide {
    width: min(860px, 94vw);
  }

  .modal header span:first-child {
    font-size: 0.98em;
  }
</style>
