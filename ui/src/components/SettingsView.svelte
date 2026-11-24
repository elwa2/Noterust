<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import { settings } from "../lib/store";
  import { arabicDigits, arabicDateTime, CURRENCIES, fileSize, NOUNS, plural } from "../lib/format";
  import BackButton from "./BackButton.svelte";
  import type { BackupEntry, DataInfo } from "../lib/types";

  let info: DataInfo | null = null;
  let backups: BackupEntry[] = [];
  let dataError = "";
  let backupBusy = false;

  let fontSize = Number($settings["font_size"] ?? "16") || 16;
  let defaultEditor = false;
  let autostart = false;

  $: theme = $settings["theme"] ?? "dark";
  $: fontFamily = $settings["font_family"] ?? "Cairo";
  $: wrap = ($settings["word_wrap"] ?? "true") === "true";
  $: retention = $settings["trash_retention_days"] ?? "30";
  $: backupRetention = $settings["backup_retention"] ?? "14";
  $: currency = $settings["default_currency"] ?? "EGP";

  onMount(async () => {
    await Promise.all([loadInfo(), loadShellState()]);
  });

  async function loadShellState() {
    try {
      const state = await api.shellState();
      defaultEditor = state.defaultEditor;
      autostart = state.autostart;
    } catch {
      /* registry unreadable: keep the toggles off */
    }
  }

  async function loadInfo() {
    try {
      info = await api.dataInfo();
      backups = await api.listBackups();
    } catch (error) {
      dataError = store.errorMessage(error);
    }
  }

  async function save(key: string, value: string) {
    await store.updateSetting(key, value);
  }

  async function applyFontSize() {
    await save("font_size", String(fontSize));
  }

  /** Number inputs: clamp into range and write the clamped value back to the field. */
  async function clampSave(event: Event, key: string, min: number, max: number) {
    const input = event.currentTarget as HTMLInputElement;
    const parsed = Number.parseInt(input.value, 10);
    const value = Number.isFinite(parsed) ? Math.min(max, Math.max(min, parsed)) : min;
    input.value = String(value);
    await save(key, String(value));
  }

  async function backupNow() {
    backupBusy = true;
    try {
      const path = await api.backupNow();
      await loadInfo();
      store.toast(`تم إنشاء نسخة احتياطية: ${path}`);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    } finally {
      backupBusy = false;
    }
  }

  async function restore(entry: BackupEntry) {
    if (!confirm(`استعادة النسخة «${entry.name}»؟ سيُستبدل محتوى قاعدة البيانات الحالية.`)) return;
    try {
      await api.restoreBackup(entry.path);
      await store.refreshAll();
      store.toast("تمت الاستعادة — أعد تشغيل التطبيق لضمان تحديث كل الواجهة.");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function exportJson() {
    try {
      const path = await api.exportJson();
      if (path) store.toast(`تم التصدير إلى ${path}`);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function exportZip() {
    try {
      const path = await api.exportZip();
      if (path) store.toast(path);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function importJson() {
    try {
      const stats = await api.importJsonFile();
      if (stats) {
        store.toast(
          `تم الاستيراد: ${plural(stats.notes, NOUNS.note)} و${plural(stats.folders, {
            zero: "لا مجلدات",
            one: "مجلد واحد",
            two: "مجلدان",
            few: "{n} مجلدات",
            many: "{n} مجلداً",
            other: "{n} مجلد",
          })}`,
        );
        await store.refreshAll();
      }
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function importFiles() {
    try {
      const stats = await api.importDialog(currentFolderId());
      if (stats) {
        store.toast(`تم استيراد ${arabicDigits(stats.notes)} ملف`);
        await store.refreshAll();
      }
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function importFolder() {
    try {
      const stats = await api.importFolderDialog(currentFolderId());
      if (stats) {
        store.toast(`تم استيراد ${arabicDigits(stats.notes)} ملف`);
        await store.refreshAll();
      }
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  function currentFolderId(): number | null {
    let folderId: number | null = null;
    const unsubscribe = store.activeFolder.subscribe((value) => (folderId = value));
    unsubscribe();
    return folderId;
  }

  async function toggleDefaultEditor() {
    try {
      const message = await api.setDefaultEditor(defaultEditor);
      store.toast(message);
    } catch (error) {
      defaultEditor = !defaultEditor;
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function toggleAutostart() {
    try {
      await api.setAutostart(autostart);
      store.toast(autostart ? "سيبدأ التطبيق مع ويندوز" : "تم إلغاء التشغيل التلقائي");
    } catch (error) {
      autostart = !autostart;
      store.toast(store.errorMessage(error), "error");
    }
  }

  const shortcuts: [string, string][] = [
    ["Ctrl+N", "ملاحظة جديدة"],
    ["Ctrl+O", "فتح ملف"],
    ["Ctrl+S", "حفظ"],
    ["Ctrl+Shift+S", "حفظ باسم"],
    ["Ctrl+W", "إغلاق تبويب"],
    ["Ctrl+Tab", "التبديل بين التبويبات"],
    ["Ctrl+F", "بحث"],
    ["Ctrl+H", "استبدال"],
    ["Ctrl+Shift+F", "بحث في كل الملاحظات"],
    ["Ctrl+P", "لوحة الأوامر"],
    ["Ctrl+G", "الذهاب إلى سطر"],
    ["Ctrl+D", "تكرار السطر"],
    ["F5", "إدراج التاريخ والوقت"],
    ["Ctrl+Shift+N", "مجلد جديد"],
    ["Ctrl+,", "الإعدادات"],
    ["Delete", "حذف إلى السلة"],
  ];
</script>

<section class="view-page">
  <header class="page-head">
    <BackButton />
    <div>
      <h1>الإعدادات</h1>
      <p class="muted">المظهر، النسخ الاحتياطي، البيانات والاختصارات.</p>
    </div>
  </header>

  <div class="scroll content">
    <div class="columns">
      <div class="left">
        <div class="card">
          <h3>المظهر</h3>
          <div class="row-field">
            <label for="theme">الثيم</label>
            <select id="theme" value={theme} on:change={(event) => save("theme", (event.currentTarget as HTMLSelectElement).value)}>
              <option value="dark">داكن</option>
              <option value="light">فاتح</option>
              <option value="system">حسب النظام</option>
            </select>
          </div>

          <div class="row-field">
            <label for="font-family">الخط</label>
            <select
              id="font-family"
              value={fontFamily}
              on:change={(event) => save("font_family", (event.currentTarget as HTMLSelectElement).value)}
            >
              <option value="Cairo">Cairo (مضمّن)</option>
              <option value="Segoe UI">Segoe UI</option>
              <option value="Noto Naskh Arabic">Noto Naskh Arabic</option>
              <option value="Tahoma">Tahoma</option>
            </select>
          </div>

          <div class="row-field">
            <label for="font-size">حجم الخط: {arabicDigits(fontSize)}</label>
            <input
              id="font-size"
              type="range"
              min="12"
              max="24"
              bind:value={fontSize}
              on:change={applyFontSize}
            />
          </div>

          <div class="row-field">
            <label for="currency">عملة المالية</label>
            <select
              id="currency"
              value={currency}
              on:change={(event) => save("default_currency", (event.currentTarget as HTMLSelectElement).value)}
            >
              {#each CURRENCIES as item (item.code)}
                <option value={item.code}>{item.name} ({item.symbol})</option>
              {/each}
            </select>
          </div>

          <div class="row-field">
            <label for="wrap-toggle">التفاف الأسطر</label>
            <input
              id="wrap-toggle"
              type="checkbox"
              checked={wrap}
              on:change={(event) => save("word_wrap", (event.currentTarget as HTMLInputElement).checked ? "true" : "false")}
            />
          </div>
        </div>

        <div class="card">
          <h3>الاحتفاظ والنسخ</h3>
          <div class="row-field">
            <label for="trash-days">الاحتفاظ بالمحذوفات (أيام)</label>
            <input
              id="trash-days"
              type="number"
              min="1"
              max="365"
              value={retention}
              on:change={(event) => clampSave(event, "trash_retention_days", 1, 365)}
            />
          </div>
          <div class="row-field">
            <label for="backup-keep">عدد النسخ الاحتياطية المحفوظة</label>
            <input
              id="backup-keep"
              type="number"
              min="1"
              max="90"
              value={backupRetention}
              on:change={(event) => clampSave(event, "backup_retention", 1, 90)}
            />
          </div>
        </div>

        <div class="card">
          <h3>النظام</h3>
          <div class="row-field">
            <label for="default-editor">تعيين NoteRust كمحرر افتراضي لملفات .txt</label>
            <input
              id="default-editor"
              type="checkbox"
              bind:checked={defaultEditor}
              on:change={toggleDefaultEditor}
            />
          </div>
          <div class="row-field">
            <label for="autostart">التشغيل مع بدء ويندوز</label>
            <input id="autostart" type="checkbox" bind:checked={autostart} on:change={toggleAutostart} />
          </div>
          <p class="faint">
            يعرض ويندوز رسالة تأكيد عند تعيين التطبيق كافتراضي، ولا يمكن تجاوزها برمجياً.
          </p>
        </div>

        <div class="card">
          <h3>الاختصارات</h3>
          <ul class="shortcuts">
            {#each shortcuts as [keys, label] (keys)}
              <li><kbd>{keys}</kbd><span>{label}</span></li>
            {/each}
          </ul>
        </div>
      </div>

      <div class="right">
        <div class="card">
          <h3>البيانات</h3>
          {#if dataError}
            <p class="error">{dataError}</p>
          {:else if info}
            <dl class="info">
              <dt>الوضع</dt>
              <dd>{info.portable ? "محمول (بجوار exe)" : "AppData"}</dd>
              <dt>مجلد البيانات</dt>
              <dd class="path">{info.dataDir}</dd>
              <dt>قاعدة البيانات</dt>
              <dd class="path">{info.dbPath}</dd>
              <dt>النسخ الاحتياطية</dt>
              <dd class="path">{info.backupDir}</dd>
            </dl>
          {:else}
            <p class="faint">جارٍ التحميل…</p>
          {/if}
          <div class="row wrap">
            <button class="btn" on:click={() => api.openDataFolder()}>فتح مجلد البيانات</button>
            <button class="btn" on:click={backupNow} disabled={backupBusy}>نسخة احتياطية الآن</button>
          </div>
        </div>

        <div class="card">
          <h3>النسخ الاحتياطية</h3>
          {#if backups.length === 0}
            <p class="faint">لا نسخ بعد. تُنشأ نسخة تلقائياً مرة يومياً.</p>
          {:else}
            <ul class="backups">
              {#each backups as entry (entry.path)}
                <li>
                  <span class="backup-name">{entry.name}</span>
                  <span class="faint">{fileSize(entry.size)} · {arabicDateTime(entry.createdAt)}</span>
                  <button class="btn small" on:click={() => restore(entry)}>استعادة</button>
                </li>
              {/each}
            </ul>
          {/if}
        </div>

        <div class="card">
          <h3>الاستيراد والتصدير</h3>
          <div class="row wrap">
            <button class="btn" on:click={exportJson}>تصدير JSON</button>
            <button class="btn" on:click={exportZip}>تصدير ZIP (Markdown)</button>
            <button class="btn" on:click={importJson}>استيراد JSON</button>
            <button class="btn" on:click={importFiles}>استيراد ملفات</button>
            <button class="btn" on:click={importFolder}>استيراد مجلد</button>
          </div>
        </div>

        <div class="card about">
          <h3>NoteRust</h3>
          <p class="muted">
            تطبيق ملاحظات محلي لويندوز، مبني بـ Rust وTauri مع SQLite. كل بياناتك تبقى على
            جهازك.
          </p>
          <p class="faint">الإصدار ١.٠٫٠ · يتطلب WebView2 (مضمّن في ويندوز ١٠ و١١).</p>
        </div>
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

  .content {
    flex: 1;
    padding: 18px 26px 40px;
    overflow-y: auto;
  }

  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 14px;
    align-items: start;
  }

  .card {
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px 16px;
    margin-bottom: 12px;
  }

  h3 {
    margin: 0 0 10px;
    font-size: 0.92em;
  }

  .row-field {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 7px 0;
    border-bottom: 1px solid var(--border);
  }

  .row-field:last-child {
    border-bottom: none;
  }

  .row-field label {
    flex: 1;
    font-size: 0.85em;
    color: var(--text-muted);
  }

  .row-field select,
  .row-field input[type="number"] {
    width: 190px;
  }

  .row-field input[type="range"] {
    width: 190px;
    padding: 0;
    accent-color: var(--accent);
    background: transparent;
    border: none;
  }

  .row-field input[type="checkbox"] {
    width: 17px;
    height: 17px;
    accent-color: var(--accent);
  }

  .shortcuts {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 14px;
  }

  .shortcuts li {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 0.8em;
    color: var(--text-muted);
  }

  .info {
    display: grid;
    grid-template-columns: 110px 1fr;
    gap: 6px 10px;
    margin: 0 0 12px;
    font-size: 0.8em;
  }

  .info dt {
    color: var(--text-faint);
  }

  .info dd {
    margin: 0;
  }

  .path {
    direction: ltr;
    text-align: left;
    word-break: break-all;
    font-size: 0.94em;
    color: var(--text-muted);
  }

  .row {
    display: flex;
    gap: 8px;
  }

  .row.wrap {
    flex-wrap: wrap;
  }

  .btn.small {
    font-size: 0.76em;
    padding: 3px 9px;
  }

  .backups {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
  }

  .backups li {
    display: flex;
    align-items: center;
    gap: 9px;
    font-size: 0.8em;
  }

  .backup-name {
    flex: 1;
    direction: ltr;
    text-align: left;
    font-size: 0.92em;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .about p {
    margin: 0 0 6px;
    font-size: 0.82em;
    line-height: 1.7;
  }

  .faint {
    font-size: 0.76em;
    line-height: 1.7;
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.8em;
  }
</style>
