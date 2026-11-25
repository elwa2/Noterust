<script lang="ts">
  import { onDestroy, tick } from "svelte";
  import { api } from "../lib/api";
  import * as store from "../lib/store";
  import {
    activeNote,
    activeTab,
    tabs,
    folders,
    saveStatus,
    lastSavedAt,
    settings,
    fileChangeNotice,
    alwaysOnTop,
    compact,
  } from "../lib/store";
  import { arabicDateTime, NOUNS, plural } from "../lib/format";
  import { renderMarkdown, setImageOptions } from "../lib/markdown";
  import { htmlToMarkdown } from "../lib/htmlToMarkdown";
  import { editorCommand } from "../lib/bus";
  import type { EditorCommand } from "../lib/bus";
  import type { Note } from "../lib/types";
  import StatusBar from "./StatusBar.svelte";
  import FindReplace from "./FindReplace.svelte";
  import SlashMenu from "./SlashMenu.svelte";
  import LockDialog from "./LockDialog.svelte";
  import VersionHistory from "./VersionHistory.svelte";
  import Modal from "./Modal.svelte";
  import Icon from "./Icon.svelte";

  let note: Note | null = null;
  let content = "";
  let loadedKey = "";
  /** The note `content` currently belongs to, used to flush edits on tab switches. */
  let loadedNote: Note | null = null;
  let dirty = false;
  let textarea: HTMLTextAreaElement | null = null;

  /** Session passwords for notes unlocked during this run. */
  let passwords: Record<number, string> = {};

  let saveTimer: ReturnType<typeof setTimeout> | undefined;
  let listTimer: ReturnType<typeof setTimeout> | undefined;

  let findOpen = false;
  let showReplace = false;
  let findText = "";
  let replaceText = "";
  let useRegex = false;
  let caseSensitive = false;
  let findError = "";
  let matches: { start: number; end: number }[] = [];

  let slashOpen = false;
  let slashQuery = "";
  let slashPos = { top: 12, left: 24 };

  let gotoOpen = false;
  let gotoValue = "";

  let moreOpen = false;
  let formatOpen = false;
  /** Rendered Markdown preview instead of the raw textarea. */
  let preview = false;
  /** Rendered preview HTML; refreshed only when the user is not editing it. */
  let previewHtml = "";
  /** True while the caret is inside the editable preview. */
  let readerEditing = false;
  let colorOpen = false;
  /** Pasted images, loaded from disk as `data:` URLs for the preview. */
  let imageAssets: Record<string, string> = {};
  const loadingImages = new Set<string>();
  let readerEl: HTMLElement | null = null;

  let lockMode: "lock" | "remove" | null = null;
  let versionsOpen = false;

  let unlockPassword = "";
  let unlockError = "";
  let unlocking = false;

  let tagDraft = "";

  const colors = ["#e8b830", "#e07a5f", "#57c785", "#4fa8e0", "#b48ce8", "#8ab4f8", "#f2a65a", null];

  $: note = $activeNote;
  $: wrap = ($settings["word_wrap"] ?? "true") === "true";
  let directionAttr: "ltr" | "rtl" | "auto" = "auto";
  $: directionAttr =
    note?.direction === "ltr" ? "ltr" : note?.direction === "rtl" ? "rtl" : "auto";
  $: sessionPassword = note ? (passwords[note.id] ?? null) : null;
  $: lockedView = !!note && note.isLocked && !sessionPassword;
  $: matches = computeMatches();
  // While the preview has the caret, its DOM is the source of truth: re-rendering
  // it would throw the caret away mid-typing.
  $: if (preview && !readerEditing) previewHtml = renderMarkdown(content, imageAssets);
  $: if (!preview) previewHtml = "";
  $: externalChanged = !!note && $fileChangeNotice === note.id;
  $: trashed = !!note?.deletedAt;
  $: trashDaysLeft = note?.deletedAt
    ? Math.max(
        0,
        Number($settings["trash_retention_days"] ?? 30) -
          Math.floor((Date.now() - note.deletedAt) / 86_400_000),
      )
    : 0;

  $: if (content) void loadNoteImages(content);
  $: if (preview) {
    void previewHtml;
    void refreshReader();
  }

  $: if (note) {
    const key = `${note.id}:${passwords[note.id] ?? ""}`;
    if (key !== loadedKey) {
      // Switching tabs while a debounce is pending used to drop the last edits.
      if (dirty && loadedNote && !loadedNote.deletedAt) {
        void flushSave(loadedNote, content);
      }
      loadedKey = key;
      void hydrate(note);
    }
  }

  $: if (note?.folderId !== undefined && note) {
    /* keeps the move-select in sync with the loaded note */
  }

  /** Saves an older note's pending edits before its tab is switched away. */
  async function flushSave(target: Note, text: string) {
    clearTimeout(saveTimer);
    dirty = false;
    const password = passwords[target.id];
    try {
      let saved: Note | null = null;
      if (target.isLocked && password) {
        saved = await api.saveLockedNote(target.id, password, text);
      } else if (target.sourcePath) {
        saved = await api.saveExternalFile(target.id, text);
      } else if (!target.isLocked) {
        saved = await api.updateNoteContent(target.id, text);
      }
      if (saved) {
        // Keep the tab's cached copy in sync without stealing the window title.
        tabs.update((list) => list.map((item) => (item.id === saved!.id ? saved! : item)));
        scheduleListRefresh();
      }
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function hydrate(current: Note) {
    unlockError = "";
    readerEditing = false;
    loadedNote = current;
    if (current.isLocked) {
      const password = passwords[current.id];
      if (!password) {
        content = "";
        saveStatus.set("saved");
        return;
      }
      try {
        content = await api.unlockNote(current.id, password);
        saveStatus.set("saved");
      } catch (error) {
        passwords = withoutKey(passwords, current.id);
        content = "";
        unlockError = "تعذر فك القفل";
        store.toast(store.errorMessage(error), "error");
      }
      return;
    }
    content = current.content;
    saveStatus.set("saved");
    dirty = false;
  }

  function withoutKey(map: Record<number, string>, id: number) {
    const copy = { ...map };
    delete copy[id];
    return copy;
  }

  function markDirty() {
    dirty = true;
    saveStatus.set("unsaved");
    scheduleSave();
  }

  function scheduleSave() {
    clearTimeout(saveTimer);
    saveTimer = setTimeout(() => void saveNow(), 500);
  }

  function scheduleListRefresh() {
    clearTimeout(listTimer);
    listTimer = setTimeout(() => void store.refreshNotes(), 900);
  }

  function applySavedNote(updated: Note) {
    loadedKey = "";
    store.updateTab(updated);
    void store.refreshNotes();
  }

  async function saveNow() {
    const current = note;
    if (!current || current.deletedAt) return;
    if (current.isLocked && !sessionPassword) {
      saveStatus.set("error");
      return;
    }
    clearTimeout(saveTimer);
    saveStatus.set("saving");
    try {
      let updated: Note;
      if (current.isLocked && sessionPassword) {
        updated = await api.saveLockedNote(current.id, sessionPassword, content);
      } else if (current.sourcePath) {
        updated = await api.saveExternalFile(current.id, content);
      } else {
        updated = await api.updateNoteContent(current.id, content);
      }
      loadedKey = `${updated.id}:${passwords[updated.id] ?? ""}`;
      store.updateTab(updated);
      dirty = false;
      lastSavedAt.set(Date.now());
      saveStatus.set("saved");
      scheduleListRefresh();
    } catch (error) {
      saveStatus.set("error");
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function saveAs() {
    const current = note;
    if (!current) return;
    try {
      const suggested = `${(current.title || "ملاحظة").slice(0, 60).trim() || "ملاحظة"}.txt`;
      const path = await api.saveFileAs(
        content,
        suggested,
        current.encoding ?? "UTF-8",
        current.lineEnding ?? "CRLF",
      );
      if (path) store.toast(`تم الحفظ في ${path}`);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function updateTitle(title: string) {
    const current = note;
    if (!current) return;
    try {
      const updated = await api.updateNoteTitle(current.id, title);
      store.updateTab(updated);
      void store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function moveTo(value: string) {
    const current = note;
    if (!current) return;
    const folderId = value === "" ? null : Number(value);
    try {
      const updated = await api.moveNote(current.id, folderId);
      store.updateTab(updated);
      await store.refreshAll();
      store.toast("تم نقل الملاحظة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function togglePin() {
    const current = note;
    if (!current) return;
    try {
      const updated = await api.pinNote(current.id, !current.isPinned);
      store.updateTab(updated);
      void store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function toggleArchive() {
    const current = note;
    if (!current) return;
    try {
      const updated = await api.archiveNote(current.id, !current.isArchived);
      store.updateTab(updated);
      await store.refreshAll();
      store.toast(updated.isArchived ? "تمت الأرشفة" : "تم إلغاء الأرشفة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function setColor(color: string | null) {
    const current = note;
    if (!current) return;
    colorOpen = false;
    try {
      const updated = await api.setNoteColor(current.id, color);
      store.updateTab(updated);
      void store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function cycleDirection() {
    const current = note;
    if (!current) return;
    const next =
      current.direction === "auto" ? "rtl" : current.direction === "rtl" ? "ltr" : "auto";
    try {
      const updated = await api.setNoteDirection(current.id, next);
      store.updateTab(updated);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function toggleWrap() {
    await store.updateSetting("word_wrap", wrap ? "false" : "true");
  }

  async function deleteNote() {
    const current = note;
    if (!current) return;
    if (!confirm("نقل الملاحظة إلى سلة المحذوفات؟")) return;
    try {
      await api.deleteNote(current.id);
      await store.removeNoteFromUi(current.id);
      store.toast("نُقلت إلى سلة المحذوفات", "ok", {
        label: "تراجع",
        run: () => {
          void api
            .restoreNote(current.id)
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

  async function restoreFromTrash() {
    const current = note;
    if (!current) return;
    try {
      await api.restoreNote(current.id);
      loadedKey = "";
      await store.openNote(current.id, true);
      await store.refreshAll();
      store.toast("تم استرجاع الملاحظة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function purgeForever() {
    const current = note;
    if (!current) return;
    if (!confirm("حذف الملاحظة نهائياً؟ لا يمكن التراجع.")) return;
    try {
      await api.purgeNote(current.id);
      await store.removeNoteFromUi(current.id);
      store.toast("حُذفت الملاحظة نهائياً");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function duplicateNote() {
    const current = note;
    if (!current) return;
    try {
      const copy = await api.duplicateNote(current.id);
      await store.refreshAll();
      await store.openNote(copy.id);
      store.toast("تم إنشاء نسخة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function exportNote(format: "txt" | "md") {
    const current = note;
    if (!current) return;
    moreOpen = false;
    try {
      const path = await api.exportNoteFile(current.id, format);
      if (path) store.toast(`تم التصدير إلى ${path}`);
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function unlockNote() {
    const current = note;
    if (!current) return;
    unlocking = true;
    unlockError = "";
    try {
      const plain = await api.unlockNote(current.id, unlockPassword);
      passwords = { ...passwords, [current.id]: unlockPassword };
      content = plain;
      unlockPassword = "";
    } catch (error) {
      unlockError = "كلمة المرور غير صحيحة";
      void error;
    } finally {
      unlocking = false;
    }
  }

  async function addTag() {
    const current = note;
    const name = tagDraft.trim();
    if (!current || !name) return;
    tagDraft = "";
    const names = [...current.tags.map((tag) => tag.name), name];
    try {
      const tags = await api.setNoteTags(current.id, names);
      store.updateTab({ ...current, tags });
      void store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  async function removeTag(name: string) {
    const current = note;
    if (!current) return;
    const names = current.tags.filter((tag) => tag.name !== name).map((tag) => tag.name);
    try {
      const tags = await api.setNoteTags(current.id, names);
      store.updateTab({ ...current, tags });
      void store.refreshNotes();
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  // -------------------------------------------------------------------------
  // Text helpers
  // -------------------------------------------------------------------------

  function insertText(text: string, caretOffset = text.length) {
    const element = textarea;
    if (!element) return;
    const start = element.selectionStart ?? content.length;
    const end = element.selectionEnd ?? start;
    content = content.slice(0, start) + text + content.slice(end);
    const caret = start + (caretOffset >= 0 ? caretOffset : text.length + caretOffset);
    markDirty();
    void tick().then(() => {
      element.focus();
      element.setSelectionRange(caret, caret);
    });
  }

  function wrapSelection(prefix: string, suffix = prefix) {
    const element = textarea;
    if (!element) return;
    const start = element.selectionStart ?? 0;
    const end = element.selectionEnd ?? start;
    const selected = content.slice(start, end);
    content = content.slice(0, start) + prefix + selected + suffix + content.slice(end);
    markDirty();
    const caret = start + prefix.length + selected.length;
    void tick().then(() => {
      element.focus();
      element.setSelectionRange(
        selected ? start + prefix.length : start + prefix.length,
        selected ? caret : caret,
      );
    });
  }

  function insertLinePrefix(prefix: string) {
    const element = textarea;
    if (!element) return;
    const start = element.selectionStart ?? 0;
    const lineStart = content.lastIndexOf("\n", start - 1) + 1;
    content = content.slice(0, lineStart) + prefix + content.slice(lineStart);
    markDirty();
    void tick().then(() => {
      element.focus();
      const caret = start + prefix.length;
      element.setSelectionRange(caret, caret);
    });
  }

  function duplicateLine() {
    const element = textarea;
    if (!element) return;
    const position = element.selectionStart ?? 0;
    const lineStart = content.lastIndexOf("\n", position - 1) + 1;
    let lineEnd = content.indexOf("\n", position);
    if (lineEnd === -1) lineEnd = content.length;
    const line = content.slice(lineStart, lineEnd);
    content = `${content.slice(0, lineEnd)}\n${line}${content.slice(lineEnd)}`;
    markDirty();
    void tick().then(() => {
      element.focus();
      const caret = position + line.length + 1;
      element.setSelectionRange(caret, caret);
    });
  }

  function insertDate() {
    insertText(arabicDateTime(Date.now()));
  }

  function selectRange(start: number, end: number) {
    const element = textarea;
    if (!element) return;
    element.focus();
    element.setSelectionRange(start, end);
    const line = content.slice(0, start).split("\n").length;
    const lineHeight = parseFloat(getComputedStyle(element).lineHeight) || 26;
    element.scrollTop = Math.max(0, (line - 3) * lineHeight);
  }

  function computeMatches(): { start: number; end: number }[] {
    const query = findText;
    if (!query) {
      findError = "";
      return [];
    }
    const flags = caseSensitive ? "g" : "gi";
    try {
      if (useRegex) {
        const pattern = new RegExp(query, flags);
        const output: { start: number; end: number }[] = [];
        let match: RegExpExecArray | null;
        let guard = 0;
        while ((match = pattern.exec(content)) !== null && guard < 5000) {
          guard += 1;
          if (match[0].length === 0) {
            pattern.lastIndex += 1;
            continue;
          }
          output.push({ start: match.index, end: match.index + match[0].length });
        }
        findError = "";
        return output;
      }
      const haystack = caseSensitive ? content : content.toLowerCase();
      const needle = caseSensitive ? query : query.toLowerCase();
      const output: { start: number; end: number }[] = [];
      let from = 0;
      while (output.length < 5000) {
        const index = haystack.indexOf(needle, from);
        if (index === -1) break;
        output.push({ start: index, end: index + needle.length });
        from = index + Math.max(1, needle.length);
      }
      findError = "";
      return output;
    } catch {
      findError = "تعبير نمطي غير صالح";
      return [];
    }
  }

  function findNext(backwards = false) {
    if (matches.length === 0) return;
    const position = textarea?.selectionStart ?? 0;
    const hit = backwards
      ? [...matches].reverse().find((item) => item.end < position) ?? matches[matches.length - 1]
      : matches.find((item) => item.start > position) ?? matches[0];
    selectRange(hit.start, hit.end);
  }

  function replacementFor(hit: { start: number; end: number }): string {
    if (!useRegex) return replaceText;
    try {
      return content
        .slice(hit.start, hit.end)
        .replace(new RegExp(findText, caseSensitive ? "" : "i"), replaceText);
    } catch {
      return replaceText;
    }
  }

  function replaceOne() {
    if (matches.length === 0) return;
    const element = textarea;
    const position = element?.selectionStart ?? 0;
    const hit =
      matches.find((item) => item.start === position) ??
      matches.find((item) => item.start >= position) ??
      matches[0];
    const replacement = replacementFor(hit);
    content = content.slice(0, hit.start) + replacement + content.slice(hit.end);
    markDirty();
    selectRange(hit.start + replacement.length, hit.start + replacement.length);
  }

  function replaceAll() {
    if (matches.length === 0) return;
    let result = "";
    let last = 0;
    for (const hit of matches) {
      result += content.slice(last, hit.start);
      result += replacementFor(hit);
      last = hit.end;
    }
    result += content.slice(last);
    content = result;
    markDirty();
    store.toast("تم الاستبدال");
  }

  function goToLine() {
    const total = content.split("\n").length;
    const line = Number(gotoValue);
    gotoOpen = false;
    if (!line || line < 1) return;
    const target = Math.min(line, total);
    const start = content.split("\n").slice(0, target - 1).join("\n").length + (target > 1 ? 1 : 0);
    selectRange(start, start);
  }

  // -------------------------------------------------------------------------
  // Slash menu
  // -------------------------------------------------------------------------

  function updateSlash() {
    const element = textarea;
    if (!element) return;
    const position = element.selectionStart ?? 0;
    const before = content.slice(0, position);
    const lineStart = before.lastIndexOf("\n") + 1;
    const line = before.slice(lineStart);
    if (line.startsWith("/") && !line.includes(" ") && !line.includes("\t")) {
      slashQuery = line.slice(1);
      const lineIndex = before.split("\n").length - 1;
      const lineHeight = parseFloat(getComputedStyle(element).lineHeight) || 26;
      const top = Math.max(8, Math.min(lineIndex * lineHeight - element.scrollTop - 4, element.clientHeight - 320));
      slashPos = { top, left: 26 };
      slashOpen = true;
    } else {
      slashOpen = false;
    }
  }

  function chooseSlash(event: CustomEvent<{ template: string; caret: number }>) {
    const element = textarea;
    if (!element) return;
    const position = element.selectionStart ?? 0;
    const before = content.slice(0, position);
    const lineStart = before.lastIndexOf("\n") + 1;
    const { template, caret } = event.detail;
    content = content.slice(0, lineStart) + template + content.slice(position);
    slashOpen = false;
    markDirty();
    const target = lineStart + (caret > 0 ? caret : template.length);
    void tick().then(() => {
      element.focus();
      element.setSelectionRange(target, target);
    });
  }

  function onTextareaKeydown(event: KeyboardEvent) {
    if (slashOpen && ["ArrowDown", "ArrowUp", "Enter", "Escape"].includes(event.key)) {
      // Handled by SlashMenu's window listener.
      return;
    }
    if (event.key === "Tab") {
      event.preventDefault();
      insertText("  ", 2);
      return;
    }
    if (event.key === "Escape" && findOpen) {
      findOpen = false;
    }
  }

  function onTextareaInput(event: Event) {
    if (trashed) return;
    content = (event.currentTarget as HTMLTextAreaElement).value;
    markDirty();
    updateSlash();
  }

  /** Typing in the rendered preview writes Markdown back into the note. */
  function onReaderInput() {
    if (trashed || lockedView || !readerEl) return;
    readerEditing = true;
    content = htmlToMarkdown(readerEl);
    markDirty();
  }

  /** Saves a clipboard image and links it in the note. */
  async function attachImage(image: File, at: "textarea" | "reader") {
    const ext = (image.type.split("/")[1] ?? "png").toLowerCase();
    try {
      const base64 = (await readFileAsBase64(image)).split(",")[1] ?? "";
      const name = await api.saveAttachment(base64, ext);
      const markdown = `\n![صورة](noteimg://${name})\n`;
      if (at === "textarea") {
        insertText(markdown);
      } else if (readerEl) {
        readerEl.focus();
        document.execCommand("insertText", false, markdown);
        readerEditing = false;
        previewHtml = renderMarkdown(content, imageAssets);
        void tick().then(refreshReader);
      }
      store.toast("أُضيفت الصورة");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
  }

  function clipboardImage(event: ClipboardEvent): File | undefined {
    const items = event.clipboardData?.items;
    if (!items) return undefined;
    return Array.from(items)
      .filter((item) => item.kind === "file" && item.type.startsWith("image/"))
      .map((item) => item.getAsFile())
      .find((file): file is File => !!file);
  }

  /** Image files in the clipboard are saved to disk and linked in the note. */
  async function onTextareaPaste(event: ClipboardEvent) {
    if (trashed || note?.isLocked) return;
    const image = clipboardImage(event);
    if (!image) return;
    event.preventDefault();
    await attachImage(image, "textarea");
  }

  /** Inside the preview, images are attached and text is pasted as plain text. */
  async function onReaderPaste(event: ClipboardEvent) {
    if (trashed || lockedView) return;
    const image = clipboardImage(event);
    if (image) {
      event.preventDefault();
      await attachImage(image, "reader");
      return;
    }
    const text = event.clipboardData?.getData("text/plain");
    if (text === undefined) return;
    event.preventDefault();
    document.execCommand("insertText", false, text);
  }

  function readFileAsBase64(file: File): Promise<string> {
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onload = () => resolve(String(reader.result ?? ""));
      reader.onerror = () => reject(reader.error ?? new Error("تعذر قراءة الصورة"));
      reader.readAsDataURL(file);
    });
  }

  /** Loads every `noteimg://` file the note references, once per file. */
  async function loadNoteImages(text: string) {
    const names = new Set<string>();
    for (const match of text.matchAll(/!\[[^\]]*\]\(noteimg:\/\/([^)\s]+)/g)) {
      names.add(match[1]);
    }
    for (const name of names) {
      if (imageAssets[name] !== undefined || loadingImages.has(name)) continue;
      loadingImages.add(name);
      try {
        const url = await api.readAttachment(name);
        imageAssets = { ...imageAssets, [name]: url };
      } catch {
        imageAssets = { ...imageAssets, [name]: "" };
      } finally {
        loadingImages.delete(name);
      }
    }
  }

  async function copyCode(text: string) {
    try {
      await navigator.clipboard.writeText(text);
      store.toast("تم نسخ الكود كاملاً");
    } catch {
      store.toast("تعذر النسخ", "error");
    }
  }

  /** Zoom/rotate buttons and the code-block copy button live in the preview. */
  function onReaderClick(event: MouseEvent) {
    const button = (event.target as HTMLElement).closest<HTMLElement>("button[data-act]");
    if (!button) return;
    if (button.dataset.act === "copy-code") {
      event.preventDefault();
      const code = button.parentElement?.querySelector("code")?.textContent ?? "";
      void copyCode(code);
      return;
    }
    const figure = button.closest<HTMLElement>(".md-image");
    if (!figure) return;
    event.preventDefault();
    const index = Number(figure.dataset.img ?? "-1");
    if (index < 0) return;
    const image = figure.querySelector("img");
    const width = Number(figure.dataset.w ?? "0") || Math.round(image?.clientWidth ?? 0);
    const rotation = Number(figure.dataset.r ?? "0");
    let nextWidth = width;
    let nextRotation = rotation;
    switch (button.dataset.act) {
      case "zoom-in":
        nextWidth = Math.min(4000, Math.round(width * 1.25));
        break;
      case "zoom-out":
        nextWidth = Math.max(40, Math.round(width / 1.25));
        break;
      case "rotate-left":
        nextRotation = (rotation + 270) % 360;
        break;
      case "rotate-right":
        nextRotation = (rotation + 90) % 360;
        break;
      case "reset":
        nextWidth = 0;
        nextRotation = 0;
        break;
      default:
        return;
    }
    content = setImageOptions(content, index, { width: nextWidth, rotation: nextRotation });
    markDirty();
    readerEditing = false;
    previewHtml = renderMarkdown(content, imageAssets);
    void tick().then(() => {
      layoutImages();
      void refreshReader();
    });
  }

  /** Rotated images are painted turned, so their box height follows the width. */
  function layoutImages() {
    if (!readerEl) return;
    const available = Math.max(220, readerEl.clientWidth - 56);
    const visualLimit = Math.max(240, Math.round(window.innerHeight * 0.62));
    for (const figure of readerEl.querySelectorAll<HTMLElement>(".md-image")) {
      const frame = figure.querySelector<HTMLElement>(".md-image-frame");
      const image = frame?.querySelector("img");
      if (!frame || !image) continue;
      const rotation = ((Number(figure.dataset.r ?? "0") % 360) + 360) % 360;
      if (rotation % 180 === 0) {
        frame.style.height = "";
        image.style.maxWidth = "";
        continue;
      }
      const ratio = image.naturalWidth ? image.naturalHeight / image.naturalWidth : 1;
      const wanted = Number(figure.dataset.w ?? "0") || image.offsetWidth || available;
      const width = Math.round(
        Math.min(wanted, visualLimit, available / Math.max(ratio, 0.05)),
      );
      image.style.maxWidth = `${width}px`;
      frame.style.height = `${width}px`;
    }
  }

  async function refreshReader() {
    await tick();
    if (!preview || !readerEl) return;
    for (const image of readerEl.querySelectorAll("img")) {
      if (!image.complete) image.addEventListener("load", layoutImages, { once: true });
    }
    layoutImages();
  }

  function onWheel(event: WheelEvent) {
    if (!event.ctrlKey) return;
    event.preventDefault();
    const size = Number($settings["font_size"] ?? "16") || 16;
    const next = Math.max(12, Math.min(30, size + (event.deltaY < 0 ? 1 : -1)));
    void store.updateSetting("font_size", String(next));
  }

  // -------------------------------------------------------------------------
  // External file changes
  // -------------------------------------------------------------------------

  async function reloadExternal() {
    const current = note;
    if (!current) return;
    try {
      const text = await api.reloadExternalFile(current.id);
      content = text;
      loadedKey = `${current.id}:`;
      dirty = false;
      saveStatus.set("saved");
      await store.openNote(current.id, true);
      content = text;
      store.toast("تم إعادة تحميل الملف");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
    fileChangeNotice.set(null);
  }

  async function discardExternal() {
    const current = note;
    if (!current) return;
    try {
      const updated = await api.discardExternalLink(current.id);
      store.updateTab(updated);
      store.toast("تم فصل الملاحظة عن الملف");
    } catch (error) {
      store.toast(store.errorMessage(error), "error");
    }
    fileChangeNotice.set(null);
  }

  // -------------------------------------------------------------------------
  // Commands from the global shortcut handler
  // -------------------------------------------------------------------------

  const unsubscribe = editorCommand.subscribe((command) => {
    if (!command) return;
    handleCommand(command.name);
  });
  onDestroy(unsubscribe);

  function handleCommand(name: EditorCommand) {
    switch (name) {
      case "find":
        if (!note || lockedView) return;
        findOpen = true;
        showReplace = false;
        void tick().then(() => document.querySelector<HTMLInputElement>(".find input")?.focus());
        break;
      case "replace":
        if (!note || lockedView) return;
        findOpen = true;
        showReplace = true;
        void tick().then(() => document.querySelector<HTMLInputElement>(".find input")?.focus());
        break;
      case "go-to-line":
        if (!note || lockedView) return;
        gotoValue = "";
        gotoOpen = true;
        break;
      case "duplicate-line":
        if (!note || lockedView) return;
        duplicateLine();
        break;
      case "insert-date":
        if (!note || lockedView) return;
        insertDate();
        break;
      case "toggle-wrap":
        void toggleWrap();
        break;
      case "toggle-preview":
        togglePreview();
        break;
      case "save":
        void saveNow();
        break;
      case "save-as":
        void saveAs();
        break;
      case "focus":
        textarea?.focus();
        break;
    }
  }

  function closePopovers() {
    moreOpen = false;
    formatOpen = false;
    colorOpen = false;
  }

  function togglePreview() {
    preview = !preview;
    closePopovers();
    readerEditing = false;
    if (preview) {
      previewHtml = renderMarkdown(content, imageAssets);
      void tick().then(refreshReader);
    } else {
      void tick().then(() => textarea?.focus());
    }
  }
</script>

<svelte:window on:click={closePopovers} />

<section class="editor-pane" class:compact={$compact}>
  <div class="tabs">
    <div class="tab-strip">
      {#each $tabs as tab (tab.id)}
        <div class="tab" class:active={$activeTab === tab.id}>
          <button class="tab-label" on:click={() => store.openNote(tab.id)} title={tab.title}>
            {#if tab.isLocked}<span class="tiny"><Icon name="lock" size={11} /></span>{/if}
            {#if tab.sourcePath}<span class="tiny"><Icon name="book" size={11} /></span>{/if}
            <span class="ellipsis">{tab.title || "بدون عنوان"}</span>
          </button>
          <button class="tab-close" title="إغلاق (Ctrl+W)" on:click={() => store.closeTab(tab.id)}>✕</button>
        </div>
      {/each}
    </div>
    <button class="icon-btn" title="ملاحظة جديدة (Ctrl+N)" on:click={() => store.createNote()}>＋</button>
  </div>

  {#if !note}
    <div
      class="empty click-target"
      role="button"
      tabindex="0"
      aria-label="إنشاء ملاحظة جديدة"
      title="اضغط في أي مكان لإنشاء ملاحظة جديدة"
      on:click={() => void store.createNote()}
      on:keydown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          void store.createNote();
        }
      }}
    >
      <div class="empty-mark"><Icon name="notes" size={44} /></div>
      <h3>لا ملاحظة مفتوحة</h3>
      <p class="muted">اضغط في أي مكان هنا لإنشاء ملاحظة جديدة فوراً.</p>
      <div class="row">
        <button class="btn primary" on:click|stopPropagation={() => store.createNote()}>
          ＋ ملاحظة جديدة
        </button>
        <button class="btn" on:click|stopPropagation={() => store.setView("tasks")}>المهام</button>
      </div>
    </div>
  {:else}
    <div class="toolbar">
      <button class="icon-btn" title="حذف إلى السلة" on:click={deleteNote} disabled={trashed}>
        <Icon name="trash" />
      </button>
      <select
        class="move"
        value={note.folderId ?? ""}
        on:change={(event) => moveTo((event.currentTarget as HTMLSelectElement).value)}
        title="نقل إلى مجلد"
      >
        <option value="">بدون مجلد</option>
        {#each store.flattenFolders($folders) as folder (folder.id)}
          <option value={folder.id}>{"—".repeat(folder.depth)} {folder.name}</option>
        {/each}
      </select>
      <span class="sep"></span>
      <button class="icon-btn" class:active={note.isPinned} title="تثبيت" on:click={togglePin}>
        <Icon name="pin" />
      </button>
      <button class="icon-btn" class:active={note.isArchived} title="أرشفة" on:click={toggleArchive}>
        <Icon name="archive" />
      </button>
      <button
        class="icon-btn"
        title="قائمة تحقق"
        on:click={() => insertLinePrefix("- [ ] ")}><Icon name="checklist" /></button
      >
      <div class="menu-wrap" on:click|stopPropagation>
        <button class="icon-btn" class:active={formatOpen} title="تنسيق النص" on:click={() => (formatOpen = !formatOpen)}>Aa</button>
        {#if formatOpen}
          <div class="popover format">
            <button on:click={() => { wrapSelection("**"); formatOpen = false; }}>**عريض**</button>
            <button on:click={() => { wrapSelection("_"); formatOpen = false; }}>_مائل_</button>
            <button on:click={() => { wrapSelection("~~"); formatOpen = false; }}>~~يتوسطه خط~~</button>
            <button on:click={() => { insertLinePrefix("# "); formatOpen = false; }}>عنوان ١</button>
            <button on:click={() => { insertLinePrefix("## "); formatOpen = false; }}>عنوان ٢</button>
            <button on:click={() => { insertLinePrefix("- "); formatOpen = false; }}>قائمة نقطية</button>
            <button on:click={() => { insertLinePrefix("1. "); formatOpen = false; }}>قائمة مرقمة</button>
            <button on:click={() => { insertLinePrefix("> "); formatOpen = false; }}>اقتباس</button>
            <button on:click={() => { wrapSelection("`"); formatOpen = false; }}>`كود`</button>
            <button on:click={() => { insertText("---\n"); formatOpen = false; }}>فاصل</button>
            <button on:click={() => { insertDate(); formatOpen = false; }}>التاريخ والوقت</button>
          </div>
        {/if}
      </div>
      <div class="menu-wrap" on:click|stopPropagation>
        <button class="icon-btn" class:active={colorOpen} title="لون الملاحظة" on:click={() => (colorOpen = !colorOpen)}><Icon name="sun" /></button>
        {#if colorOpen}
          <div class="popover colors">
            {#each colors as color (color ?? "none")}
              <button
                class="swatch"
                class:selected={(note.color ?? null) === color}
                style={color ? `background:${color}` : ""}
                title={color ?? "بلا لون"}
                on:click={() => setColor(color)}
              >{color ? "" : "✕"}</button>
            {/each}
          </div>
        {/if}
      </div>
      <button
        class="icon-btn"
        class:active={preview}
        title={preview ? "تحرير النص (Ctrl+E)" : "معاينة Markdown منسّقة (Ctrl+E)"}
        on:click={togglePreview}
        >{preview ? "✎" : "👁"}</button
      >
      <button
        class="icon-btn"
        class:active={$alwaysOnTop}
        title={$alwaysOnTop ? "إلغاء البقاء فوق كل البرامج" : "البقاء فوق كل البرامج (Ctrl+Shift+T)"}
        on:click={() => store.setAlwaysOnTop(!$alwaysOnTop)}>📌</button
      >
      <button
        class="icon-btn"
        class:active={$compact}
        title="وضع الملاحظة البسيطة فوق كل شي (Ctrl+Shift+M)"
        on:click={() => store.setCompact(!$compact)}>🗗</button
      >
      <button
        class="icon-btn"
        title={note.isLocked ? "إدارة القفل" : "قفل الملاحظة"}
        class:active={note.isLocked}
        on:click={() => (lockMode = note!.isLocked ? "remove" : "lock")}><Icon name="lock" /></button
      >
      <button class="icon-btn" title="تاريخ الإصدارات" on:click={() => (versionsOpen = true)}>
        <Icon name="history" />
      </button>
      <span class="spacer"></span>
      <button class="icon-btn" class:active={findOpen} title="بحث (Ctrl+F)" on:click={() => (findOpen = !findOpen)}>
        <Icon name="search" />
      </button>
      <div class="menu-wrap" on:click|stopPropagation>
        <button class="icon-btn" class:active={moreOpen} title="المزيد" on:click={() => (moreOpen = !moreOpen)}>
          <Icon name="more" />
        </button>
        {#if moreOpen}
          <div class="popover more">
            <button on:click={() => { void saveAs(); moreOpen = false; }}>حفظ باسم…</button>
            <button on:click={() => exportNote("txt")}>تصدير إلى .txt</button>
            <button on:click={() => exportNote("md")}>تصدير إلى .md</button>
            <button on:click={() => { void duplicateNote(); moreOpen = false; }}>إنشاء نسخة</button>
            <button on:click={() => { void toggleArchive(); moreOpen = false; }}>
              {note.isArchived ? "إلغاء الأرشفة" : "أرشفة"}
            </button>
            <div class="separator"></div>
            <button on:click={() => { void store.openNote(note!.id); moreOpen = false; }}>تحديث</button>
          </div>
        {/if}
      </div>
    </div>

    {#if lockedView}
      <div class="locked">
        <div class="lock-card">
          <div class="lock-icon"><Icon name="lock" size={30} /></div>
          <h3>هذه الملاحظة مقفلة</h3>
          <p class="muted">أدخل كلمة المرور لعرض النص وتحريره.</p>
          <input
            type="password"
            placeholder="كلمة المرور"
            bind:value={unlockPassword}
            autocomplete="off"
            on:keydown={(event) => event.key === "Enter" && unlockNote()}
          />
          {#if unlockError}<p class="error">{unlockError}</p>{/if}
          <button class="btn primary" on:click={unlockNote} disabled={unlocking}>فتح</button>
          <button class="link danger" on:click={() => (lockMode = "remove")}>إزالة القفل نهائياً</button>
        </div>
      </div>
    {:else}
      <div class="head">
        <input
          class="title-input"
          value={note.title}
          placeholder="بدون عنوان"
          on:change={(event) => updateTitle((event.currentTarget as HTMLInputElement).value)}
          on:keydown={(event) => {
            if (event.key === "Enter") (event.currentTarget as HTMLInputElement).blur();
          }}
        />
        <div class="meta">
          <span>{arabicDateTime(note.updatedAt)}</span>
          {#if note.sourcePath}
            <span class="path" title={note.sourcePath}>📄 {note.sourcePath}</span>
          {/if}
          <span class="spacer"></span>
          <button class="chip" on:click={cycleDirection} title="تبديل اتجاه النص">
            {note.direction === "ltr" ? "LTR" : note.direction === "rtl" ? "RTL" : "اتجاه تلقائي"}
          </button>
          <button class="chip" class:on={wrap} on:click={toggleWrap} title="التفاف الأسطر">
            التفاف
          </button>
        </div>
        <div class="tags">
          {#each note.tags as tag (tag.id)}
            <span class="tag" style="border-color:{tag.color}; color:{tag.color}">
              #{tag.name}
              <button class="tag-x" title="إزالة الوسم" on:click={() => removeTag(tag.name)}>✕</button>
            </span>
          {/each}
          <input
            class="tag-input"
            placeholder="+ وسم"
            bind:value={tagDraft}
            on:keydown={(event) => {
              if (event.key === "Enter") addTag();
            }}
          />
        </div>
      </div>

      {#if trashed}
        <div class="banner trash-banner">
          <span>
            <Icon name="trash" size={14} />
            في «المحذوفة مؤخراً» — ستُحذف نهائياً بعد {plural(trashDaysLeft, NOUNS.day)}.
          </span>
          <button class="btn small" on:click={restoreFromTrash}>استعادة</button>
          <button class="btn small ghost" on:click={purgeForever}>حذف نهائي</button>
        </div>
      {/if}

      {#if externalChanged}
        <div class="banner">
          <span>⚠️ تغيّر الملف الموجود على القرص من خارج التطبيق.</span>
          <button class="btn small" on:click={reloadExternal}>إعادة التحميل</button>
          <button class="btn small ghost" on:click={discardExternal}>فصل الارتباط</button>
        </div>
      {/if}

      <div class="body">
        {#if preview}
          <div
            class="reader scroll"
            class:editing={readerEditing}
            dir={directionAttr}
            bind:this={readerEl}
            role="textbox"
            aria-multiline="true"
            aria-label="محرر المعاينة"
            tabindex="0"
            contenteditable={!trashed && !lockedView}
            spellcheck="false"
            on:click={onReaderClick}
            on:input={onReaderInput}
            on:paste={onReaderPaste}
            on:focus={() => (readerEditing = true)}
            on:blur={() => {
              readerEditing = false;
              if (dirty) void saveNow();
            }}
          >
            {#if content.trim()}
              {@html previewHtml}
            {:else}
              <p class="faint" contenteditable="false">لا يوجد نص للمعاينة بعد.</p>
            {/if}
          </div>
        {:else}
          <textarea
            bind:this={textarea}
            class="text"
            class:nowrap={!wrap}
            value={content}
            dir={directionAttr}
            wrap={wrap ? "soft" : "off"}
            spellcheck="false"
            readonly={trashed}
            placeholder="اكتب، أو اضغط «/» لإدراج عنصر"
            on:input={onTextareaInput}
            on:paste={onTextareaPaste}
            on:keydown={onTextareaKeydown}
            on:wheel={onWheel}
            on:blur={() => dirty && void saveNow()}
          ></textarea>

          {#if slashOpen}
            <SlashMenu query={slashQuery} top={slashPos.top} left={slashPos.left} on:choose={chooseSlash} />
          {/if}
        {/if}
      </div>

      {#if findOpen}
        <FindReplace
          bind:find={findText}
          bind:replace={replaceText}
          bind:showReplace
          bind:useRegex
          bind:caseSensitive
          matchCount={matches.length}
          error={findError}
          on:findNext={() => findNext(false)}
          on:findPrev={() => findNext(true)}
          on:replaceOne={replaceOne}
          on:replaceAll={replaceAll}
          on:close={() => (findOpen = false)}
          on:changed={() => {
            /* match count is reactive */
          }}
        />
      {/if}

      <StatusBar {content} {note} {wrap} />
    {/if}
  {/if}
</section>

{#if gotoOpen}
  <Modal title="الذهاب إلى سطر" on:close={() => (gotoOpen = false)}>
    <div class="field">
      <label for="goto-line">رقم السطر (من ١ إلى {content.split("\n").length})</label>
      <input id="goto-line" type="number" min="1" bind:value={gotoValue} on:keydown={(event) => event.key === "Enter" && goToLine()} />
    </div>
    <svelte:fragment slot="footer">
      <button class="btn primary" on:click={goToLine}>انتقال</button>
      <button class="btn ghost" on:click={() => (gotoOpen = false)}>إلغاء</button>
    </svelte:fragment>
  </Modal>
{/if}

{#if lockMode && note}
  <LockDialog
    {note}
    mode={lockMode}
    on:close={() => (lockMode = null)}
    on:saved={(event) => {
      passwords = withoutKey(passwords, note!.id);
      applySavedNote(event.detail);
      lockMode = null;
    }}
  />
{/if}

{#if versionsOpen && note}
  <VersionHistory
    noteId={note.id}
    on:close={() => (versionsOpen = false)}
    on:restored={(event) => {
      applySavedNote(event.detail);
      versionsOpen = false;
    }}
  />
{/if}

<style>
  .editor-pane {
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg);
    position: relative;
  }

  /* Sticky-note mode: no tab strip, no folder picker — just the note. */
  .editor-pane.compact .tabs,
  .editor-pane.compact .move {
    display: none;
  }

  .tabs {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 6px 8px 0;
    background: var(--bg-elevated);
    border-bottom: 1px solid var(--border);
    min-height: 40px;
  }

  .tab-strip {
    display: flex;
    gap: 3px;
    overflow-x: auto;
    scrollbar-width: none;
    flex: 1;
  }

  .tab-strip::-webkit-scrollbar {
    display: none;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 2px;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-bottom: none;
    border-radius: 8px 8px 0 0;
    max-width: 190px;
    flex-shrink: 0;
  }

  .tab.active {
    background: var(--bg);
    border-color: var(--border-strong);
  }

  .tab-label {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 8px;
    font-size: 0.82em;
    color: var(--text-muted);
    max-width: 160px;
  }

  .tab.active .tab-label {
    color: var(--text);
  }

  .tiny {
    font-size: 0.8em;
  }

  .ellipsis {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tab-close {
    padding: 4px 6px;
    color: var(--text-faint);
    font-size: 0.7em;
  }

  .tab-close:hover {
    color: var(--danger);
  }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 6px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-elevated);
    flex-wrap: wrap;
  }

  .move {
    font-size: 0.78em;
    padding: 4px 6px;
    max-width: 138px;
  }

  .sep {
    width: 1px;
    height: 18px;
    background: var(--border);
    margin: 0 4px;
  }

  .menu-wrap {
    position: relative;
  }

  .popover {
    position: absolute;
    top: 34px;
    inset-inline-end: 0;
    z-index: 45;
    background: var(--bg-raised);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-sm);
    box-shadow: var(--shadow);
    padding: 5px;
    min-width: 160px;
    display: flex;
    flex-direction: column;
  }

  .popover button {
    text-align: right;
    padding: 7px 10px;
    border-radius: 5px;
    font-size: 0.85em;
  }

  .popover button:hover {
    background: var(--bg-hover);
  }

  .popover.format {
    min-width: 180px;
  }

  .popover.colors {
    flex-direction: row;
    flex-wrap: wrap;
    min-width: 0;
    width: 168px;
    gap: 6px;
  }

  .swatch {
    width: 26px;
    height: 26px;
    border-radius: 7px;
    border: 2px solid transparent;
    background: var(--bg-hover);
    display: grid;
    place-items: center;
    font-size: 0.7em;
    color: var(--text-muted);
  }

  .swatch.selected {
    border-color: var(--text);
  }

  .head {
    padding: 12px 20px 10px;
    border-bottom: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .title-input {
    border: none;
    background: none;
    padding: 0;
    font-size: 1.22em;
    font-weight: 700;
    color: var(--text);
  }

  .title-input:focus {
    border: none;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 10px;
    color: var(--text-faint);
    font-size: 0.74em;
    flex-wrap: wrap;
  }

  .path {
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    direction: ltr;
  }

  .chip {
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 999px;
    padding: 1px 9px;
    font-size: 0.95em;
    color: var(--text-muted);
  }

  .chip.on {
    border-color: var(--accent);
    color: var(--accent);
  }

  .tags {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-wrap: wrap;
  }

  .tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    border: 1px solid var(--border-strong);
    border-radius: 999px;
    padding: 0 8px;
    font-size: 0.72em;
  }

  .tag-x {
    font-size: 0.85em;
    opacity: 0.6;
  }

  .tag-x:hover {
    opacity: 1;
  }

  .tag-input {
    width: 88px;
    border: none;
    background: none;
    padding: 1px 4px;
    font-size: 0.74em;
    color: var(--text-muted);
  }

  .tag-input:focus {
    border: none;
  }

  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 18px;
    background: var(--accent-soft);
    color: var(--accent);
    font-size: 0.8em;
    border-bottom: 1px solid var(--border);
  }

  .banner span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }

  .trash-banner {
    background: rgba(224, 122, 95, 0.12);
    color: var(--danger);
  }

  .icon-btn:disabled {
    opacity: 0.35;
    cursor: not-allowed;
  }

  .tiny {
    display: grid;
    place-items: center;
  }

  .btn.small {
    font-size: 0.78em;
    padding: 3px 10px;
  }

  .body {
    flex: 1;
    position: relative;
    display: flex;
    min-height: 0;
  }

  /* Rendered Markdown preview */
  .reader {
    flex: 1;
    min-height: 0;
    padding: 24px 34px 70px;
    background: var(--bg);
    line-height: 1.95;
    font-size: 1.02em;
    overflow-wrap: anywhere;
  }

  .reader[contenteditable="true"] {
    outline: none;
    cursor: text;
  }

  /* Image controls stay clickable while the preview is editable. */
  .reader :global(.md-image) {
    user-select: none;
  }

  .reader :global(h1),
  .reader :global(h2),
  .reader :global(h3),
  .reader :global(h4),
  .reader :global(h5),
  .reader :global(h6) {
    margin: 1.4em 0 0.5em;
    line-height: 1.5;
  }

  .reader :global(h1) {
    font-size: 1.7em;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.25em;
  }

  .reader :global(h2) {
    font-size: 1.38em;
  }

  .reader :global(h3) {
    font-size: 1.18em;
  }

  .reader :global(p) {
    margin: 0 0 0.9em;
  }

  /* Images: pasted screenshots plus zoom/rotate controls under each one. */
  .reader :global(.md-image) {
    margin: 1.1em 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }

  .reader :global(.md-image-frame) {
    display: flex;
    align-items: center;
    justify-content: center;
    max-width: 100%;
  }

  /* An explicit zoom width may exceed the pane; the reader scrolls sideways. */
  .reader :global(.md-image[data-w]:not([data-w="0"]) .md-image-frame),
  .reader :global(.md-image[data-w]:not([data-w="0"]) img) {
    max-width: none;
  }

  .reader :global(.md-image img) {
    display: block;
    max-width: 100%;
    height: auto;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg-hover);
  }

  .reader :global(.md-image-tools) {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg-hover);
    font-size: 0.74em;
    color: var(--text-faint);
    opacity: 0.7;
    transition: opacity 0.15s ease;
  }

  .reader :global(.md-image):hover :global(.md-image-tools) {
    opacity: 1;
  }

  .reader :global(.md-image-tools button) {
    background: transparent;
    border: 0;
    color: var(--text-muted, var(--text-faint));
    cursor: pointer;
    padding: 1px 7px;
    border-radius: 999px;
    font-family: inherit;
    font-size: 1em;
    line-height: 1.7;
  }

  .reader :global(.md-image-tools button:hover) {
    background: var(--border);
    color: var(--text);
  }

  .reader :global(.md-image-zoom) {
    min-width: 46px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .reader :global(a) {
    color: var(--accent);
    text-decoration: underline;
    text-underline-offset: 3px;
  }

  .reader :global(strong) {
    color: var(--text);
    font-weight: 700;
  }

  .reader :global(em) {
    color: var(--text-muted);
  }

  .reader :global(del) {
    color: var(--text-faint);
  }

  .reader :global(ul),
  .reader :global(ol) {
    margin: 0 0 0.9em;
    padding-inline-start: 1.5em;
  }

  .reader :global(li) {
    margin: 0.25em 0;
  }

  .reader :global(li.md-nested) {
    margin-inline-start: 1.3em;
    list-style-type: circle;
  }

  .reader :global(li.md-task) {
    list-style: none;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    margin-inline-start: -1.1em;
  }

  .reader :global(.md-check) {
    width: 17px;
    height: 17px;
    border-radius: 5px;
    border: 1.5px solid var(--border-strong);
    display: inline-grid;
    place-items: center;
    font-size: 0.7em;
    margin-top: 6px;
    flex: 0 0 auto;
  }

  .reader :global(.md-check.on) {
    background: var(--success);
    border-color: var(--success);
    color: #10240f;
  }

  .reader :global(.md-task-text.done) {
    color: var(--text-faint);
    text-decoration: line-through;
  }

  .reader :global(blockquote) {
    margin: 0 0 1em;
    padding: 8px 14px;
    border-inline-start: 3px solid var(--accent);
    background: var(--bg-elevated);
    border-radius: var(--radius-sm);
    color: var(--text-muted);
  }

  .reader :global(hr) {
    border: none;
    border-top: 1px solid var(--border);
    margin: 1.6em 0;
  }

  .reader :global(code) {
    direction: ltr;
    unicode-bidi: embed;
    display: inline-block;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 0 5px;
    font-family: "Cascadia Code", Consolas, monospace;
    font-size: 0.86em;
  }

  .reader :global(.md-code-block) {
    position: relative;
    margin: 0 0 1em;
  }

  .reader :global(.md-copy) {
    position: absolute;
    top: 7px;
    left: 8px;
    z-index: 2;
    padding: 3px 10px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--bg-elevated);
    color: var(--text-muted);
    font-size: 0.72em;
    cursor: pointer;
  }

  .reader :global(.md-copy:hover) {
    color: var(--accent);
    border-color: var(--accent);
  }

  .reader :global(.md-code-block pre.md-code) {
    margin: 0;
  }

  .reader :global(pre.md-code) {
    direction: ltr;
    text-align: left;
    background: var(--bg-raised);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    padding: 14px 16px;
    overflow-x: auto;
    margin: 0 0 1em;
    line-height: 1.65;
  }

  .reader :global(pre.md-code code) {
    display: block;
    background: none;
    border: none;
    padding: 0;
    font-size: 0.88em;
    white-space: pre;
  }

  .reader :global(.md-table-wrap) {
    overflow-x: auto;
    margin: 0 0 1em;
  }

  .reader :global(table) {
    border-collapse: collapse;
    width: 100%;
    font-size: 0.92em;
  }

  .reader :global(th),
  .reader :global(td) {
    border: 1px solid var(--border);
    padding: 7px 11px;
  }

  .reader :global(th) {
    background: var(--bg-elevated);
    font-weight: 700;
  }

  .reader :global(img) {
    max-width: 100%;
    border-radius: var(--radius-sm);
  }

  .text {
    flex: 1;
    width: 100%;
    border: none;
    border-radius: 0;
    background: var(--bg);
    padding: 18px 24px 28px;
    resize: none;
    line-height: 1.9;
    font-family: var(--font-family);
    font-size: 1em;
    caret-color: var(--accent);
  }

  .text:focus {
    border: none;
  }

  .text.nowrap {
    white-space: pre;
    overflow-x: auto;
  }

  .empty.click-target {
    cursor: pointer;
  }

  .empty.click-target:hover {
    background: color-mix(in srgb, var(--bg-hover) 45%, transparent);
  }

  .empty {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    text-align: center;
    padding: 24px;
  }

  .empty-mark {
    font-size: 2.6em;
    opacity: 0.5;
  }

  .empty h3 {
    margin: 4px 0 0;
    font-size: 1.05em;
  }

  .empty p {
    margin: 0 0 10px;
    font-size: 0.86em;
  }

  .locked {
    flex: 1;
    display: grid;
    place-items: center;
    padding: 24px;
  }

  .lock-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    text-align: center;
    background: var(--bg-elevated);
    border: 1px solid var(--border);
    border-radius: 14px;
    padding: 26px 30px;
    width: min(360px, 88%);
  }

  .lock-icon {
    font-size: 2em;
  }

  .lock-card h3 {
    margin: 0;
    font-size: 1.02em;
  }

  .lock-card p {
    margin: 0;
    font-size: 0.84em;
  }

  .lock-card input {
    width: 100%;
    text-align: center;
  }

  .link {
    font-size: 0.78em;
    color: var(--text-muted);
    text-decoration: underline;
  }

  .link.danger {
    color: var(--danger);
  }

  .error {
    margin: 0;
    color: var(--danger);
    font-size: 0.78em;
  }
</style>
