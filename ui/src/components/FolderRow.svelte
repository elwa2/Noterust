<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { arabicDigits } from "../lib/format";
  import type { FolderNode } from "../lib/types";
  import Self from "./FolderRow.svelte";
  import Icon from "./Icon.svelte";

  export let folder: FolderNode;
  export let depth = 0;
  export let activeId: number | null = null;

  const dispatch = createEventDispatcher<{
    select: number;
    drop: { event: DragEvent; folderId: number | null };
    menu: FolderNode;
    edit: FolderNode;
    addChild: number;
    reorder: { folder: FolderNode; direction: number };
  }>();

  let dragOver = false;
  let expanded = true;

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
    dragOver = true;
  }
</script>

<button
  class="item"
  class:active={activeId === folder.id}
  class:dragover={dragOver}
  style="padding-inline-start:{9 + depth * 14}px"
  on:click={() => dispatch("select", folder.id)}
  on:dragover={handleDragOver}
  on:dragleave={() => (dragOver = false)}
  on:drop={(event) => {
    dragOver = false;
    dispatch("drop", { event, folderId: folder.id });
  }}
  on:contextmenu|preventDefault={(event) => {
    event.stopPropagation();
    dispatch("menu", folder);
  }}
  on:dblclick={() => dispatch("edit", folder)}
  title={folder.name}
>
  {#if folder.children.length > 0}
    <span
      class="chevron"
      class:collapsed={!expanded}
      role="button"
      tabindex="-1"
      on:click|stopPropagation={() => (expanded = !expanded)}
    ><Icon name="chevronDown" size={12} /></span
    >
  {:else}
    <span class="chevron empty"></span>
  {/if}
  <span class="icon" style="color:{folder.color}">{folder.icon}</span>
  <span class="label">{folder.name}</span>
  <span class="badge">{folder.noteCount ? arabicDigits(folder.noteCount) : ""}</span>
</button>

{#if expanded}
  {#each folder.children as child (child.id)}
    <Self
      folder={child}
      depth={depth + 1}
      {activeId}
      on:select
      on:drop
      on:menu
      on:edit
      on:addChild
      on:reorder
    />
  {/each}
{/if}

<style>
  .item {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 6px 9px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    text-align: right;
    font-size: 0.9em;
  }

  .item:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .item.active {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }

  .item.dragover {
    outline: 1px dashed var(--accent);
    background: var(--accent-soft);
  }

  .chevron {
    width: 12px;
    color: var(--text-faint);
    display: grid;
    place-items: center;
    cursor: pointer;
    flex-shrink: 0;
    transition: transform 140ms ease;
  }

  .chevron.collapsed {
    transform: rotate(-90deg);
  }

  .chevron.empty {
    cursor: default;
  }

  .icon {
    width: 17px;
    text-align: center;
    flex-shrink: 0;
  }

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    display: inline-flex;
    min-width: 18px;
    justify-content: center;
    padding: 0 5px;
    border-radius: 999px;
    background: var(--bg-hover);
    color: var(--text-muted);
    font-size: 0.72em;
  }
</style>
