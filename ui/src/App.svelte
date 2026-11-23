<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api } from "./lib/api";
  import * as store from "./lib/store";
  import {
    settings,
    view,
    tabs,
    activeTab,
    activeNote,
    sortBy,
    activeFolder,
    toasts,
  } from "./lib/store";
  import { sendEditorCommand } from "./lib/bus";
  import Sidebar from "./components/Sidebar.svelte";
  import NoteList from "./components/NoteList.svelte";
  import Editor from "./components/Editor.svelte";
  import TasksView from "./components/TasksView.svelte";
  import FinanceView from "./components/FinanceView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import HelpDialog from "./components/HelpDialog.svelte";
  import Icon from "./components/Icon.svelte";

  let paletteOpen = false;
  let helpOpen = false;
  let themeMedia: MediaQueryList | null = null;

  let sidebarHidden = false;
  let sidebarWidth = 246;
  let listWidth = 322;
  const UI_KEY = "noterust.ui";

  function loadUi() {
    try {
      const raw = localStorage.getItem(UI_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw) as Partial<{
        sidebarHidden: boolean;
        sidebarWidth: number;
        listWidth: number;
      }>;
      if (typeof saved.sidebarWidth === "number") sidebarWidth = clamp(saved.sidebarWidth, 200, 380);
      if (typeof saved.listWidth === "number") listWidth = clamp(saved.listWidth, 260, 520);
      if (typeof saved.sidebarHidden === "boolean") sidebarHidden = saved.sidebarHidden;
    } catch {
      /* ignore corrupt prefs */
    }
  }

  function saveUi() {
    localStorage.setItem(UI_KEY, JSON.stringify({ sidebarHidden, sidebarWidth, listWidth }));
  }

  function clamp(value: number, min: number, max: number) {
    return Math.min(max, Math.max(min, value));
  }

  let resizing: "sidebar" | "list" | null = null;

  function startResize(kind: "sidebar" | "list", event: PointerEvent) {
    event.preventDefault();
    resizing = kind;
    const onMove = (move: PointerEvent) => {
      if (kind === "sidebar") {
        sidebarWidth = clamp(window.innerWidth - move.clientX, 200, 380);
      } else {
        const sidebar = document.querySelector(".sidebar");
        const right = sidebar ? sidebar.getBoundingClientRect().left : window.innerWidth;
        listWidth = clamp(right - move.clientX, 260, 520);
      }
    };
    const onUp = () => {
      resizing = null;
      document.body.classList.remove("col-resizing");
      window.removeEventListener("pointermove", onMove);
      saveUi();
    };
    document.body.classList.add("col-resizing");
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", onUp, { once: true });
  }

  function toggleSidebar() {
    sidebarHidden = !sidebarHidden;
    saveUi();
  }

  $: theme = $settings["theme"] ?? "dark";
  $: fontFamily = $settings["font_family"] ?? "Cairo";
  $: fontSize = Number($settings["font_size"] ?? 16) || 16;
  $: fontStack =
    fontFamily === "Cairo"
      ? '"Cairo", "Segoe UI", "Noto Naskh Arabic", system-ui, sans-serif'
      : `"${fontFamily}", "Cairo", "Segoe UI", system-ui, sans-serif`;
  $: notesMode = $view === "notes" || $view === "trash";

  $: if (theme !== undefined && typeof document !== "undefined") applyTheme();

  function applyTheme() {
    const prefersDark = themeMedia?.matches ?? true;
    const resolved = theme === "system" ? (prefersDark ? "dark" : "light") : theme;
    document.documentElement.dataset.theme = resolved;
  }

  onMount(() => {
    loadUi();
    themeMedia = window.matchMedia("(prefers-color-scheme: dark)");
    themeMedia.addEventListener("change", applyTheme);
    void bootstrap();

    const unlisteners = [
      listen<number>("file-changed", (event) => store.fileChangeNotice.set(event.payload)),
      listen("open-files", () => void openPendingFiles()),
      listen("new-note", () => void store.createNote()),
    ];

    const onKeydown = (event: KeyboardEvent) => handleShortcut(event);
    window.addEventListener("keydown", onKeydown);

    return () => {
      window.removeEventListener("keydown", onKeydown);
      themeMedia?.removeEventListener("change", applyTheme);
      for (const pending of unlisteners) {
        void pending.then((unlisten) => unlisten());
      }
    };
  });

  async function bootstrap() {
    await store.loadSettings();
    applyTheme();

    const session = store.loadSession();
    if (session) {
      sortBy.set(session.sortBy ?? "updated");
      activeFolder.set(session.folderId ?? null);
      view.set(session.view ?? "notes");
    }

    await store.refreshAll();

    if (session?.tabs?.length) {
      for (const id of session.tabs) {
        await store.openNote(id, true);
      }
      if (session.activeTab !== null && session.tabs.includes(session.activeTab)) {
        activeTab.set(session.activeTab);
        const note = await api.getNote(session.activeTab).catch(() => null);
        store.updateWindowTitle(note);
      }
    }

    await openPendingFiles();
    void api.refreshDailyBackup().catch(() => undefined);
  }

  async function openPendingFiles() {
    try {
      const files = await api.openStartupFiles();
      for (const file of files) {
        const opened = await api.openPath(file);
        await store.openNote(opened.noteId);
        if (opened.truncated) {
          store.toast("الملف كبير جداً — حُمّل جزء منه فقط وقد يُمنع الحفظ.", "error");
        }
      }
      if (files.length > 0) await store.refreshAll();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  function isTypingTarget(target: EventTarget | null): boolean {
    const element = target as HTMLElement | null;
    if (!element) return false;
    return (
      element.tagName === "INPUT" || element.tagName === "TEXTAREA" || element.isContentEditable
    );
  }

  function handleShortcut(event: KeyboardEvent) {
    const ctrl = event.ctrlKey || event.metaKey;
    const key = event.key.toLowerCase();

    if (event.key === "F1") {
      event.preventDefault();
      helpOpen = true;
      return;
    }
    if (ctrl && key === "b") {
      event.preventDefault();
      toggleSidebar();
      return;
    }
    if (ctrl && (event.key === "1" || event.key === "2" || event.key === "3") && !event.shiftKey) {
      event.preventDefault();
      void store.setView((["notes", "tasks", "finance"] as const)[Number(event.key) - 1]);
      return;
    }
    if (ctrl && event.shiftKey && key === "f") {
      event.preventDefault();
      void store.setView("notes");
      document.getElementById("global-search")?.focus();
      return;
    }
    if (ctrl && key === "p") {
      event.preventDefault();
      paletteOpen = true;
      return;
    }
    if (ctrl && event.shiftKey && key === "n") {
      event.preventDefault();
      window.dispatchEvent(new CustomEvent("noterust:new-folder"));
      return;
    }
    if (ctrl && key === "n") {
      event.preventDefault();
      void store.createNote();
      return;
    }
    if (ctrl && key === "o") {
      event.preventDefault();
      void openFile();
      return;
    }
    if (ctrl && key === ",") {
      event.preventDefault();
      void store.setView("settings");
      return;
    }
    if (ctrl && key === "w") {
      event.preventDefault();
      if ($activeTab !== null) void store.closeTab($activeTab);
      return;
    }
    if (ctrl && key === "tab") {
      event.preventDefault();
      cycleTab(event.shiftKey ? -1 : 1);
      return;
    }
    if (ctrl && key === "s") {
      event.preventDefault();
      sendEditorCommand(event.shiftKey ? "save-as" : "save");
      return;
    }
    if (ctrl && key === "f") {
      event.preventDefault();
      sendEditorCommand("find");
      return;
    }
    if (ctrl && key === "h") {
      event.preventDefault();
      sendEditorCommand("replace");
      return;
    }
    if (ctrl && key === "g") {
      event.preventDefault();
      sendEditorCommand("go-to-line");
      return;
    }
    if (ctrl && key === "d" && isTypingTarget(event.target)) {
      event.preventDefault();
      sendEditorCommand("duplicate-line");
      return;
    }
    if (event.key === "F5") {
      event.preventDefault();
      sendEditorCommand("insert-date");
      return;
    }
    if (event.key === "Delete" && !isTypingTarget(event.target) && $activeTab !== null) {
      event.preventDefault();
      void removeActiveNote();
      return;
    }
    if (event.key === "Escape") {
      paletteOpen = false;
      helpOpen = false;
    }
  }

  async function removeActiveNote() {
    const note = $activeNote;
    if (!note) return;
    if (!confirm("نقل الملاحظة إلى سلة المحذوفات؟")) return;
    try {
      await api.deleteNote(note.id);
      await store.removeNoteFromUi(note.id);
      store.toast("نُقلت إلى سلة المحذوفات", "ok", {
        label: "تراجع",
        run: () => {
          void api
            .restoreNote(note.id)
            .then(async () => {
              await store.refreshAll();
              store.toast("تم الاسترجاع");
            })
            .catch((error) => store.toast(store.errorMessage(error), "error"));
        },
      });
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  function cycleTab(direction: number) {
    const list = $tabs;
    if (list.length < 2) return;
    const index = list.findIndex((note) => note.id === $activeTab);
    const next = (index + direction + list.length) % list.length;
    activeTab.set(list[next].id);
    store.updateWindowTitle(list[next]);
    store.saveSession();
  }

  async function openFile() {
    try {
      const opened = await api.openFileDialog();
      if (opened) {
        await store.openNote(opened.noteId);
        await store.refreshAll();
      }
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }
</script>

<div
  class="app"
  class:notes-mode={notesMode}
  class:sidebar-hidden={sidebarHidden}
  style="--sidebar-w:{sidebarWidth}px; --list-w:{listWidth}px; --font-size:{fontSize}px; --font-family:{fontStack}"
>
  {#if !sidebarHidden}
    <Sidebar on:toggleSidebar={toggleSidebar} />
    <div
      class="resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="تغيير عرض الشريط الجانبي"
      class:dragging={resizing === "sidebar"}
      on:pointerdown={(event) => startResize("sidebar", event)}
    ></div>
  {:else}
    <div class="rail"></div>
    <div class="rail-gap"></div>
  {/if}

  {#if notesMode}
    <NoteList />
    <div
      class="resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="تغيير عرض قائمة الملاحظات"
      class:dragging={resizing === "list"}
      on:pointerdown={(event) => startResize("list", event)}
    ></div>
    <Editor />
  {:else if $view === "tasks"}
    <TasksView />
  {:else if $view === "finance"}
    <FinanceView />
  {:else if $view === "settings"}
    <SettingsView />
  {/if}
</div>

{#if sidebarHidden}
  <button class="sidebar-reopen" title="إظهار الشريط الجانبي (Ctrl+B)" on:click={toggleSidebar}>
    <Icon name="sidebar" size={17} />
  </button>
{/if}

{#if paletteOpen}
  <CommandPalette
    on:close={() => (paletteOpen = false)}
    on:openFile={() => {
      paletteOpen = false;
      void openFile();
    }}
    on:newFolder={() => {
      paletteOpen = false;
      window.dispatchEvent(new CustomEvent("noterust:new-folder"));
    }}
    on:help={() => {
      paletteOpen = false;
      helpOpen = true;
    }}
  />
{/if}

{#if helpOpen}
  <HelpDialog on:close={() => (helpOpen = false)} />
{/if}

<div class="toasts">
  {#each $toasts as item (item.id)}
    <div class="toast" class:error={item.kind === "error"}>
      <span>{item.text}</span>
      {#if item.action}
        <button
          class="toast-action"
          on:click={() => {
            item.action?.run();
            store.dismissToast(item.id);
          }}
          >{item.action.label}</button
        >
      {/if}
    </div>
  {/each}
</div>

<style>
  .app {
    display: grid;
    grid-template-columns: var(--sidebar-w) 6px var(--list-w) 6px 1fr;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
    font-family: var(--font-family);
    font-size: var(--font-size);
  }

  .app:not(.notes-mode) {
    grid-template-columns: var(--sidebar-w) 0 0 0 1fr;
  }

  .app.sidebar-hidden {
    grid-template-columns: 0 0 var(--list-w) 6px 1fr;
  }

  .app.sidebar-hidden:not(.notes-mode) {
    grid-template-columns: 0 0 0 0 1fr;
  }

  .resizer {
    cursor: col-resize;
    background: transparent;
    position: relative;
    z-index: 5;
    touch-action: none;
  }

  .resizer::after {
    content: "";
    position: absolute;
    inset: 0 2px;
    border-radius: 2px;
    background: transparent;
    transition: background 120ms ease;
  }

  .resizer:hover::after,
  .resizer.dragging::after {
    background: var(--accent);
  }

  .rail,
  .rail-gap {
    background: var(--bg-elevated);
  }

  .sidebar-reopen {
    position: fixed;
    top: 8px;
    left: 8px;
    z-index: 50;
    width: 30px;
    height: 30px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  .sidebar-reopen:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .toasts {
    position: fixed;
    bottom: 46px;
    left: 50%;
    transform: translateX(-50%);
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 90;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 12px;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-inline-start: 3px solid var(--accent);
    color: var(--text);
    padding: 9px 16px;
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    font-size: 0.9em;
    max-width: 70vw;
    pointer-events: auto;
  }

  .toast.error {
    border-inline-start-color: var(--danger);
  }

  .toast-action {
    color: var(--accent);
    font-size: 0.92em;
    font-weight: 600;
    white-space: nowrap;
  }

  .toast-action:hover {
    text-decoration: underline;
  }

  :global(body.col-resizing) {
    cursor: col-resize;
    user-select: none;
  }
</style>
