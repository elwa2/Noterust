<script lang="ts">
  import { createEventDispatcher } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import { folders } from "../lib/store";
  import Modal from "./Modal.svelte";
  import type { FolderNode } from "../lib/types";

  export let mode: "create" | "edit" = "create";
  export let folder: FolderNode | null = null;
  export let parentId: number | null = null;

  const dispatch = createEventDispatcher<{ close: void; saved: void }>();

  let name = folder?.name ?? "";
  let icon = folder?.icon ?? "📁";
  let color = folder?.color ?? "#e8b830";
  let parent: number | null = parentId ?? folder?.parentId ?? null;
  let busy = false;
  let error = "";

  const icons = ["📁", "📝", "💼", "🏠", "💡", "📚", "🎯", "🧾", "🛒", "✈️", "🎵", "🔬", "❤️", "⭐"];
  const colors = ["#e8b830", "#e07a5f", "#57c785", "#4fa8e0", "#b48ce8", "#8ab4f8", "#f2a65a", "#9a968f"];

  $: options = store.flattenFolders($folders).filter((item) => item.id !== folder?.id);

  async function submit() {
    const trimmed = name.trim();
    if (!trimmed) {
      error = "اكتب اسماً للمجلد";
      return;
    }
    busy = true;
    error = "";
    try {
      if (mode === "edit" && folder) {
        await api.updateFolder(folder.id, { name: trimmed, icon, color, parentId: parent });
      } else {
        await api.createFolder(parent, trimmed, icon, color);
      }
      dispatch("saved");
      dispatch("close");
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      busy = false;
    }
  }
</script>

<Modal title={mode === "edit" ? "تعديل المجلد" : "مجلد جديد"} on:close={() => dispatch("close")}>
  <div class="field">
    <label for="folder-name">الاسم</label>
    <input
      id="folder-name"
      bind:value={name}
      placeholder="مثال: العمل"
      on:keydown={(event) => event.key === "Enter" && submit()}
    />
  </div>

  <div class="field">
    <label>الأيقونة</label>
    <div class="palette">
      {#each icons as candidate (candidate)}
        <button
          class="swatch icon"
          class:selected={icon === candidate}
          on:click={() => (icon = candidate)}
          title={candidate}
        >
          {candidate}
        </button>
      {/each}
    </div>
  </div>

  <div class="field">
    <label>اللون</label>
    <div class="palette">
      {#each colors as candidate (candidate)}
        <button
          class="swatch"
          class:selected={color === candidate}
          style="background:{candidate}"
          on:click={() => (color = candidate)}
          title={candidate}
        ></button>
      {/each}
    </div>
  </div>

  <div class="field">
    <label for="folder-parent">المجلد الأب</label>
    <select id="folder-parent" bind:value={parent}>
      <option value={null}>بدون (المستوى الأول)</option>
      {#each options as item (item.id)}
        <option value={item.id}>{"—".repeat(item.depth)} {item.name}</option>
      {/each}
    </select>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  <svelte:fragment slot="footer">
    <button class="btn primary" on:click={submit} disabled={busy}>
      {mode === "edit" ? "حفظ التعديلات" : "إنشاء المجلد"}
    </button>
    <button class="btn ghost" on:click={() => dispatch("close")}>إلغاء</button>
  </svelte:fragment>
</Modal>

<style>
  .palette {
    display: flex;
    flex-wrap: wrap;
    gap: 7px;
  }

  .swatch {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    border: 2px solid transparent;
    display: grid;
    place-items: center;
    font-size: 0.9em;
    background: var(--bg-raised);
  }

  .swatch.icon {
    width: 34px;
    height: 34px;
    font-size: 1.05em;
  }

  .swatch.selected {
    border-color: var(--text);
    box-shadow: 0 0 0 2px var(--accent-soft);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.82em;
  }
</style>
