<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { toast } from "../lib/store";
  import { api } from "../lib/api";
  import { arabicDigits } from "../lib/format";
  import { previewFrameUrl } from "../lib/preview";
  import Icon from "./Icon.svelte";
  import BackButton from "./BackButton.svelte";

  const STORAGE_KEY = "noterust.html.playground";

  const EXAMPLE = `<!doctype html>
<html lang="ar" dir="rtl">
<head>
  <meta charset="utf-8" />
  <title>تجربة</title>
  <style>
    body { font-family: "Segoe UI", Tahoma, sans-serif; background: #101014; color: #eee;
           display: grid; place-items: center; min-height: 100vh; margin: 0; }
    .card { padding: 28px 34px; border-radius: 16px; background: #1b1b22;
            border: 1px solid #2c2c38; text-align: center; }
    button { margin-top: 14px; padding: 9px 18px; border-radius: 10px; border: none;
             background: #6965db; color: #fff; font-family: inherit; cursor: pointer; }
  </style>
</head>
<body>
  <div class="card">
    <h1>مرحباً 👋</h1>
    <p>عدّل الكود ثم اضغط «تشغيل» أو Ctrl+Enter.</p>
    <button onclick="this.textContent='تم النقر ✓'">اضغطني</button>
  </div>
</body>
</html>`;

  let code = EXAMPLE;
  let preview = "";
  /** Same-origin-free URL the preview frame loads each run. */
  let previewUrl = "";
  // Windows serves custom schemes as http://<scheme>.localhost.
  const previewOrigin = navigator.userAgent.includes("Windows")
    ? "http://htmlpreview.localhost"
    : "htmlpreview://localhost";
  let autoRun = false;
  let fileInput: HTMLInputElement | null = null;
  let timer: ReturnType<typeof setTimeout> | null = null;
  /** Autosave bookkeeping (debounced, mirrored to disk). */
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let saving = false;
  let savedAt: number | null = null;
  let ready = false;
  let saveQueue: Promise<void> = Promise.resolve();

  onMount(() => {
    void (async () => {
      try {
        const saved = await api.loadHtmlLab();
        if (saved !== null) {
          code = saved;
          savedAt = Date.now();
        } else {
          const local = localStorage.getItem(STORAGE_KEY);
          if (local) code = local;
        }
      } catch {
        const local = localStorage.getItem(STORAGE_KEY);
        if (local) code = local;
      }
      ready = true;
      void refreshPreview();
    })();
    return () => {
      if (timer) clearTimeout(timer);
    };
  });

  onDestroy(() => {
    if (saveTimer) clearTimeout(saveTimer);
    try {
      localStorage.setItem(STORAGE_KEY, code);
    } catch {
      /* ignore */
    }
    void api.saveHtmlLab(code).catch(() => undefined);
  });

  // Every edit (typing, paste, open file, example, clear) is saved automatically.
  $: if (ready) scheduleAutoSave(code);

  function scheduleAutoSave(value: string) {
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => persistCode(value), 600);
  }

  /** localStorage for speed and the document on disk for durability. */
  function persistCode(value: string) {
    try {
      localStorage.setItem(STORAGE_KEY, value);
    } catch {
      /* storage full — keep the code in memory */
    }
    saving = true;
    saveQueue = saveQueue.then(async () => {
      try {
        await api.saveHtmlLab(value);
        savedAt = Date.now();
      } catch (error) {
        toast(error instanceof Error ? error.message : String(error), "error");
      } finally {
        saving = false;
      }
    });
  }

  /**
   * The preview lives on the `htmlpreview://` origin so the app's CSP is not
   * inherited (it would block the page's scripts, web fonts and remote images).
   */
  async function refreshPreview() {
    try {
      await api.setHtmlPreview(code);
      preview = code;
      previewUrl = previewFrameUrl(code, previewOrigin);
    } catch (error) {
      toast(error instanceof Error ? error.message : String(error), "error");
    }
  }

  function run() {
    persistCode(code);
    void refreshPreview().then(() => toast("تم تشغيل الكود"));
  }

  function onInput() {
    if (!autoRun) return;
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => {
      void refreshPreview();
    }, 500);
  }

  function download() {
    const blob = new Blob([code], { type: "text/html;charset=utf-8" });
    const link = document.createElement("a");
    link.download = "noterust-page.html";
    link.href = URL.createObjectURL(blob);
    link.click();
    setTimeout(() => URL.revokeObjectURL(link.href), 4000);
  }

  function openFile(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    const reader = new FileReader();
    reader.onload = () => {
      code = String(reader.result ?? "");
      void refreshPreview();
      try {
        localStorage.setItem(STORAGE_KEY, code);
      } catch {
        /* ignore */
      }
      toast("تم تحميل الملف");
    };
    reader.readAsText(file);
  }

  function clearCode() {
    if (!confirm("مسح الكود الحالي؟")) return;
    code = "";
    preview = "";
    previewUrl = "";
    void api.setHtmlPreview("").catch(() => undefined);
    localStorage.removeItem(STORAGE_KEY);
  }

  function resetExample() {
    code = EXAMPLE;
    void refreshPreview();
  }

  function formatSize(value: string): string {
    return `${arabicDigits(value.length)} حرفاً`;
  }
</script>

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>معمل HTML</h1>
      <p class="muted">اكتب أو الصق كود HTML وشغّله فوراً داخل التطبيق.</p>
    </div>
    <span class="badge">{formatSize(code)}</span>
    <span class="save-state" title="يُحفظ كل تعديل تلقائياً على القرص">
      {#if saving}جارٍ الحفظ…{:else if savedAt}محفوظ تلقائياً{:else}—{/if}
    </span>
    <button class="btn primary" on:click={run}><Icon name="exportIcon" size={14} /> تشغيل</button>
  </header>

  <div class="toolbar">
    <label class="toggle">
      <input type="checkbox" bind:checked={autoRun} /> تحديث تلقائي أثناء الكتابة
    </label>
    <span class="spacer"></span>
    <button class="btn small" on:click={download}>تنزيل .html</button>
    <button class="btn small" on:click={() => fileInput?.click()}>فتح ملف</button>
    <button class="btn small ghost" on:click={resetExample}>مثال</button>
    <button class="btn small ghost danger" on:click={clearCode}>مسح</button>
    <input class="hidden-input" type="file" accept=".html,.htm,text/html" bind:this={fileInput} on:change={openFile} />
  </div>

  <div class="workspace">
    <div class="pane editor-pane">
      <div class="pane-head">
        <span>الكود</span>
        <span class="muted tiny">Ctrl+Enter للتشغيل</span>
      </div>
      <textarea
        class="code"
        dir="ltr"
        spellcheck="false"
        wrap="off"
        bind:value={code}
        on:input={onInput}
        on:keydown={(event) => {
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            event.preventDefault();
            run();
          }
          if (event.key === "Tab") {
            event.preventDefault();
            const target = event.currentTarget as HTMLTextAreaElement;
            const start = target.selectionStart;
            const end = target.selectionEnd;
            code = `${code.slice(0, start)}  ${code.slice(end)}`;
            void Promise.resolve().then(() => {
              target.selectionStart = start + 2;
              target.selectionEnd = start + 2;
            });
          }
        }}
      ></textarea>
    </div>

    <div class="pane preview-pane">
      <div class="pane-head">
        <span>المعاينة</span>
        <span class="muted tiny">تعمل في إطار معزول</span>
      </div>
      {#if preview.trim() && previewUrl}
        <iframe
          title="معاينة HTML"
          sandbox="allow-scripts allow-same-origin allow-modals allow-forms allow-popups allow-downloads"
          src={previewUrl}
        ></iframe>
      {:else}
        <div class="empty">لا يوجد كود لتشغيله — اكتب HTML ثم اضغط «تشغيل».</div>
      {/if}
    </div>
  </div>
</section>

<style>
  .view-page {
    grid-column: 3 / -1;
    grid-row: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    overflow: hidden;
    background: var(--bg);
  }

  .page-head {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 16px 26px 12px;
    border-bottom: 1px solid var(--border);
  }

  .page-head > div {
    flex: 1;
    min-width: 0;
  }

  .page-head :global(.btn) {
    white-space: nowrap;
    flex-shrink: 0;
  }

  h1 {
    margin: 0;
    font-size: 1.24em;
  }

  .page-head p {
    margin: 2px 0 0;
    font-size: 0.8em;
  }

  .badge {
    font-size: 0.72em;
    color: var(--text-muted);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 3px 10px;
  }

  .save-state {
    font-size: 0.72em;
    color: var(--text-faint);
    min-width: 84px;
    text-align: center;
    user-select: none;
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
    padding: 8px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .toggle {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    font-size: 0.78em;
    color: var(--text-muted);
    cursor: pointer;
  }

  .spacer {
    flex: 1;
  }

  .hidden-input {
    display: none;
  }

  .workspace {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
    gap: 1px;
    background: var(--border);
  }

  .pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    background: var(--bg);
  }

  .pane-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 7px 14px;
    font-size: 0.76em;
    color: var(--text-muted);
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
  }

  .tiny {
    font-size: 0.92em;
  }

  .code {
    flex: 1;
    width: 100%;
    min-height: 0;
    border: none;
    outline: none;
    resize: none;
    padding: 14px 16px;
    background: var(--bg);
    color: var(--text);
    font-family: "Cascadia Code", Consolas, "Courier New", monospace;
    font-size: 0.86em;
    line-height: 1.65;
    white-space: pre;
    overflow: auto;
    tab-size: 2;
  }

  iframe {
    flex: 1;
    min-height: 0;
    width: 100%;
    border: none;
    background: #fff;
  }

  .empty {
    flex: 1;
    display: grid;
    place-items: center;
    color: var(--text-faint);
    font-size: 0.86em;
    padding: 20px;
    text-align: center;
  }

  @media (max-width: 980px) {
    .workspace {
      grid-template-columns: 1fr;
      grid-template-rows: minmax(0, 1fr) minmax(0, 1fr);
    }
  }
</style>
