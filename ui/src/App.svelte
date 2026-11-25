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
    compact,
    alwaysOnTop,
  } from "./lib/store";
  import { sendEditorCommand } from "./lib/bus";
  import { sidebarPrefs, setSidebarCollapsed } from "./lib/sidebarPrefs";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import Sidebar from "./components/Sidebar.svelte";
  import NoteList from "./components/NoteList.svelte";
  import Editor from "./components/Editor.svelte";
  import TasksView from "./components/TasksView.svelte";
  import FinanceView from "./components/FinanceView.svelte";
  import SnippetsView from "./components/SnippetsView.svelte";
  import WhiteboardView from "./components/WhiteboardView.svelte";
  import AiView from "./components/AiView.svelte";
  import HtmlView from "./components/HtmlView.svelte";
  import SettingsView from "./components/SettingsView.svelte";
  import CommandPalette from "./components/CommandPalette.svelte";
  import HelpDialog from "./components/HelpDialog.svelte";
  import CalculatorDialog from "./components/CalculatorDialog.svelte";
  import Icon from "./components/Icon.svelte";

  let paletteOpen = false;
  let helpOpen = false;
  let calcOpen = false;
  let themeMedia: MediaQueryList | null = null;

  let sidebarWidth = 246;
  let listWidth = 322;
  const UI_KEY = "noterust.ui";

  function loadUi() {
    try {
      const raw = localStorage.getItem(UI_KEY);
      if (!raw) return;
      const saved = JSON.parse(raw) as Partial<{
        sidebarWidth: number;
        listWidth: number;
      }>;
      if (typeof saved.sidebarWidth === "number") sidebarWidth = clamp(saved.sidebarWidth, 200, 380);
      if (typeof saved.listWidth === "number") listWidth = clamp(saved.listWidth, 260, 520);
    } catch {
      /* ignore corrupt prefs */
    }
  }

  function saveUi() {
    localStorage.setItem(UI_KEY, JSON.stringify({ sidebarWidth, listWidth }));
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
    setSidebarCollapsed(!$sidebarPrefs.collapsed);
  }

  // ---- Frameless window: the app draws its own title bar ----
  const appWindow = getCurrentWindow();
  let maximized = false;

  $: windowTitle = $activeNote?.title?.trim() || "NoteRust";

  function startWindowDrag(event: PointerEvent) {
    if (event.button !== 0) return;
    void appWindow.startDragging().catch(() => undefined);
  }

  async function toggleMaximize() {
    try {
      await appWindow.toggleMaximize();
      maximized = await appWindow.isMaximized();
    } catch {
      /* ignore unavailable window control */
    }
  }

  function minimizeWindow() {
    void appWindow.minimize().catch(() => undefined);
  }

  function closeWindow() {
    void appWindow.close().catch(() => undefined);
  }

  function startWindowResize(direction: "North" | "South" | "East" | "West" | "NorthEast" | "NorthWest" | "SouthEast" | "SouthWest") {
    void appWindow.startResizeDragging(direction).catch(() => undefined);
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

    const openCalc = () => (calcOpen = true);
    window.addEventListener("noterust:calc", openCalc);
    window.addEventListener("contextmenu", onContextMenu);

    return () => {
      window.removeEventListener("keydown", onKeydown);
      window.removeEventListener("noterust:calc", openCalc);
      window.removeEventListener("contextmenu", onContextMenu);
      themeMedia?.removeEventListener("change", applyTheme);
      for (const pending of unlisteners) {
        void pending.then((unlisten) => unlisten());
      }
    };
  });

  /** The webview's own menu (Back/Refresh/Print) is never useful here; keep it
   *  only inside text fields where copy/paste matters. */
  function onContextMenu(event: MouseEvent) {
    const target = event.target as HTMLElement | null;
    const tag = target?.tagName ?? "";
    if (tag === "INPUT" || tag === "TEXTAREA" || target?.isContentEditable) return;
    event.preventDefault();
  }

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
    void appWindow
      .isMaximized()
      .then((value) => (maximized = value))
      .catch(() => undefined);
  }

  async function openPendingFiles() {
    try {
      const files = await api.openStartupFiles();
      for (const file of files) {
        // One unreadable file must not stop the rest of the batch.
        try {
          const opened = await api.openPath(file);
          await store.openNote(opened.noteId);
          if (opened.truncated) {
            store.toast("الملف كبير جداً — حُمّل جزء منه فقط وقد يُمنع الحفظ.", "error");
          }
        } catch (error) {
          store.toast(store.errorMessage(error), "error");
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
    if (ctrl && event.shiftKey && key === "t") {
      event.preventDefault();
      void store.setAlwaysOnTop(!$alwaysOnTop);
      return;
    }
    if (ctrl && event.shiftKey && key === "m") {
      event.preventDefault();
      void store.setCompact(!$compact);
      return;
    }
    if (event.key === "Escape" && $compact) {
      event.preventDefault();
      void store.setCompact(false);
      return;
    }
    if (ctrl && key === "b") {
      event.preventDefault();
      toggleSidebar();
      return;
    }
    if (ctrl && ["1", "2", "3", "4", "5", "6", "7"].includes(event.key) && !event.shiftKey) {
      event.preventDefault();
      void store.setView(
        (["notes", "tasks", "finance", "snippets", "board", "ai", "html"] as const)[
          Number(event.key) - 1
        ],
      );
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
    if (ctrl && event.shiftKey && key === "c") {
      event.preventDefault();
      calcOpen = true;
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
    if (ctrl && key === "e" && notesMode) {
      event.preventDefault();
      sendEditorCommand("toggle-preview");
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
      if (calcOpen) {
        calcOpen = false;
        return;
      }
      paletteOpen = false;
      helpOpen = false;
      if ($view !== "notes" && $view !== "trash") void store.setView("notes");
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

<div class="shell">
  {#if !$compact}
    <header
      class="titlebar"
      on:pointerdown={startWindowDrag}
      on:dblclick={() => void toggleMaximize()}
    >
      <span class="tb-logo">N</span>
      <span class="tb-title">{windowTitle}</span>
      <div class="tb-actions" on:pointerdown|stopPropagation>
        <button class="tb-btn" title="تصغير" aria-label="تصغير" on:click={minimizeWindow}>
          <Icon name="minimize" size={14} />
        </button>
        <button
          class="tb-btn"
          title={maximized ? "استعادة" : "تكبير"}
          aria-label={maximized ? "استعادة" : "تكبير"}
          on:click={() => void toggleMaximize()}
        >
          <Icon name={maximized ? "restore" : "maximize"} size={14} />
        </button>
        <button class="tb-btn close" title="إغلاق" aria-label="إغلاق" on:click={closeWindow}>
          <Icon name="close" size={14} />
        </button>
      </div>
    </header>
  {/if}

<div
  class="app"
  class:notes-mode={notesMode}
  class:compact={$compact}
  class:sidebar-collapsed={$sidebarPrefs.collapsed}
  style="--sidebar-w:{$sidebarPrefs.collapsed ? 64 : sidebarWidth}px; --list-w:{listWidth}px; --font-size:{fontSize}px; --font-family:{fontStack}"
>
  {#if !$compact}
    <Sidebar />
    {#if !$sidebarPrefs.collapsed}
      <div
        class="resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label="تغيير عرض الشريط الجانبي"
        class:dragging={resizing === "sidebar"}
        on:pointerdown={(event) => startResize("sidebar", event)}
      ></div>
    {:else}
      <div class="rail-gap"></div>
    {/if}
  {/if}

  {#if notesMode && $compact}
    <div class="compact-wrap">
      <div role="presentation" class="compact-bar" on:pointerdown={startWindowDrag}>
        <span>وضع الملاحظة البسيطة</span>
        <button
          title="رجوع للنافذة الكاملة (Esc)"
          on:pointerdown|stopPropagation
          on:click={() => void store.setCompact(false)}
        >
          ⤢ رجوع
        </button>
      </div>
      <Editor />
    </div>
  {:else if notesMode}
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
  {:else if $view === "snippets"}
    <SnippetsView />
  {:else if $view === "board"}
    <WhiteboardView />
  {:else if $view === "ai"}
    <AiView />
  {:else if $view === "html"}
    <HtmlView />
  {:else if $view === "settings"}
    <SettingsView />
  {/if}
</div>

  {#if !$compact}
    <!-- Frameless windows lose the native resize border, so the edges are
         hot zones that start the same native resize drag. -->
    <div role="presentation" class="rz n" on:pointerdown={() => startWindowResize("North")}></div>
    <div role="presentation" class="rz s" on:pointerdown={() => startWindowResize("South")}></div>
    <div role="presentation" class="rz e" on:pointerdown={() => startWindowResize("East")}></div>
    <div role="presentation" class="rz w" on:pointerdown={() => startWindowResize("West")}></div>
    <div role="presentation" class="rz ne" on:pointerdown={() => startWindowResize("NorthEast")}></div>
    <div role="presentation" class="rz nw" on:pointerdown={() => startWindowResize("NorthWest")}></div>
    <div role="presentation" class="rz se" on:pointerdown={() => startWindowResize("SouthEast")}></div>
    <div role="presentation" class="rz sw" on:pointerdown={() => startWindowResize("SouthWest")}></div>
  {/if}
</div>

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

{#if calcOpen}
  <CalculatorDialog on:close={() => (calcOpen = false)} />
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
    grid-template-rows: 100%;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
    font-family: var(--font-family);
    font-size: var(--font-size);
  }

  .app:not(.notes-mode) {
    grid-template-columns: var(--sidebar-w) 6px 0 0 1fr;
  }

  /* One row only: every pane stays beside the sidebar instead of wrapping below it. */
  .app > :global(*) {
    grid-row: 1;
    min-height: 0;
  }

  /* Small sticky-note mode: one column, only the open note. */
  .app.compact {
    grid-template-columns: 1fr;
  }

  .app.compact > :global(*) {
    grid-column: 1;
  }

  /* ---- Frameless shell: custom title bar + resize hot zones ---- */
  .shell {
    position: relative;
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--bg);
  }

  .shell > .app {
    flex: 1;
    height: auto;
    min-height: 0;
    grid-template-rows: minmax(0, 1fr);
  }

  .titlebar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 9px;
    height: 36px;
    padding: 0 12px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    user-select: none;
  }

  .tb-logo {
    width: 22px;
    height: 22px;
    border-radius: 6px;
    background: var(--accent);
    color: var(--accent-text);
    font-size: 0.72em;
    font-weight: 700;
    display: grid;
    place-items: center;
    flex-shrink: 0;
  }

  .tb-title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 0.8em;
    color: var(--text-muted);
  }

  .tb-actions {
    display: flex;
    align-items: stretch;
    align-self: stretch;
    margin-right: 8px;
  }

  .tb-btn {
    width: 44px;
    display: grid;
    place-items: center;
    border-radius: 0;
    color: var(--text-muted);
    background: transparent;
  }

  .tb-btn:hover {
    background: var(--bg-hover);
    color: var(--text);
  }

  .tb-btn.close:hover {
    background: #e81123;
    color: #fff;
  }

  .rz {
    position: absolute;
    z-index: 30;
  }

  .rz.n,
  .rz.s {
    left: 8px;
    right: 8px;
    height: 5px;
  }

  .rz.n {
    top: 0;
    cursor: ns-resize;
  }

  .rz.s {
    bottom: 0;
    cursor: ns-resize;
  }

  .rz.e,
  .rz.w {
    top: 8px;
    bottom: 8px;
    width: 5px;
  }

  .rz.e {
    right: 0;
    cursor: ew-resize;
  }

  .rz.w {
    left: 0;
    cursor: ew-resize;
  }

  .rz.ne,
  .rz.nw,
  .rz.se,
  .rz.sw {
    width: 10px;
    height: 10px;
  }

  .rz.ne {
    top: 0;
    right: 0;
    cursor: nesw-resize;
  }

  .rz.nw {
    top: 0;
    left: 0;
    cursor: nwse-resize;
  }

  .rz.se {
    bottom: 0;
    right: 0;
    cursor: nwse-resize;
  }

  .rz.sw {
    bottom: 0;
    left: 0;
    cursor: nesw-resize;
  }

  .compact-wrap {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .compact-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 3px 10px;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
    color: var(--text-dim);
    font-size: 0.74em;
    flex: 0 0 auto;
  }

  .compact-bar button {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-dim);
    border-radius: 999px;
    padding: 1px 10px;
    font-family: inherit;
    font-size: 1em;
    cursor: pointer;
  }

  .compact-bar button:hover {
    color: var(--text);
    border-color: var(--accent);
  }

  .compact-wrap > :global(.editor-pane) {
    flex: 1;
    min-height: 0;
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
