<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import { arabicDigits, plural, NOUNS, relativeDay } from "../lib/format";
  import Icon from "./Icon.svelte";
  import BackButton from "./BackButton.svelte";
  import type { Snippet } from "../lib/types";

  type Kind = "code" | "prompt";

  let items: Snippet[] = [];
  let loading = true;
  let kindFilter: "" | Kind = "";
  let query = "";

  let active: Snippet | null = null;
  let isNew = false;
  let form = { title: "", kind: "code" as Kind, language: "", tags: "", content: "" };
  let saving = false;
  let dirty = false;
  let error = "";
  let searchTimer: ReturnType<typeof setTimeout> | null = null;

  $: codeCount = items.filter((item) => item.kind === "code").length;
  $: promptCount = items.filter((item) => item.kind === "prompt").length;
  $: summary = plural(items.length, NOUNS.snippet);

  onMount(() => void load());

  async function load() {
    loading = true;
    try {
      items = await api.listSnippets(query.trim() || undefined, kindFilter || undefined);
      if (active && !items.some((item) => item.id === active?.id)) {
        clearSelection();
      }
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    } finally {
      loading = false;
    }
  }

  function searchSoon() {
    if (searchTimer) clearTimeout(searchTimer);
    searchTimer = setTimeout(() => void load(), 220);
  }

  function setKindFilter(next: "" | Kind) {
    kindFilter = next;
    void load();
  }

  function preview(item: Snippet): string {
    return item.content.split("\n").find((line) => line.trim())?.trim().slice(0, 90) ?? "";
  }

  function clearSelection() {
    active = null;
    isNew = false;
    dirty = false;
    error = "";
    form = { title: "", kind: "code", language: "", tags: "", content: "" };
  }

  async function select(item: Snippet) {
    if (dirty) await save();
    active = item;
    isNew = false;
    dirty = false;
    error = "";
    form = {
      title: item.title,
      kind: item.kind,
      language: item.language,
      tags: item.tags,
      content: item.content,
    };
  }

  async function startNew() {
    if (dirty) await save();
    clearSelection();
    isNew = true;
  }

  function markDirty() {
    dirty = true;
  }

  async function save() {
    if (!dirty && active) return;
    saving = true;
    error = "";
    try {
      const draft = {
        title: form.title.trim(),
        kind: form.kind,
        language: form.language.trim(),
        tags: form.tags.trim(),
        content: form.content,
      };
      const creating = isNew || !active;
      const saved = creating
        ? await api.createSnippet(draft)
        : await api.updateSnippet(active!.id, draft);
      dirty = false;
      isNew = false;
      active = saved;
      await load();
      store.toast(creating ? "تم حفظ العنصر الجديد" : "تم حفظ التعديلات");
    } catch (err) {
      error = store.errorMessage(err);
    } finally {
      saving = false;
    }
  }

  async function copy(item: Snippet | null = active) {
    if (!item) return;
    try {
      await navigator.clipboard.writeText(item.content);
      store.toast("تم نسخ المحتوى");
    } catch {
      store.toast("تعذر النسخ من المتصفح", "error");
    }
  }

  async function togglePin(item: Snippet) {
    try {
      await api.updateSnippet(item.id, { pinned: !item.isPinned });
      await load();
      if (active?.id === item.id) active = { ...active, isPinned: !item.isPinned };
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  async function remove(item: Snippet) {
    if (!confirm(`حذف «${item.title || "بدون عنوان"}»؟`)) return;
    try {
      await api.deleteSnippet(item.id);
      if (active?.id === item.id) clearSelection();
      await load();
      store.toast("تم الحذف");
    } catch (err) {
      store.toast(store.errorMessage(err), "error");
    }
  }

  const kindLabels: Record<Kind, string> = { code: "كود", prompt: "برومبت" };
</script>

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>الأكواد والبرومبت</h1>
      <p class="muted">احفظ المقاطع الجاهزة والبرومبتات، وانسخها بضغطة واحدة.</p>
    </div>
    <span class="badge">{summary}</span>
  </header>

  <div class="scroll content">
    <div class="toolbar">
      <button class="btn primary" on:click={startNew}>
        <Icon name="plus" size={14} /> جديد
      </button>
      <div class="kind-chips">
        <button class="chip" class:on={kindFilter === ""} on:click={() => setKindFilter("")}>
          الكل <span class="count">{arabicDigits(codeCount + promptCount)}</span>
        </button>
        <button class="chip" class:on={kindFilter === "code"} on:click={() => setKindFilter("code")}>
          أكواد <span class="count">{arabicDigits(codeCount)}</span>
        </button>
        <button
          class="chip"
          class:on={kindFilter === "prompt"}
          on:click={() => setKindFilter("prompt")}
        >
          برومبت <span class="count">{arabicDigits(promptCount)}</span>
        </button>
      </div>
      <input
        class="search"
        placeholder="ابحث في العنوان أو المحتوى أو الوسوم…"
        bind:value={query}
        on:input={searchSoon}
      />
    </div>

    <div class="split">
      <aside class="list card">
        {#if loading && items.length === 0}
          <p class="faint pad">جارٍ التحميل…</p>
        {:else if items.length === 0}
          <p class="empty">
            لا يوجد شيء محفوظ بعد.<br />
            اضغط «جديد» لحفظ أول كود أو برومبت.
          </p>
        {:else}
          {#each items as item (item.id)}
            <button
              class="snippet"
              class:active={active?.id === item.id}
              on:click={() => select(item)}
            >
              <div class="snippet-head">
                <span class="kind" class:prompt={item.kind === "prompt"}>
                  <Icon name={item.kind === "prompt" ? "prompt" : "code"} size={12} />
                  {kindLabels[item.kind]}
                </span>
                {#if item.language}<span class="lang">{item.language}</span>{/if}
                <span class="spacer"></span>
                {#if item.isPinned}<span class="pin">📌</span>{/if}
              </div>
              <div class="snippet-title">{item.title || "بدون عنوان"}</div>
              <p class="snippet-preview">{preview(item)}</p>
              <div class="snippet-meta">
                <span>{relativeDay(item.updatedAt)}</span>
                {#if item.tags}<span class="tags">{item.tags}</span>{/if}
              </div>
            </button>
          {/each}
        {/if}
      </aside>

      <div class="editor card">
        {#if !active && !isNew}
          <div class="placeholder">
            <Icon name="code" size={30} />
            <p>اختر عنصراً من القائمة أو أضف واحداً جديداً.</p>
          </div>
        {:else}
          <div class="row">
            <input
              class="grow title-input"
              placeholder="العنوان"
              bind:value={form.title}
              on:input={markDirty}
            />
            <select bind:value={form.kind} on:change={markDirty} title="النوع">
              <option value="code">كود</option>
              <option value="prompt">برومبت</option>
            </select>
          </div>
          <div class="row">
            <input
              class="lang-input"
              placeholder="اللغة (ts, python, sql…)"
              bind:value={form.language}
              on:input={markDirty}
            />
            <input
              class="grow"
              placeholder="وسوم مفصولة بفاصلة"
              bind:value={form.tags}
              on:input={markDirty}
            />
          </div>
          <textarea
            class="content-input"
            class:code={form.kind === "code"}
            dir={form.kind === "code" ? "ltr" : "auto"}
            spellcheck="false"
            placeholder={form.kind === "code" ? "// اكتب الكود هنا" : "اكتب البرومبت هنا…"}
            bind:value={form.content}
            on:input={markDirty}
          ></textarea>

          {#if error}<p class="error">{error}</p>{/if}

          <div class="actions">
            <button class="btn primary" on:click={save} disabled={saving}>
              <Icon name="check" size={14} /> حفظ
            </button>
            <button class="btn" on:click={() => copy()} disabled={!active}>
              <Icon name="copy" size={14} /> نسخ
            </button>
            {#if active}
              <button class="btn ghost" on:click={() => togglePin(active!)}>
                <Icon name="pin" size={14} /> {active.isPinned ? "إلغاء التثبيت" : "تثبيت"}
              </button>
              <span class="spacer"></span>
              <button class="btn ghost danger" on:click={() => remove(active!)}>
                <Icon name="trash" size={14} /> حذف
              </button>
            {:else}
              <span class="spacer"></span>
            {/if}
            <span class="faint meta">
              {#if dirty}تغييرات غير محفوظة{:else if active}حُفظ {relativeDay(active.updatedAt)}{/if}
            </span>
          </div>
        {/if}
      </div>
    </div>
  </div>
</section>

<style>
  .view-page {
    grid-column: 2 / -1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
  }

  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 20px 26px 14px;
    border-bottom: 1px solid var(--border);
  }

  h1 {
    margin: 0;
    font-size: 1.28em;
  }

  .page-head p {
    margin: 2px 0 0;
    font-size: 0.82em;
  }

  .badge {
    margin-inline-start: auto;
    font-size: 0.76em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 11px;
  }

  .content {
    flex: 1;
    padding: 16px 26px 40px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 12px;
    min-height: 0;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .kind-chips {
    display: flex;
    gap: 6px;
  }

  .chip {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 5px 12px;
    font-size: 0.8em;
    color: var(--text-muted);
    background: var(--bg-elevated);
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
    background: var(--accent-soft);
  }

  .chip .count {
    font-size: 0.85em;
    opacity: 0.75;
  }

  .search {
    flex: 1;
    min-width: 200px;
  }

  .split {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 330px 1fr;
    gap: 12px;
  }

  .card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }

  .list {
    overflow-y: auto;
    padding: 6px;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .pad {
    padding: 12px;
  }

  .snippet {
    text-align: right;
    padding: 9px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid transparent;
    display: flex;
    flex-direction: column;
    gap: 4px;
    width: 100%;
  }

  .snippet:hover {
    background: var(--bg-hover);
  }

  .snippet.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 40%, transparent);
  }

  .snippet-head {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 0.7em;
    color: var(--text-faint);
  }

  .kind {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    color: var(--accent);
  }

  .kind.prompt {
    color: var(--success);
  }

  .lang {
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 0 6px;
  }

  .spacer {
    flex: 1;
  }

  .snippet-title {
    font-size: 0.9em;
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .snippet-preview {
    margin: 0;
    font-size: 0.75em;
    color: var(--text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: ltr;
    text-align: right;
  }

  .snippet-meta {
    display: flex;
    gap: 8px;
    font-size: 0.7em;
    color: var(--text-faint);
  }

  .tags {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .editor {
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-height: 0;
  }

  .row {
    display: flex;
    gap: 8px;
    align-items: center;
  }

  .grow {
    flex: 1;
    min-width: 120px;
  }

  .title-input {
    font-weight: 600;
  }

  .lang-input {
    width: 180px;
  }

  .content-input {
    flex: 1;
    min-height: 260px;
    resize: none;
    background: var(--bg);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 12px;
    font-family: var(--font-family);
    font-size: 0.92em;
    line-height: 1.7;
  }

  .content-input.code {
    font-family: "Cascadia Code", "Consolas", "Courier New", monospace;
    font-size: 0.85em;
    line-height: 1.6;
    tab-size: 2;
  }

  .actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .actions .danger {
    color: var(--danger);
  }

  .meta {
    font-size: 0.72em;
  }

  .placeholder {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--text-faint);
    font-size: 0.85em;
  }

  .empty {
    padding: 30px 14px;
    text-align: center;
    color: var(--text-faint);
    font-size: 0.82em;
    line-height: 1.8;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.8em;
  }
</style>
