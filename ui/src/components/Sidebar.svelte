<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import { sidebarPrefs, setSidebarCollapsed } from "../lib/sidebarPrefs";
  import * as store from "../lib/store";
  import { view, counts, folders, activeFolder, saveStatus, lastSavedAt } from "../lib/store";
  import { arabicDigits, shortDay } from "../lib/format";
  import FolderRow from "./FolderRow.svelte";
  import FolderDialog from "./FolderDialog.svelte";
  import Icon from "./Icon.svelte";
  import type { FolderNode } from "../lib/types";

  /** Whether an entry is currently shown (users can hide any tab). */
  function visible(id: string): boolean {
    return !$sidebarPrefs.hidden.includes(id);
  }

  let dialog: { mode: "create" | "edit"; parentId?: number | null; folder?: FolderNode } | null =
    null;
  let menu: { folder: FolderNode; x: number; y: number } | null = null;
  let pointer = { x: 0, y: 0 };

  onMount(() => {
    const openNewFolder = () => (dialog = { mode: "create", parentId: null });
    window.addEventListener("noterust:new-folder", openNewFolder);
    const closeMenu = () => (menu = null);
    window.addEventListener("click", closeMenu);
    const trackPointer = (event: MouseEvent) => (pointer = { x: event.clientX, y: event.clientY });
    window.addEventListener("contextmenu", trackPointer, true);
    return () => {
      window.removeEventListener("noterust:new-folder", openNewFolder);
      window.removeEventListener("click", closeMenu);
      window.removeEventListener("contextmenu", trackPointer, true);
    };
  });

  async function dropNote(event: DragEvent, folderId: number | null) {
    event.preventDefault();
    const raw = event.dataTransfer?.getData("text/noterust-note");
    if (!raw) return;
    try {
      await api.moveNote(Number(raw), folderId);
      await store.refreshAll();
      store.toast("تم نقل الملاحظة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function newNoteIn(folder: FolderNode) {
    try {
      const note = await api.createNote(folder.id);
      await store.setView("notes");
      await store.openNote(note.id);
      await store.refreshAll();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function reorder(folder: FolderNode, direction: number) {
    try {
      await api.reorderFolder(folder.id, folder.sortOrder + direction);
      await store.refreshFolders();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function removeFolder(folder: FolderNode) {
    if (!confirm(`حذف مجلد «${folder.name}»؟ ستبقى ملاحظاته في «كل الملاحظات».`)) return;
    try {
      await api.deleteFolder(folder.id);
      if ($activeFolder === folder.id) store.activeFolder.set(null);
      await store.refreshAll();
      store.toast("تم حذف المجلد");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }
</script>

<aside class="sidebar" class:collapsed={$sidebarPrefs.collapsed}>
  <div class="brand">
    <div class="logo">N</div>
    <div class="brand-text">
      <div class="brand-name">NoteRust</div>
      <div class="faint">ملاحظاتك، محلياً</div>
    </div>
    <span class="spacer"></span>
    <button
      class="icon-btn"
      title={$sidebarPrefs.collapsed
        ? "إظهار الشريط الجانبي (Ctrl+B)"
        : "طيّ الشريط إلى أيقونات فقط (Ctrl+B)"}
      on:click={() => setSidebarCollapsed(!$sidebarPrefs.collapsed)}
    >
      <Icon name="sidebar" size={17} />
    </button>
  </div>

  <div class="scroll sections">
    <section>
      <div class="section-title">الملاحظات</div>
      {#if visible("notes")}
        <button
          class="item"
          title="كل الملاحظات"
          class:active={$view === "notes" && $activeFolder === null}
          on:click={() => store.selectFolder(null)}
          on:dragover|preventDefault
          on:drop={(event) => dropNote(event, null)}
        >
          <span class="icon"><Icon name="notes" /></span>
          <span class="label">كل الملاحظات</span>
          <span class="badge">{$counts.all ? arabicDigits($counts.all) : ""}</span>
        </button>
      {/if}

      {#if !$sidebarPrefs.collapsed}
        {#each $folders as folder (folder.id)}
          <FolderRow
            {folder}
            activeId={$view === "notes" ? $activeFolder : null}
            on:select={(event) => store.selectFolder(event.detail)}
            on:drop={(event) => dropNote(event.detail.event, event.detail.folderId)}
            on:edit={(event) => (dialog = { mode: "edit", folder: event.detail })}
            on:menu={(event) => (menu = { folder: event.detail, x: pointer.x, y: pointer.y })}
            on:reorder={(event) => reorder(event.detail.folder, event.detail.direction)}
          />
        {/each}
      {/if}

      {#if visible("trash")}
        <button
          class="item"
          title="المحذوفة مؤخراً"
          class:active={$view === "trash"}
          on:click={() => store.setView("trash")}
        >
          <span class="icon"><Icon name="trash" /></span>
          <span class="label">المحذوفة مؤخراً</span>
          <span class="badge">{$counts.trash ? arabicDigits($counts.trash) : ""}</span>
        </button>
      {/if}
    </section>

    <section>
      <div class="section-title">المتابعة</div>
      {#if visible("tasks")}
        <button class="item" title="المهام" class:active={$view === "tasks"} on:click={() => store.setView("tasks")}>
          <span class="icon"><Icon name="tasks" /></span>
          <span class="label">المهام</span>
          <span class="badge">{$counts.tasksOpen ? arabicDigits($counts.tasksOpen) : ""}</span>
        </button>
      {/if}
      {#if visible("finance")}
        <button class="item" title="الدخل والمصروفات" class:active={$view === "finance"} on:click={() => store.setView("finance")}>
          <span class="icon"><Icon name="finance" /></span>
          <span class="label">الدخل والمصروفات</span>
        </button>
      {/if}
      {#if visible("snippets")}
        <button class="item" title="الأكواد والبرومبت" class:active={$view === "snippets"} on:click={() => store.setView("snippets")}>
          <span class="icon"><Icon name="code" /></span>
          <span class="label">الأكواد والبرومبت</span>
        </button>
      {/if}
      {#if visible("board")}
        <button class="item" title="السبورة" class:active={$view === "board"} on:click={() => store.setView("board")}>
          <span class="icon"><Icon name="board" /></span>
          <span class="label">السبورة</span>
        </button>
      {/if}
      {#if visible("ai")}
        <button class="item" title="الذكاء الاصطناعي" class:active={$view === "ai"} on:click={() => store.setView("ai")}>
          <span class="icon"><Icon name="sparkle" /></span>
          <span class="label">الذكاء الاصطناعي</span>
        </button>
      {/if}
      {#if visible("html")}
        <button class="item" title="معمل HTML" class:active={$view === "html"} on:click={() => store.setView("html")}>
          <span class="icon"><Icon name="html" /></span>
          <span class="label">معمل HTML</span>
        </button>
      {/if}
      {#if visible("calculator")}
        <button
          class="item"
          title="الآلة الحاسبة (Ctrl+Shift+C)"
          on:click={() => window.dispatchEvent(new CustomEvent("noterust:calc"))}
        >
          <span class="icon"><Icon name="calculator" /></span>
          <span class="label">الآلة الحاسبة</span>
        </button>
      {/if}
    </section>
  </div>

  <div class="footer">
    <div class="save-indicator" title="حالة الحفظ">
      <span
        class="dot"
        class:saving={$saveStatus === "saving"}
        class:error={$saveStatus === "error"}
      ></span>
      <span>
        {#if $saveStatus === "saving"}
          يحفظ…
        {:else if $saveStatus === "error"}
          تعذر الحفظ
        {:else if $saveStatus === "unsaved"}
          غير محفوظ
        {:else}
          محفوظ · {shortDay($lastSavedAt)}
        {/if}
      </span>
    </div>

    <div class="footer-actions">
      <button class="btn ghost small" on:click={() => (dialog = { mode: "create", parentId: null })}>
        <Icon name="folderPlus" size={15} /> مجلد جديد
      </button>
      <button class="icon-btn" title="الإعدادات (Ctrl+,)" on:click={() => store.setView("settings")}>
        <Icon name="settings" />
      </button>
    </div>
  </div>
</aside>

{#if menu}
  <div class="context-menu" style="top:{menu.y}px; left:{menu.x}px" on:click|stopPropagation>
    <button on:click={() => { dialog = { mode: "edit", folder: menu!.folder }; menu = null; }}>
      تعديل المجلد
    </button>
    <button on:click={() => { void newNoteIn(menu!.folder); menu = null; }}>
      ملاحظة جديدة هنا
    </button>
    <button on:click={() => { dialog = { mode: "create", parentId: menu!.folder.id }; menu = null; }}>
      مجلد فرعي
    </button>
    <button on:click={() => { void reorder(menu!.folder, -1); menu = null; }}>نقل لأعلى</button>
    <button on:click={() => { void reorder(menu!.folder, 1); menu = null; }}>نقل لأسفل</button>
    <button class="danger" on:click={() => { void removeFolder(menu!.folder); menu = null; }}>
      حذف
    </button>
  </div>
{/if}

{#if dialog}
  <FolderDialog
    mode={dialog.mode}
    folder={dialog.folder}
    parentId={dialog.parentId ?? dialog.folder?.parentId ?? null}
    on:close={() => (dialog = null)}
    on:saved={() => {
      dialog = null;
      void store.refreshAll();
    }}
  />
{/if}

<style>
  .sidebar {
    background: var(--bg-elevated);
    border-inline-end: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-width: 0;
    overflow: hidden;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 14px 12px 10px 16px;
  }

  .brand-text {
    min-width: 0;
  }

  .logo {
    width: 32px;
    height: 32px;
    border-radius: 9px;
    background: var(--accent);
    color: var(--accent-text);
    font-weight: 700;
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .brand-name {
    font-weight: 700;
    font-size: 1.02em;
  }

  .sections {
    flex: 1;
    padding: 0 8px 8px;
  }

  section {
    margin-bottom: 14px;
  }

  .section-title {
    color: var(--text-faint);
    font-size: 0.72em;
    padding: 8px 8px 4px;
  }

  .item {
    display: flex;
    align-items: center;
    gap: 9px;
    width: 100%;
    padding: 7px 9px;
    border-radius: var(--radius-sm);
    color: var(--text-muted);
    text-align: right;
    font-size: 0.92em;
  }

  /* Icon-only rail: names, badges and section titles step aside. */
  .sidebar.collapsed .brand-text,
  .sidebar.collapsed .logo,
  .sidebar.collapsed .label,
  .sidebar.collapsed .badge,
  .sidebar.collapsed .section-title,
  .sidebar.collapsed .save-indicator span:last-child,
  .sidebar.collapsed .footer-actions .btn {
    display: none;
  }

  .sidebar.collapsed .brand {
    justify-content: center;
    gap: 0;
    padding: 14px 6px 10px;
  }

  .sidebar.collapsed .brand .spacer {
    display: none;
  }

  .sidebar.collapsed .sections {
    padding: 0 6px 8px;
  }

  .sidebar.collapsed section {
    margin-bottom: 8px;
  }

  .sidebar.collapsed .item {
    justify-content: center;
    padding: 9px 4px;
  }

  .sidebar.collapsed .footer-actions {
    justify-content: center;
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

  .label {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .icon {
    width: 18px;
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .badge {
    display: inline-flex;
    min-width: 20px;
    justify-content: center;
    padding: 0 6px;
    border-radius: 999px;
    background: var(--bg-hover);
    color: var(--text-muted);
    font-size: 0.72em;
  }

  .footer {
    border-top: 1px solid var(--border);
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .save-indicator {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 0.78em;
    color: var(--text-muted);
    white-space: nowrap;
    overflow: hidden;
  }

  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--saved);
    box-shadow: 0 0 0 3px rgba(87, 199, 133, 0.15);
    flex-shrink: 0;
  }

  .dot.saving {
    background: var(--accent);
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .dot.error {
    background: var(--danger);
    box-shadow: 0 0 0 3px rgba(224, 122, 95, 0.18);
  }

  .footer-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .btn.small {
    font-size: 0.8em;
    padding: 5px 9px;
    flex: 1;
    justify-content: center;
  }

  .context-menu {
    position: fixed;
    z-index: 70;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    padding: 5px;
    display: flex;
    flex-direction: column;
    min-width: 170px;
  }

  .context-menu button {
    text-align: right;
    padding: 7px 10px;
    border-radius: 5px;
    font-size: 0.88em;
  }

  .context-menu button:hover {
    background: var(--bg-hover);
  }

  .context-menu button.danger {
    color: var(--danger);
  }
</style>
