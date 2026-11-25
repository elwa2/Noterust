<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import type { AiConnection, AiConnectionDraft } from "../lib/types";
  import { toast } from "../lib/store";
  import { arabicDigits } from "../lib/format";
  import Icon from "./Icon.svelte";
  import BackButton from "./BackButton.svelte";

  interface Preset {
    id: string;
    label: string;
    hint: string;
    kind: AiConnection["kind"];
    baseUrl: string;
    model: string;
    needsKey: boolean;
  }

  const PRESETS: Preset[] = [
    {
      id: "ollama",
      label: "Ollama",
      hint: "خادم محلي — يعمل بدون مفتاح",
      kind: "openai",
      baseUrl: "http://127.0.0.1:11434/v1",
      model: "llama3.2",
      needsKey: false,
    },
    {
      id: "lmstudio",
      label: "LM Studio",
      hint: "خادم محلي متوافق مع OpenAI",
      kind: "openai",
      baseUrl: "http://127.0.0.1:1234/v1",
      model: "local-model",
      needsKey: false,
    },
    {
      id: "custom",
      label: "خادم محلي آخر (vLLM / FreeLM)",
      hint: "أي خادم متوافق مع OpenAI",
      kind: "openai",
      baseUrl: "http://127.0.0.1:8000/v1",
      model: "",
      needsKey: false,
    },
    {
      id: "openai",
      label: "OpenAI",
      hint: "يحتاج مفتاح API",
      kind: "openai",
      baseUrl: "https://api.openai.com/v1",
      model: "gpt-4o-mini",
      needsKey: true,
    },
    {
      id: "openrouter",
      label: "OpenRouter",
      hint: "موديلات كثيرة بمفتاح واحد",
      kind: "openai",
      baseUrl: "https://openrouter.ai/api/v1",
      model: "openai/gpt-4o-mini",
      needsKey: true,
    },
    {
      id: "gemini",
      label: "Google Gemini",
      hint: "مفتاح من Google AI Studio",
      kind: "gemini",
      baseUrl: "",
      model: "gemini-2.0-flash",
      needsKey: true,
    },
    {
      id: "anthropic",
      label: "Anthropic",
      hint: "مفتاح Claude API",
      kind: "anthropic",
      baseUrl: "",
      model: "claude-sonnet-4-5",
      needsKey: true,
    },
  ];

  let connections: AiConnection[] = [];
  let loading = true;
  let error = "";

  let dialogOpen = false;
  let editing: number | null = null;
  let preset = PRESETS[0];
  let form: AiConnectionDraft = draftFrom(PRESETS[0]);
  let saving = false;

  let testingId: number | null = null;
  let deletingId: number | null = null;

  let selectedId: number | null = null;
  let prompt = "";
  let answer = "";
  let asking = false;
  let askError = "";

  function draftFrom(entry: Preset): AiConnectionDraft {
    return { id: null, name: entry.label, kind: entry.kind, baseUrl: entry.baseUrl, apiKey: "", model: entry.model };
  }

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = "";
    try {
      connections = await api.aiConnections();
      if (selectedId === null && connections.length > 0) selectedId = connections[0].id;
    } catch (err) {
      error = (err as Error).message;
    } finally {
      loading = false;
    }
  }

  function openAdd() {
    editing = null;
    const first = PRESETS[0];
    preset = first;
    form = draftFrom(first);
    dialogOpen = true;
  }

  function openEdit(connection: AiConnection) {
    editing = connection.id;
    preset = PRESETS.find((entry) => entry.kind === connection.kind && entry.baseUrl === connection.baseUrl) ??
      PRESETS.find((entry) => entry.kind === connection.kind) ?? PRESETS[0];
    form = {
      id: connection.id,
      name: connection.name,
      kind: connection.kind,
      baseUrl: connection.baseUrl,
      apiKey: connection.apiKey,
      model: connection.model,
    };
    dialogOpen = true;
  }

  function choosePreset(id: string) {
    const found = PRESETS.find((entry) => entry.id === id) ?? PRESETS[0];
    preset = found;
    form = {
      ...draftFrom(found),
      id: editing,
      name: form.name && editing !== null ? form.name : found.label,
      apiKey: form.apiKey,
    };
  }

  async function save() {
    if (saving) return;
    if (form.kind === "openai" && !form.baseUrl.trim()) {
      toast("اكتب عنوان الخادم مثل http://127.0.0.1:11434/v1", "error");
      return;
    }
    saving = true;
    try {
      const saved = await api.aiSaveConnection({ ...form, id: editing });
      dialogOpen = false;
      await load();
      selectedId = saved.id;
      toast("تم حفظ الاتصال");
    } catch (err) {
      toast((err as Error).message, "error");
    } finally {
      saving = false;
    }
  }

  async function test(connection: AiConnection) {
    testingId = connection.id;
    try {
      const message = await api.aiTestConnection(connection.id);
      toast(message);
    } catch (err) {
      toast((err as Error).message, "error");
    } finally {
      testingId = null;
    }
  }

  async function remove(connection: AiConnection) {
    if (!confirm(`حذف اتصال «${connection.name}»؟`)) return;
    deletingId = connection.id;
    try {
      await api.aiDeleteConnection(connection.id);
      if (selectedId === connection.id) selectedId = null;
      await load();
      toast("تم حذف الاتصال");
    } catch (err) {
      toast((err as Error).message, "error");
    } finally {
      deletingId = null;
    }
  }

  async function ask() {
    if (selectedId === null) {
      askError = "أضف اتصالاً أولاً";
      return;
    }
    if (!prompt.trim()) return;
    asking = true;
    askError = "";
    answer = "";
    try {
      answer = await api.aiChat(selectedId, prompt);
    } catch (err) {
      askError = (err as Error).message;
    } finally {
      asking = false;
    }
  }

  async function copyAnswer() {
    try {
      await navigator.clipboard.writeText(answer);
      toast("تم نسخ الرد");
    } catch {
      toast("تعذر النسخ", "error");
    }
  }

  function onDialogKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") dialogOpen = false;
  }

  $: activeConnection = connections.find((connection) => connection.id === selectedId) ?? null;
</script>

<svelte:window on:keydown={onDialogKeydown} />

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>الذكاء الاصطناعي</h1>
      <p class="muted">اربط NoteRust بخوادم محلية (Ollama، LM Studio) أو خدمات سحابية واستخدمها من هنا.</p>
    </div>
    <button class="btn primary" on:click={openAdd}><Icon name="plus" size={14} /> إضافة اتصال</button>
  </header>

  <div class="content scroll">
    {#if error}
      <p class="error">{error}</p>
    {/if}

    <section class="block">
      <div class="block-head">
        <h2>الاتصالات</h2>
        <span class="muted small">
          {connections.length === 0 ? "لا يوجد اتصال بعد" : `${arabicDigits(connections.length)} اتصال`}
        </span>
      </div>

      {#if loading}
        <p class="muted">جارٍ التحميل…</p>
      {:else if connections.length === 0}
        <div class="empty">
          <p>أضف اتصالاً لتشغيل الموديلات المحلية أو السحابية.</p>
          <div class="chips">
            {#each PRESETS.slice(0, 6) as entry (entry.id)}
              <button
                class="chip"
                on:click={() => {
                  editing = null;
                  preset = entry;
                  form = draftFrom(entry);
                  dialogOpen = true;
                }}>{entry.label}</button
              >
            {/each}
          </div>
        </div>
      {:else}
        <ul class="connections">
          {#each connections as connection (connection.id)}
            <li>
              <span class="avatar">AI</span>
              <div class="meta">
                <strong>{connection.name}</strong>
                <span class="muted small">
                  {connection.kind === "gemini"
                    ? "Google Gemini"
                    : connection.kind === "anthropic"
                      ? "Anthropic"
                      : connection.baseUrl || "OpenAI-compatible"}
                  {connection.model ? ` · ${connection.model}` : ""}
                </span>
              </div>
              <div class="actions">
                <button class="btn small" disabled={testingId === connection.id} on:click={() => test(connection)}>
                  {testingId === connection.id ? "جارٍ الاختبار…" : "اختبار"}
                </button>
                <button class="btn small ghost" on:click={() => openEdit(connection)}>تعديل</button>
                <button class="btn small ghost danger" disabled={deletingId === connection.id} on:click={() => remove(connection)}>
                  حذف
                </button>
              </div>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    {#if connections.length > 0}
      <section class="block">
        <div class="block-head">
          <h2>اسأل</h2>
          <select bind:value={selectedId}>
            {#each connections as connection (connection.id)}
              <option value={connection.id}>{connection.name}</option>
            {/each}
          </select>
        </div>
        <div class="ask">
          <textarea
            bind:value={prompt}
            placeholder="اكتب سؤالك أو البرومبت… (Ctrl+Enter للإرسال)"
            on:keydown={(event) => {
              if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
                event.preventDefault();
                void ask();
              }
            }}
          ></textarea>
          <div class="ask-actions">
            <span class="muted small">
              {activeConnection?.model ? `الموديل: ${activeConnection.model}` : ""}
            </span>
            <button class="btn primary" disabled={asking || !prompt.trim()} on:click={ask}>
              {asking ? "جارٍ التفكير…" : "إرسال"}
            </button>
          </div>
        </div>

        {#if askError}
          <p class="error">{askError}</p>
        {/if}

        {#if answer}
          <div class="answer">
            <div class="answer-head">
              <span class="muted small">الرد</span>
              <button class="link" on:click={copyAnswer}>نسخ</button>
            </div>
            <pre>{answer}</pre>
          </div>
        {/if}
      </section>
    {/if}
  </div>
</section>

{#if dialogOpen}
  <div class="overlay" role="presentation" on:click={() => (dialogOpen = false)}>
    <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" on:click|stopPropagation on:keydown|stopPropagation>
      <h3>{editing === null ? "إضافة اتصال" : "تعديل الاتصال"}</h3>

      <label class="field">
        <span>النوع</span>
        <select value={preset.id} on:change={(event) => choosePreset((event.currentTarget as HTMLSelectElement).value)}>
          {#each PRESETS as entry (entry.id)}
            <option value={entry.id}>{entry.label}</option>
          {/each}
        </select>
        <small class="muted">{preset.hint}</small>
      </label>

      <label class="field">
        <span>الاسم</span>
        <input bind:value={form.name} placeholder="اسم يظهر في القائمة" />
      </label>

      {#if form.kind === "openai"}
        <label class="field">
          <span>عنوان الخادم</span>
          <input bind:value={form.baseUrl} dir="ltr" placeholder="http://127.0.0.1:11434/v1" />
          <small class="muted">لـ Ollama: ‎/v1‎ بعد المنفذ. لأي خادم متوافق مع OpenAI نفس الصيغة.</small>
        </label>
      {/if}

      <label class="field">
        <span>الموديل</span>
        <input bind:value={form.model} dir="ltr" placeholder={form.kind === "gemini" ? "gemini-2.0-flash" : "llama3.2"} />
        <small class="muted">اترك اسم الموديل كما هو في الخادم (مثال: llama3.2، lmstudio-community/…).</small>
      </label>

      <label class="field">
        <span>مفتاح API {preset.needsKey ? "" : "(اختياري للخوادم المحلية)"}</span>
        <input bind:value={form.apiKey} type="password" dir="ltr" placeholder="sk-…" autocomplete="off" />
        <small class="muted">يُحفظ محلياً في قاعدة بيانات التطبيق ولا يُرسل إلا لذلك الخادم.</small>
      </label>

      <div class="dialog-actions">
        <button class="btn ghost" on:click={() => (dialogOpen = false)}>إلغاء</button>
        <button class="btn primary" disabled={saving} on:click={save}>{saving ? "جارٍ الحفظ…" : "حفظ"}</button>
      </div>
    </div>
  </div>
{/if}

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

  .content {
    flex: 1;
    min-height: 0;
    padding: 20px 26px 60px;
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .block {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: var(--bg-elevated);
    padding: 14px 16px;
  }

  .block-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 10px;
  }

  h2 {
    margin: 0;
    font-size: 0.95em;
  }

  .small {
    font-size: 0.76em;
  }

  .empty {
    display: flex;
    flex-direction: column;
    gap: 10px;
    color: var(--text-muted);
    font-size: 0.88em;
  }

  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .chip {
    border: 1px solid var(--border);
    background: var(--bg-raised);
    color: var(--text-muted);
    border-radius: 999px;
    padding: 4px 12px;
    font-size: 0.76em;
  }

  .chip:hover {
    color: var(--accent);
    border-color: var(--accent);
  }

  .connections {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }

  .connections li {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 2px;
    border-top: 1px solid var(--border);
  }

  .connections li:first-child {
    border-top: none;
  }

  .avatar {
    width: 34px;
    height: 34px;
    display: grid;
    place-items: center;
    border-radius: 9px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 0.72em;
    font-weight: 700;
    flex-shrink: 0;
  }

  .meta {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
    flex: 1;
  }

  .meta strong {
    font-size: 0.92em;
  }

  .meta span {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    direction: ltr;
    text-align: start;
    unicode-bidi: plaintext;
  }

  .actions {
    display: flex;
    gap: 6px;
  }

  .ask textarea {
    width: 100%;
    min-height: 110px;
    resize: vertical;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    padding: 10px 12px;
    font-family: inherit;
    font-size: 0.9em;
    line-height: 1.8;
  }

  .ask-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    margin-top: 8px;
  }

  .answer {
    margin-top: 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    background: var(--bg);
  }

  .answer-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
  }

  .answer pre {
    margin: 0;
    padding: 12px;
    white-space: pre-wrap;
    word-break: break-word;
    font-family: inherit;
    font-size: 0.9em;
    line-height: 1.9;
    max-height: 420px;
    overflow: auto;
  }

  select {
    max-width: 200px;
  }

  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0, 0, 0, 0.55);
    display: grid;
    place-items: center;
    z-index: 60;
  }

  .dialog {
    width: min(520px, 92vw);
    max-height: 88vh;
    overflow: auto;
    background: var(--bg-elevated);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius);
    padding: 20px;
    box-shadow: 0 30px 80px rgba(0, 0, 0, 0.5);
  }

  .dialog h3 {
    margin: 0 0 12px;
    font-size: 1.05em;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
    margin-bottom: 12px;
    font-size: 0.84em;
  }

  .field span {
    color: var(--text-muted);
  }

  .field input,
  .field select {
    width: 100%;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text);
    font-family: inherit;
    font-size: 0.95em;
  }

  .field small {
    font-size: 0.86em;
    line-height: 1.6;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 16px;
  }

  .error {
    color: var(--danger);
    font-size: 0.84em;
  }
</style>
