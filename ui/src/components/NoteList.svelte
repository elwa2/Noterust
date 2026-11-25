<script lang="ts">
  import * as store from "../lib/store";
  import {
    notes,
    activeFolder,
    activeTab,
    folders,
    searchQuery,
    sortBy,
    view,
    loading,
  } from "../lib/store";
  import { api } from "../lib/api";
  import { arabicDigits, dateGroup, listTime, NOUNS, plural } from "../lib/format";
  import Icon from "./Icon.svelte";
  import type { NoteSummary, SearchHit } from "../lib/types";

  $: currentFolder = store.findFolder($folders, $activeFolder);
  $: heading = $view === "trash" ? "المحذوفة مؤخراً" : (currentFolder?.name ?? "كل الملاحظات");
  $: grouped = groupNotes($notes);
  $: isSearching = $searchQuery.trim().length > 0;
  $: subtitle = isSearching
    ? `${plural($notes.length, NOUNS.note)} مطابقة`
    : plural($notes.length, NOUNS.note);

  function groupNotes(list: (NoteSummary | SearchHit)[]) {
    const groups: { label: string; items: (NoteSummary | SearchHit)[] }[] = [];
    for (const note of list) {
      const label = note.isPinned ? "مثبتة" : isSearching ? "نتائج البحث" : dateGroup(note.updatedAt);
      let bucket = groups.find((group) => group.label === label);
      if (!bucket) {
        bucket = { label, items: [] };
        groups.push(bucket);
      }
      bucket.items.push(note);
    }
    return groups;
  }

  function dragStart(event: DragEvent, note: NoteSummary) {
    event.dataTransfer?.setData("text/noterust-note", String(note.id));
    if (event.dataTransfer) event.dataTransfer.effectAllowed = "move";
  }

  async function togglePin(note: NoteSummary, event: MouseEvent) {
    event.stopPropagation();
    try {
      await api.pinNote(note.id, !note.isPinned);
      await store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function deleteForever(note: NoteSummary, event: MouseEvent) {
    event.stopPropagation();
    if (!confirm("حذف الملاحظة نهائياً؟")) return;
    try {
      await api.purgeNote(note.id);
      await store.removeNoteFromUi(note.id);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function restore(note: NoteSummary, event: MouseEvent) {
    event.stopPropagation();
    try {
      await api.restoreNote(note.id);
      await store.refreshAll();
      store.toast("تم الاسترجاع");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  let menu: { x: number; y: number; note: NoteSummary | SearchHit } | null = null;

  function openMenu(event: MouseEvent, note: NoteSummary | SearchHit) {
    event.preventDefault();
    event.stopPropagation();
    const width = 190;
    const height = 130;
    menu = {
      x: Math.min(event.clientX, window.innerWidth - width),
      y: Math.min(event.clientY, window.innerHeight - height),
      note,
    };
  }

  function closeMenu() {
    menu = null;
  }

  async function moveToTrash(note: NoteSummary | SearchHit, event?: MouseEvent) {
    event?.stopPropagation();
    closeMenu();
    try {
      await api.deleteNote(note.id);
      await store.removeNoteFromUi(note.id);
      store.toast("نُقلت إلى المحذوفة مؤقتاً", "ok", {
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

  async function emptyTrash() {
    if ($notes.length === 0) return;
    if (!confirm(`حذف ${plural($notes.length, NOUNS.note)} نهائياً؟ لا يمكن التراجع.`)) return;
    try {
      const removed = await api.emptyTrash();
      await store.refreshAll();
      store.toast(`حُذفت ${plural(removed, NOUNS.note)} نهائياً`);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  let searchTimer: ReturnType<typeof setTimeout>;

  function onSearchInput() {
    clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void store.refreshNotes(), 120);
  }
</script>

<section class="list-pane">
  <header class="head">
    <div class="head-row">
      <div class="titles">
        <h2>{heading}</h2>
        <span class="sub">{subtitle}</span>
      </div>
      <span class="spacer"></span>
      {#if $view === "trash" && $notes.length > 0}
        <button class="btn small danger" on:click={emptyTrash} title="حذف كل المحذوفات نهائياً">
          إفراغ
        </button>
      {/if}
      <button class="btn primary new" on:click={() => store.createNote()} title="Ctrl+N">
        <Icon name="plus" size={15} /> ملاحظة
      </button>
      <button
        class="icon-btn"
        title="إخفاء قائمة الملاحظات (Ctrl+Shift+L)"
        aria-label="إخفاء قائمة الملاحظات"
        on:click={() => store.toggleListHidden()}
      >
        <Icon name="sidebar" size={15} />
      </button>
    </div>

    <div class="controls">
      <div class="search">
        <span class="search-icon"><Icon name="search" size={15} /></span>
        <input
          id="global-search"
          placeholder="ابحث في الملاحظات…"
          bind:value={$searchQuery}
          on:input={onSearchInput}
          dir="auto"
        />
        {#if $searchQuery}
          <button
            class="icon-btn small"
            title="مسح البحث"
            on:click={() => {
              $searchQuery = "";
              void store.refreshNotes();
            }}
          >
            <Icon name="close" size={13} />
          </button>
        {/if}
      </div>
      <select bind:value={$sortBy} on:change={() => store.refreshNotes()} title="ترتيب">
        <option value="updated">التعديل</option>
        <option value="created">الإنشاء</option>
        <option value="title">العنوان</option>
      </select>
    </div>
  </header>

  <div class="scroll items">
    {#if $loading && $notes.length === 0}
      <p class="empty">جارٍ التحميل…</p>
    {:else if $notes.length === 0}
      <p class="empty">
        {#if isSearching}
          لا توجد نتائج لـ «{$searchQuery}»
        {:else if $view === "trash"}
          سلة المحذوفات فارغة
        {:else}
          لا ملاحظات هنا بعد — أنشئ واحدة.
        {/if}
      </p>
    {/if}

    {#each grouped as group (group.label)}
      <div class="group-label">{group.label}</div>
      {#each group.items as note (note.id)}
        <article
          class="note-item"
          class:active={$activeTab === note.id}
          style={note.color ? `border-inline-start-color:${note.color}` : ""}
          draggable="true"
          on:dragstart={(event) => dragStart(event, note)}
          on:click={() => store.openNote(note.id)}
          on:keydown={(event) => event.key === "Enter" && store.openNote(note.id)}
          on:contextmenu={(event) => openMenu(event, note)}
          role="button"
          tabindex="0"
        >
          <div class="item-top">
            {#if note.isPinned}<span class="pin" title="مثبتة"><Icon name="pin" size={12} /></span>{/if}
            {#if note.isLocked}<span class="pin" title="مقفلة"><Icon name="lock" size={12} /></span>{/if}
            <span class="title">{note.title || "بدون عنوان"}</span>
            <span class="spacer"></span>
            <span class="row-actions">
              {#if $view !== "trash"}
                <button
                  class="icon-btn tiny"
                  class:on={note.isPinned}
                  on:click={(event) => togglePin(note, event)}
                  title={note.isPinned ? "إلغاء التثبيت" : "تثبيت"}
                >
                  <Icon name="pin" size={13} />
                </button>
              {/if}
              <button
                class="icon-btn tiny danger"
                on:click={(event) => ($view === "trash" ? deleteForever(note, event) : moveToTrash(note, event))}
                title={$view === "trash" ? "حذف نهائي" : "نقل إلى المحذوفة"}
              >
                <Icon name="trash" size={13} />
              </button>
            </span>
          </div>

          <p class="preview">
            {"snippet" in note && isSearching && note.snippet
              ? note.snippet
              : note.preview || "لا نص بعد…"}
          </p>

          <div class="item-foot">
            <span class="time">{listTime(note.updatedAt)}</span>
            {#if note.tags.length > 0}
              <span class="tags">
                {#each note.tags.slice(0, 3) as tag (tag.id)}
                  <span class="tag" style="color:{tag.color}">#{tag.name}</span>
                {/each}
              </span>
            {/if}
            <span class="spacer"></span>
            {#if $view === "trash"}
              <button class="link" on:click={(event) => restore(note, event)}>استرجاع</button>
              <button class="link danger" on:click={(event) => deleteForever(note, event)}>حذف</button>
            {:else}
              <span class="words">{arabicDigits(note.wordCount)} كلمة</span>
            {/if}
          </div>
        </article>
      {/each}
    {/each}
  </div>

  {#if menu}
    <div
      class="ctx-layer"
      role="presentation"
      on:click={closeMenu}
      on:contextmenu|preventDefault={closeMenu}
    ></div>
    <div class="ctx-menu" style="left:{menu.x}px; top:{menu.y}px" role="menu" tabindex="-1">
      {#if $view === "trash"}
        <button class="ctx-item" on:click={(event) => restore(menu!.note, event)}>استرجاع</button>
        <button class="ctx-item danger" on:click={(event) => deleteForever(menu!.note, event)}>
          حذف نهائياً
        </button>
      {:else}
        <button class="ctx-item" on:click={() => { const id = menu!.note.id; closeMenu(); void store.openNote(id); }}>
          فتح
        </button>
        <button class="ctx-item" on:click={(event) => togglePin(menu!.note, event)}>
          {menu!.note.isPinned ? "إلغاء التثبيت" : "تثبيت"}
        </button>
        <button class="ctx-item danger" on:click={(event) => moveToTrash(menu!.note, event)}>
          نقل إلى المحذوفة
        </button>
      {/if}
    </div>
  {/if}
</section>

<style>
  .list-pane {
    background: var(--bg);
    border-inline-end: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .head {
    padding: 14px 14px 10px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .head-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .titles {
    min-width: 0;
  }

  h2 {
    margin: 0;
    font-size: 1.02em;
    font-weight: 700;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .sub {
    color: var(--text-faint);
    font-size: 0.72em;
  }

  .new {
    font-size: 0.82em;
    padding: 6px 10px;
  }

  .btn.small {
    font-size: 0.78em;
    padding: 5px 9px;
  }

  .controls {
    display: flex;
    gap: 8px;
  }

  .search {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 6px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 0 8px;
  }

  .search:focus-within {
    border-color: var(--accent);
  }

  .search input {
    flex: 1;
    border: none;
    background: none;
    padding: 6px 0;
    min-width: 0;
  }

  .search-icon {
    color: var(--text-faint);
    display: grid;
    place-items: center;
  }

  .icon-btn.small {
    width: 22px;
    height: 22px;
  }

  .items {
    flex: 1;
    padding: 8px;
  }

  .group-label {
    color: var(--text-faint);
    font-size: 0.72em;
    padding: 10px 8px 4px;
  }

  .note-item {
    background: var(--bg-elevated);
    border: 1px solid transparent;
    border-inline-start: 3px solid transparent;
    border-radius: var(--radius);
    padding: 9px 11px;
    margin-bottom: 6px;
    cursor: pointer;
    transition: background 120ms ease, border-color 120ms ease;
  }

  .note-item:hover {
    background: var(--bg-hover);
  }

  .note-item.active {
    border-color: var(--border-strong);
    border-inline-start-color: var(--accent);
    background: var(--bg-raised);
  }

  .item-top {
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .title {
    font-weight: 600;
    font-size: 0.93em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .pin {
    color: var(--text-faint);
    display: grid;
    place-items: center;
  }

  .row-actions {
    display: flex;
    gap: 1px;
    opacity: 0;
    transition: opacity 120ms ease;
  }

  .note-item:hover .row-actions,
  .note-item.active .row-actions,
  .note-item:focus-within .row-actions {
    opacity: 1;
  }

  .icon-btn.tiny {
    width: 22px;
    height: 22px;
  }

  .icon-btn.tiny.on {
    color: var(--accent);
  }

  .icon-btn.tiny.danger:hover {
    color: var(--danger);
  }

  .ctx-layer {
    position: fixed;
    inset: 0;
    z-index: 40;
  }

  .ctx-menu {
    position: fixed;
    z-index: 41;
    min-width: 178px;
    padding: 5px;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: 0 16px 44px rgba(0, 0, 0, 0.45);
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .ctx-item {
    text-align: start;
    padding: 7px 11px;
    border-radius: 6px;
    font-size: 0.84em;
    color: var(--text);
  }

  .ctx-item:hover {
    background: var(--bg-hover);
  }

  .ctx-item.danger {
    color: var(--danger);
  }

  .preview {
    margin: 3px 0 5px;
    color: var(--text-muted);
    font-size: 0.8em;
    line-height: 1.5;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .item-foot {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.72em;
    color: var(--text-faint);
  }

  .tags {
    display: flex;
    gap: 6px;
  }

  .tag {
    font-size: 0.95em;
  }

  .link {
    color: var(--accent);
    font-size: 1em;
  }

  .link.danger {
    color: var(--danger);
  }

  .empty {
    text-align: center;
    color: var(--text-faint);
    padding: 40px 16px;
    font-size: 0.86em;
  }
</style>
