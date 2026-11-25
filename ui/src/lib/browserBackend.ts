/**
 * Browser preview backend.
 *
 * The production backend is Rust (Tauri IPC). This module lets the very same
 * UI run inside a plain browser so the whole app can be reviewed before a CI
 * build: it installs a `window.__TAURI_INTERNALS__` stub whose `invoke`
 * answers the same command names with the same payloads, backed by
 * localStorage instead of SQLite.
 *
 * `installBrowserBackend()` is a no-op when the real Tauri runtime is present,
 * so the desktop app never sees this code.
 */

import type {
  AiConnection,
  AiConnectionDraft,
  BackupEntry,
  Category,
  CategoryTotal,
  Counts,
  DataInfo,
  DayTotal,
  Folder,
  FolderNode,
  ImportStats,
  MonthlySummary,
  Note,
  NoteSummary,
  NoteVersion,
  OpenedFile,
  SearchHit,
  Snippet,
  Tag,
  TaskItem,
  TransactionItem,
} from "./types";

type Args = Record<string, any>;

const STORAGE_KEY = "noterust.preview.v1";

interface PreviewState {
  notes: Note[];
  folders: Folder[];
  tags: Tag[];
  tasks: TaskItem[];
  snippets: Snippet[];
  aiConnections: AiConnection[];
  categories: Category[];
  transactions: TransactionItem[];
  settings: Record<string, string>;
  versions: NoteVersion[];
  board: string | null;
  htmlLab: string | null;
  attachments: Record<string, string>;
  trash: Record<string, string>;
  nextId: number;
}

/** True when the app runs inside the real Tauri webview. */
export function isTauriRuntime(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window && !previewMode;
}

/** True when the UI runs on the browser preview backend. */
export function isPreviewMode(): boolean {
  return previewMode;
}

/** Set once the preview stub replaced the real Tauri internals. */
let previewMode = false;

const now = () => Date.now();
const DAY = 24 * 60 * 60 * 1000;

// ---------------------------------------------------------------------------
// State
// ---------------------------------------------------------------------------

function seedState(): PreviewState {
  const t = now();
  const day = 24 * 60 * 60 * 1000;
  let nextId = 1;
  const id = () => nextId++;

  const folders: Folder[] = [
    { id: id(), parentId: null, name: "شغل", icon: "💼", color: "#6b7cff", sortOrder: 0, createdAt: t - 30 * day },
    { id: id(), parentId: null, name: "شخصي", icon: "🏠", color: "#22a06b", sortOrder: 1, createdAt: t - 30 * day },
  ];
  const workId = folders[0].id;
  const personalId = folders[1].id;
  folders.push({ id: id(), parentId: workId, name: "أفكار", icon: "💡", color: "#f5a524", sortOrder: 0, createdAt: t - 20 * day });

  const tags: Tag[] = [
    { id: id(), name: "مهم", color: "#e5484d" },
    { id: id(), name: "شغل", color: "#6b7cff" },
    { id: id(), name: "أفكار", color: "#f5a524" },
  ];
  const [important, work, ideas] = tags;

  const makeNote = (
    folderId: number | null,
    title: string,
    content: string,
    extra: Partial<Note> = {},
  ): Note => ({
    id: id(),
    folderId,
    title,
    content,
    contentFormat: "markdown",
    isPinned: false,
    isArchived: false,
    isLocked: false,
    color: null,
    direction: "auto",
    sourcePath: null,
    encoding: null,
    lineEnding: null,
    createdAt: t - Math.floor(Math.random() * 20) * day - day,
    updatedAt: t - Math.floor(Math.random() * 5) * day,
    deletedAt: null,
    tags: [],
    ...extra,
  });

  const notes: Note[] = [
    makeNote(
      workId,
      "تقرير الأسبوع",
      [
        "# تقرير الأسبوع",
        "",
        "أهم الإنجازات ده الأسبوع:",
        "",
        "- إنهاء **تصميم لوحة التحكم**",
        "- مراجعة الكود مع الفريق",
        "- إضافة اختبارات للمزامنة",
        "",
        "> ملاحظة: الموعد النهائي يوم الخميس.",
        "",
        "```bash",
        "cargo check --offline",
        "npm run build",
        "```",
      ].join("\n"),
      { isPinned: true, tags: [important, work] },
    ),
    makeNote(
      folders[2].id,
      "أفكار مشروع جديد",
      [
        "## فكرة",
        "",
        "تطبيق بسيط يجمع ملاحظات يومية في مكان واحد.",
        "",
        "- واجهة عربية بالكامل",
        "- تخزين محلي بدون حساب",
        "- تصدير Markdown بضغطة واحدة",
      ].join("\n"),
      { tags: [ideas] },
    ),
    makeNote(
      personalId,
      "وصفة كيك",
      [
        "# كيك الشوكولاتة",
        "",
        "| المادة | الكمية |",
        "| --- | --- |",
        "| دقيق | 2 كوب |",
        "| سكر | 1 كوب |",
        "| بيض | 2 |",
        "",
        "1. اخلط الجاف",
        "2. أضف البيض",
        "3. اخبز 35 دقيقة",
      ].join("\n"),
    ),
    makeNote(
      personalId,
      "مصروفات الشهر",
      "الإيجار 2500 جنيه والمواصلات 400 جنيه وأكل 1200 جنيه.",
      { tags: [important] },
    ),
    makeNote(
      workId,
      "ملاحظات القراءة",
      [
        "## كتاب: قواعد التيسير",
        "",
        "1. ابدأ بالأصعب",
        "2. التفاصيل الصغيرة",
        "3. الشحن مرة واحدة",
      ].join("\n"),
      { isArchived: true },
    ),
    makeNote(
      workId,
      "اجتماع الفريق",
      "قرارات الاجتماع:\n\n- تأجيل إطلاق النسخة الأولى أسبوع\n- توزيع المهام: أحمد على الواجهة، سارة على الـ API",      { deletedAt: t - day },
    ),
  ];

  const tasks: TaskItem[] = [
    {
      id: id(),
      noteId: notes[0].id,
      noteTitle: notes[0].title,
      title: "تجهيز العرض التقديمي",
      description: "شرائح مختصرة + الأرقام",
      dueAt: t + 2 * day,
      priority: 2,
      isDone: false,
      repeatRule: null,
      createdAt: t - 3 * day,
      completedAt: null,
      fromChecklist: false,
    },
    {
      id: id(),
      noteId: null,
      noteTitle: null,
      title: "مراجعة كود الفوترة",
      description: "",
      dueAt: t + 5 * day,
      priority: 1,
      isDone: false,
      repeatRule: null,
      createdAt: t - day,
      completedAt: null,
      fromChecklist: false,
    },
    {
      id: id(),
      noteId: null,
      noteTitle: null,
      title: "شراء مستلزمات المكتب",
      description: "",
      dueAt: null,
      priority: 0,
      isDone: true,
      repeatRule: null,
      createdAt: t - 6 * day,
      completedAt: t - 2 * day,
      fromChecklist: false,
    },
  ];

  const snippets: Snippet[] = [
    {
      id: id(),
      title: "تشغيل السيرفر محلياً",
      kind: "code",
      language: "bash",
      content: "npm run dev\ncargo check --offline",
      tags: "dev,shell",
      isPinned: true,
      createdAt: t - 10 * day,
      updatedAt: t - 2 * day,
    },
    {
      id: id(),
      title: "prompt تلخيص نص",
      kind: "prompt",
      language: "",
      content: "لخّص النص التالي في 3 نقاط عملية:\n\n",
      tags: "ai",
      isPinned: false,
      createdAt: t - 8 * day,
      updatedAt: t - 8 * day,
    },
  ];

  const categories: Category[] = [
    { id: id(), name: "إيجار", kind: "expense", color: "#e5484d" },
    { id: id(), name: "أكل", kind: "expense", color: "#f5a524" },
    { id: id(), name: "مواصلات", kind: "expense", color: "#6b7cff" },
    { id: id(), name: "مرتب", kind: "income", color: "#22a06b" },
    { id: id(), name: "مشروع", kind: "income", color: "#8b5cf6" },
  ];

  const month = new Date();
  const inMonth = (dayOfMonth: number) =>
    new Date(month.getFullYear(), month.getMonth(), dayOfMonth, 12, 0, 0).getTime();

  const transactions: TransactionItem[] = [
    {
      id: id(),
      kind: "income",
      amountMinor: 1200000,
      currency: "EGP",
      categoryId: categories[3].id,
      categoryName: categories[3].name,
      categoryColor: categories[3].color,
      note: "مرتب الشهر",
      occurredAt: inMonth(1),
    },
    {
      id: id(),
      kind: "expense",
      amountMinor: 250000,
      currency: "EGP",
      categoryId: categories[0].id,
      categoryName: categories[0].name,
      categoryColor: categories[0].color,
      note: "إيجار",
      occurredAt: inMonth(3),
    },
    {
      id: id(),
      kind: "expense",
      amountMinor: 42000,
      currency: "EGP",
      categoryId: categories[1].id,
      categoryName: categories[1].name,
      categoryColor: categories[1].color,
      note: "أكل",
      occurredAt: inMonth(7),
    },
    {
      id: id(),
      kind: "expense",
      amountMinor: 18500,
      currency: "EGP",
      categoryId: categories[2].id,
      categoryName: categories[2].name,
      categoryColor: categories[2].color,
      note: "مواصلات",
      occurredAt: inMonth(11),
    },
    {
      id: id(),
      kind: "income",
      amountMinor: 350000,
      currency: "EGP",
      categoryId: categories[4].id,
      categoryName: categories[4].name,
      categoryColor: categories[4].color,
      note: "مشروع جانبي",
      occurredAt: inMonth(14),
    },
  ];

  const aiConnections: AiConnection[] = [
    {
      id: id(),
      name: "OpenAI",
      kind: "openai",
      baseUrl: "https://api.openai.com/v1",
      apiKey: "sk-preview",
      model: "gpt-4o-mini",
      createdAt: t - 12 * day,
      updatedAt: t - 12 * day,
    },
  ];

  const versions: NoteVersion[] = [
    {
      id: id(),
      noteId: notes[0].id,
      content: "# تقرير الأسبوع\n\n- المسودة الأولى",
      title: notes[0].title,
      savedAt: t - 4 * day,
    },
  ];

  return {
    notes,
    folders,
    tags,
    tasks,
    snippets,
    aiConnections,
    categories,
    transactions,
    settings: { theme: "dark", font_family: "Cairo", font_size: "16", currency: "EGP" },
    versions,
    board: null,
    htmlLab: null,
    attachments: {},
    trash: {},
    nextId,
  };
}

function loadState(): PreviewState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw) as PreviewState;
  } catch {
    /* fall through to a fresh seed */
  }
  const seeded = seedState();
  saveState(seeded);
  return seeded;
}

function saveState(next: PreviewState = state) {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  } catch {
    /* storage full or blocked — the preview still works in memory */
  }
}

let state: PreviewState = loadState();

/** Writes the state back to localStorage. */
function commit() {
  saveState(state);
}

function nextId(): number {
  const value = state.nextId;
  state.nextId += 1;
  return value;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function stripMarkdown(text: string): string {
  return text
    .replace(/```[\s\S]*?```/g, " ")
    .replace(/`([^`]*)`/g, "$1")
    .replace(/!\[[^\]]*\]\([^)]*\)/g, " [صورة] ")
    .replace(/\[([^\]]*)\]\([^)]*\)/g, "$1")
    .replace(/^[>#\-*+\s]+/gm, "")
    .replace(/[*_~]/g, "")
    .replace(/\s+/g, " ")
    .trim();
}

function summary(note: Note): NoteSummary {
  return {
    id: note.id,
    folderId: note.folderId,
    title: note.title,
    preview: stripMarkdown(note.content).slice(0, 160),
    isPinned: note.isPinned,
    isLocked: note.isLocked,
    isArchived: note.isArchived,
    color: note.color,
    createdAt: note.createdAt,
    updatedAt: note.updatedAt,
    wordCount: note.content.trim() ? note.content.trim().split(/\s+/).length : 0,
    tags: note.tags,
  };
}

function findNote(id: number): Note {
  const note = state.notes.find((item) => item.id === id);
  if (!note) throw new Error("الملاحظة غير موجودة");
  return note;
}

function folderTree(): FolderNode[] {
  const build = (parentId: number | null): FolderNode[] =>
    state.folders
      .filter((folder) => folder.parentId === parentId)
      .sort((a, b) => a.sortOrder - b.sortOrder || a.id - b.id)
      .map((folder) => ({
        ...folder,
        noteCount: state.notes.filter((note) => note.folderId === folder.id && !note.deletedAt).length,
        children: build(folder.id),
      }));
  return build(null);
}

function tagsOf(note: Note): Tag[] {
  return note.tags ?? [];
}

function resolveTags(names: string[]): Tag[] {
  const result: Tag[] = [];
  for (const raw of names ?? []) {
    const name = String(raw).trim();
    if (!name) continue;
    let tag = state.tags.find((item) => item.name === name);
    if (!tag) {
      tag = { id: nextId(), name, color: "#8b8b96" };
      state.tags.push(tag);
    }
    if (!result.some((item) => item.id === tag!.id)) result.push(tag);
  }
  return result;
}

function touch(note: Note) {
  note.updatedAt = now();
}

function pushVersion(note: Note) {
  state.versions.unshift({
    id: nextId(),
    noteId: note.id,
    content: note.content,
    title: note.title,
    savedAt: now(),
  });
  if (state.versions.length > 60) state.versions.length = 60;
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

function listNotes(args: Args): NoteSummary[] {
  const query = args.query ?? {};
  const folderId = query.folderId ?? null;
  const trash = Boolean(query.trash);
  const includeArchived = Boolean(query.includeArchived);
  const sort = String(query.sort ?? "updated");
  const limit = Number(query.limit ?? 1000);

  let list = state.notes.filter((note) => (trash ? note.deletedAt !== null : note.deletedAt === null));
  if (!trash && !includeArchived) list = list.filter((note) => !note.isArchived);
  if (folderId !== null && folderId !== undefined) {
    list = list.filter((note) => note.folderId === folderId);
  }

  if (sort === "created") list = list.sort((a, b) => b.createdAt - a.createdAt);
  else if (sort === "title") list = list.sort((a, b) => a.title.localeCompare(b.title, "ar"));
  else if (sort === "oldest") list = list.sort((a, b) => a.updatedAt - b.updatedAt);
  else list = list.sort((a, b) => b.updatedAt - a.updatedAt);

  return list.slice(0, limit).map(summary);
}

function searchNotes(args: Args): SearchHit[] {
  const query = String(args.query ?? "").trim().toLowerCase();
  const folderId = args.folderId ?? null;
  const limit = Number(args.limit ?? 200);
  if (!query) return [];
  const hits: SearchHit[] = [];
  for (const note of state.notes) {
    if (note.deletedAt || note.isArchived) continue;
    if (folderId !== null && folderId !== undefined && note.folderId !== folderId) continue;
    const haystack = `${note.title}\n${note.content}`.toLowerCase();
    if (!haystack.includes(query)) continue;
    const index = haystack.indexOf(query);
    const plain = stripMarkdown(note.content);
    hits.push({
      ...summary(note),
      snippet: plain.slice(Math.max(0, index - 40), index + 90),
    });
    if (hits.length >= limit) break;
  }
  return hits;
}

function counts(): Counts {
  const live = state.notes.filter((note) => !note.deletedAt);
  return {
    all: live.filter((note) => !note.isArchived).length,
    trash: state.notes.filter((note) => note.deletedAt).length,
    archived: live.filter((note) => note.isArchived).length,
    tasksOpen: state.tasks.filter((task) => !task.isDone).length,
  };
}

function monthlySummary(args: Args): MonthlySummary {
  const year = Number(args.year);
  const month = Number(args.month);
  const inMonth = state.transactions.filter((item) => {
    const date = new Date(item.occurredAt);
    return date.getFullYear() === year && date.getMonth() + 1 === month;
  });
  const incomeMinor = inMonth
    .filter((item) => item.kind === "income")
    .reduce((sum, item) => sum + item.amountMinor, 0);
  const expenseMinor = inMonth
    .filter((item) => item.kind === "expense")
    .reduce((sum, item) => sum + item.amountMinor, 0);

  const byCategoryMap = new Map<number | null, CategoryTotal>();
  for (const item of inMonth) {
    const key = item.categoryId ?? null;
    const entry = byCategoryMap.get(key) ?? {
      categoryId: key,
      name: item.categoryName ?? "بدون فئة",
      kind: item.kind,
      color: item.categoryColor ?? "#8b8b96",
      totalMinor: 0,
    };
    entry.totalMinor += item.amountMinor;
    byCategoryMap.set(key, entry);
  }

  const byDayMap = new Map<number, DayTotal>();
  for (const item of inMonth) {
    const day = new Date(item.occurredAt).getDate();
    const entry = byDayMap.get(day) ?? { day, incomeMinor: 0, expenseMinor: 0 };
    if (item.kind === "income") entry.incomeMinor += item.amountMinor;
    else entry.expenseMinor += item.amountMinor;
    byDayMap.set(day, entry);
  }

  return {
    year,
    month,
    incomeMinor,
    expenseMinor,
    balanceMinor: incomeMinor - expenseMinor,
    currency: "EGP",
    byCategory: [...byCategoryMap.values()].sort((a, b) => b.totalMinor - a.totalMinor),
    byDay: [...byDayMap.values()].sort((a, b) => a.day - b.day),
  };
}

function transactionsCsv(args: Args): string {
  const year = Number(args.year);
  const month = Number(args.month);
  const rows = state.transactions
    .filter((item) => {
      const date = new Date(item.occurredAt);
      return date.getFullYear() === year && date.getMonth() + 1 === month;
    })
    .map((item) => `${item.kind},${(item.amountMinor / 100).toFixed(2)},${item.categoryName ?? ""},${item.note ?? ""}`);
  return ["type,amount,category,note", ...rows].join("\n");
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

function handle(command: string, args: Args): unknown {
  switch (command) {
    // Notes ---------------------------------------------------------------
    case "list_notes":
      return listNotes(args);
    case "get_note":
      return findNote(Number(args.id));
    case "create_note": {
      const note: Note = {
        id: nextId(),
        folderId: args.folderId ?? null,
        title: "ملاحظة جديدة",
        content: "",
        contentFormat: "markdown",
        isPinned: false,
        isArchived: false,
        isLocked: false,
        color: null,
        direction: "auto",
        sourcePath: null,
        encoding: null,
        lineEnding: null,
        createdAt: now(),
        updatedAt: now(),
        deletedAt: null,
        tags: [],
      };
      state.notes.push(note);
      commit();
      return note;
    }
    case "update_note_content": {
      const note = findNote(Number(args.id));
      note.content = String(args.content ?? "");
      touch(note);
      pushVersion(note);
      commit();
      return note;
    }
    case "update_note_title": {
      const note = findNote(Number(args.id));
      note.title = String(args.title ?? "");
      touch(note);
      commit();
      return note;
    }
    case "move_note": {
      const note = findNote(Number(args.id));
      note.folderId = args.folderId ?? null;
      touch(note);
      commit();
      return note;
    }
    case "pin_note": {
      const note = findNote(Number(args.id));
      note.isPinned = Boolean(args.pinned);
      commit();
      return note;
    }
    case "archive_note": {
      const note = findNote(Number(args.id));
      note.isArchived = Boolean(args.archived);
      commit();
      return note;
    }
    case "set_note_color": {
      const note = findNote(Number(args.id));
      note.color = args.color ?? null;
      commit();
      return note;
    }
    case "set_note_direction": {
      const note = findNote(Number(args.id));
      note.direction = args.direction ?? "auto";
      commit();
      return note;
    }
    case "delete_note": {
      const note = findNote(Number(args.id));
      note.deletedAt = now();
      commit();
      return null;
    }
    case "restore_note": {
      const note = findNote(Number(args.id));
      note.deletedAt = null;
      commit();
      return note;
    }
    case "purge_note": {
      state.notes = state.notes.filter((note) => note.id !== Number(args.id));
      commit();
      return null;
    }
    case "empty_trash": {
      const before = state.notes.length;
      state.notes = state.notes.filter((note) => !note.deletedAt);
      commit();
      return before - state.notes.length;
    }
    case "duplicate_note": {
      const source = findNote(Number(args.id));
      const copy: Note = {
        ...source,
        id: nextId(),
        title: `${source.title} (نسخة)`,
        createdAt: now(),
        updatedAt: now(),
        deletedAt: null,
      };
      state.notes.push(copy);
      commit();
      return copy;
    }
    case "search_notes":
      return searchNotes(args);
    case "counts":
      return counts();
    case "set_note_tags": {
      const note = findNote(Number(args.id));
      note.tags = resolveTags(args.tags ?? []);
      commit();
      return note.tags;
    }
    case "list_tags":
      return state.tags.map((tag) => ({
        ...tag,
        count: state.notes.filter((note) => !note.deletedAt && tagsOf(note).some((item) => item.id === tag.id))
          .length,
      }));
    case "delete_tag": {
      const id = Number(args.id);
      state.tags = state.tags.filter((tag) => tag.id !== id);
      for (const note of state.notes) note.tags = tagsOf(note).filter((tag) => tag.id !== id);
      commit();
      return null;
    }
    case "note_versions":
      return state.versions.filter((version) => version.noteId === Number(args.id));
    case "restore_version": {
      const version = state.versions.find((item) => item.id === Number(args.versionId));
      if (!version) throw new Error("النسخة غير موجودة");
      const note = findNote(version.noteId);
      note.content = version.content;
      touch(note);
      commit();
      return note;
    }
    case "lock_note": {
      const note = findNote(Number(args.id));
      note.isLocked = true;
      commit();
      return note;
    }
    case "unlock_note": {
      const note = findNote(Number(args.id));
      if (!note.isLocked) throw new Error("الملاحظة غير مقفلة");
      return note.content;
    }
    case "save_locked_note": {
      const note = findNote(Number(args.id));
      note.content = String(args.content ?? "");
      touch(note);
      commit();
      return note;
    }
    case "remove_lock": {
      const note = findNote(Number(args.id));
      note.isLocked = false;
      commit();
      return note;
    }

    // Folders -------------------------------------------------------------
    case "list_folders":
      return folderTree();
    case "create_folder": {
      const folder: Folder = {
        id: nextId(),
        parentId: args.parentId ?? null,
        name: String(args.name ?? "فولدر"),
        icon: args.icon ?? "📁",
        color: args.color ?? "#6b7cff",
        sortOrder: state.folders.filter((item) => item.parentId === (args.parentId ?? null)).length,
        createdAt: now(),
      };
      state.folders.push(folder);
      commit();
      return folder;
    }
    case "update_folder": {
      const folder = state.folders.find((item) => item.id === Number(args.id));
      if (!folder) throw new Error("الفولدر غير موجود");
      if (args.name !== undefined) folder.name = String(args.name);
      if (args.icon !== undefined) folder.icon = String(args.icon);
      if (args.color !== undefined) folder.color = String(args.color);
      if (args.parentId !== undefined) folder.parentId = args.parentId ?? null;
      commit();
      return folder;
    }
    case "delete_folder": {
      const id = Number(args.id);
      state.folders = state.folders.filter((folder) => folder.id !== id && folder.parentId !== id);
      for (const note of state.notes) {
        if (note.folderId === id) note.folderId = null;
      }
      commit();
      return null;
    }
    case "reorder_folder": {
      const folder = state.folders.find((item) => item.id === Number(args.id));
      if (folder) {
        folder.sortOrder = Number(args.index ?? folder.sortOrder);
        commit();
      }
      return null;
    }

    // Tasks ---------------------------------------------------------------
    case "list_tasks":
      return state.tasks.filter((task) => args.includeDone || !task.isDone);
    case "create_task": {
      const draft = args.draft ?? {};
      const noteId = draft.noteId ?? null;
      const task: TaskItem = {
        id: nextId(),
        noteId,
        noteTitle: noteId ? state.notes.find((note) => note.id === noteId)?.title ?? null : null,
        title: String(draft.title ?? ""),
        description: String(draft.description ?? ""),
        dueAt: draft.dueAt ?? null,
        priority: Number(draft.priority ?? 0),
        isDone: false,
        repeatRule: draft.repeatRule ?? null,
        createdAt: now(),
        completedAt: null,
        fromChecklist: false,
      };
      state.tasks.push(task);
      commit();
      return task;
    }
    case "update_task": {
      const task = state.tasks.find((item) => item.id === Number(args.id));
      if (!task) throw new Error("المهمة غير موجودة");
      if (args.title !== undefined) task.title = String(args.title);
      if (args.description !== undefined) task.description = String(args.description);
      if (args.dueAt !== undefined) task.dueAt = args.dueAt ?? null;
      if (args.priority !== undefined) task.priority = Number(args.priority);
      if (args.repeatRule !== undefined) task.repeatRule = args.repeatRule ?? null;
      commit();
      return task;
    }
    case "complete_task": {
      const task = state.tasks.find((item) => item.id === Number(args.id));
      if (!task) throw new Error("المهمة غير موجودة");
      task.isDone = Boolean(args.done);
      task.completedAt = task.isDone ? now() : null;
      commit();
      return task;
    }
    case "delete_task": {
      state.tasks = state.tasks.filter((task) => task.id !== Number(args.id));
      commit();
      return null;
    }

    // Snippets ------------------------------------------------------------
    case "list_snippets": {
      const query = String(args.query ?? "").trim().toLowerCase();
      const kind = args.kind ?? null;
      return state.snippets.filter((snippet) => {
        if (kind && snippet.kind !== kind) return false;
        if (!query) return true;
        return `${snippet.title}\n${snippet.content}\n${snippet.tags}`.toLowerCase().includes(query);
      });
    }
    case "create_snippet": {
      const draft = args.draft ?? {};
      const snippet: Snippet = {
        id: nextId(),
        title: String(draft.title ?? "مقطع جديد"),
        kind: draft.kind === "prompt" ? "prompt" : "code",
        language: String(draft.language ?? ""),
        content: String(draft.content ?? ""),
        tags: String(draft.tags ?? ""),
        isPinned: false,
        createdAt: now(),
        updatedAt: now(),
      };
      state.snippets.push(snippet);
      commit();
      return snippet;
    }
    case "update_snippet": {
      const snippet = state.snippets.find((item) => item.id === Number(args.id));
      if (!snippet) throw new Error("المقطع غير موجود");
      const patch = args.patch ?? {};
      if (patch.title !== undefined) snippet.title = String(patch.title);
      if (patch.kind !== undefined) snippet.kind = patch.kind === "prompt" ? "prompt" : "code";
      if (patch.language !== undefined) snippet.language = String(patch.language);
      if (patch.content !== undefined) snippet.content = String(patch.content);
      if (patch.tags !== undefined) snippet.tags = String(patch.tags);
      if (patch.pinned !== undefined) snippet.isPinned = Boolean(patch.pinned);
      snippet.updatedAt = now();
      commit();
      return snippet;
    }
    case "delete_snippet": {
      state.snippets = state.snippets.filter((snippet) => snippet.id !== Number(args.id));
      commit();
      return null;
    }
    case "snippet_count":
      return state.snippets.length;

    // Finance -------------------------------------------------------------
    case "list_categories":
      return state.categories;
    case "create_category": {
      const category: Category = {
        id: nextId(),
        name: String(args.name ?? "فئة"),
        kind: args.kind === "income" ? "income" : "expense",
        color: args.color ?? "#6b7cff",
      };
      state.categories.push(category);
      commit();
      return category;
    }
    case "add_transaction": {
      const draft = args.draft ?? {};
      const category = state.categories.find((item) => item.id === (draft.categoryId ?? null));
      const item: TransactionItem = {
        id: nextId(),
        kind: draft.kind === "income" ? "income" : "expense",
        amountMinor: Number(draft.amountMinor ?? 0),
        currency: draft.currency ?? "EGP",
        categoryId: draft.categoryId ?? null,
        categoryName: category?.name ?? null,
        categoryColor: category?.color ?? null,
        note: draft.note ?? null,
        occurredAt: Number(draft.occurredAt ?? now()),
      };
      state.transactions.push(item);
      commit();
      return item;
    }
    case "list_transactions": {
      const from = Number(args.from ?? 0);
      const to = Number(args.to ?? Number.MAX_SAFE_INTEGER);
      const kind = args.kind ?? null;
      return state.transactions
        .filter((item) => item.occurredAt >= from && item.occurredAt <= to)
        .filter((item) => !kind || item.kind === kind)
        .sort((a, b) => b.occurredAt - a.occurredAt);
    }
    case "delete_transaction": {
      state.transactions = state.transactions.filter((item) => item.id !== Number(args.id));
      commit();
      return null;
    }
    case "monthly_summary":
      return monthlySummary(args);
    case "export_transactions_csv":
      return transactionsCsv(args);

    // Settings ------------------------------------------------------------
    case "get_settings":
      return state.settings;
    case "set_setting": {
      state.settings[String(args.key)] = String(args.value ?? "");
      commit();
      return null;
    }
    case "data_info": {
      const info: DataInfo = {
        dataDir: "(معاينة داخل المتصفح)",
        dbPath: "localStorage",
        backupDir: "localStorage",
        portable: false,
      };
      return info;
    }
    case "open_data_folder":
      return null;
    case "set_default_editor":
      return args.enabled
        ? "تم ضبط NoteRust كالمحرر الافتراضي للملفات النصية"
        : "تم إلغاء ضبط NoteRust كمحرر افتراضي";
    case "set_autostart":
      return null;
    case "shell_state":
      return { defaultEditor: false, autostart: false };
    case "open_default_apps":
      return null;

    // Backup / import / export -------------------------------------------
    case "backup_now":
      return "(معاينة المتصفح) نسخة احتياطية وهمية";
    case "list_backups": {
      const backups: BackupEntry[] = [
        { name: "noterust-backup.zip", path: "(معاينة المتصفح)", size: 245760, createdAt: now() - DAY },
      ];
      return backups;
    }
    case "restore_backup":
      return null;
    case "export_json":
      return "(معاينة المتصفح) noterust-export.json";
    case "export_zip":
      return "(معاينة المتصفح) noterust-export.zip";
    case "import_json_file": {
      const stats: ImportStats = { folders: 0, notes: 0, tasks: 0, transactions: 0 };
      return stats;
    }
    case "import_dialog": {
      const stats: ImportStats = { folders: 0, notes: 0, tasks: 0, transactions: 0 };
      return stats;
    }
    case "import_folder_dialog": {
      const stats: ImportStats = { folders: 0, notes: 0, tasks: 0, transactions: 0 };
      return stats;
    }
    case "refresh_daily_backup":
      return null;

    // Files ---------------------------------------------------------------
    case "open_path": {
      const path = String(args.path ?? "");
      const name = path.split(/[\\/]/).pop() || "ملف";
      const note: Note = {
        id: nextId(),
        folderId: null,
        title: name,
        content: `# ${name}\n\n(معاينة المتصفح — محتوى الملف غير متاح)\n`,
        contentFormat: "markdown",
        isPinned: false,
        isArchived: false,
        isLocked: false,
        color: null,
        direction: "auto",
        sourcePath: path,
        encoding: "utf-8",
        lineEnding: "\n",
        createdAt: now(),
        updatedAt: now(),
        deletedAt: null,
        tags: [],
      };
      state.notes.push(note);
      commit();
      const opened: OpenedFile = {
        noteId: note.id,
        path,
        text: note.content,
        encoding: "utf-8",
        lineEnding: "\n",
        bytes: note.content.length,
        truncated: false,
      };
      return opened;
    }
    case "open_file_dialog":
      return null;
    case "open_startup_files":
      return [];
    case "save_external_file": {
      const note = findNote(Number(args.id));
      note.content = String(args.text ?? "");
      touch(note);
      commit();
      return note;
    }
    case "save_file_as":
      return `(معاينة المتصفح) ${args.suggestedName ?? "ملف.txt"}`;
    case "export_note_file":
      return `(معاينة المتصفح) ${state.notes.find((note) => note.id === Number(args.id))?.title ?? "ملاحظة"}.md`;
    case "stop_watching":
      return null;
    case "reload_external_file": {
      const note = findNote(Number(args.id));
      return note.content;
    }
    case "discard_external_link": {
      const note = findNote(Number(args.id));
      note.sourcePath = null;
      commit();
      return note;
    }

    // Window / board / html ------------------------------------------------
    case "set_window_title":
      if (typeof document !== "undefined") document.title = String(args.title ?? "NoteRust");
      return null;
    case "set_always_on_top":
      return null;
    case "set_compact_mode":
      return null;
    case "save_board":
      state.board = String(args.data ?? "");
      commit();
      return null;
    case "load_board":
      return state.board;
    case "save_html_lab":
      state.htmlLab = String(args.data ?? "");
      commit();
      return null;
    case "load_html_lab":
      return state.htmlLab;
    case "set_html_preview":
      return null;

    // Attachments ----------------------------------------------------------
    case "save_attachment": {
      const ext = String(args.ext ?? "png").replace(/^\./, "");
      const name = `img-${Date.now()}.${ext}`;
      state.attachments[name] = `data:image/${ext};base64,${String(args.data ?? "")}`;
      commit();
      return name;
    }
    case "read_attachment": {
      const name = String(args.name ?? "");
      const data = state.attachments[name];
      if (!data) throw new Error("الصورة غير موجودة");
      return data;
    }
    case "delete_attachment": {
      const name = String(args.name ?? "");
      const data = state.attachments[name];
      if (data) {
        state.trash[name] = data;
        delete state.attachments[name];
        commit();
      }
      return null;
    }
    case "restore_attachment": {
      const name = String(args.name ?? "");
      const data = state.trash[name];
      if (data) {
        state.attachments[name] = data;
        delete state.trash[name];
        commit();
      }
      return null;
    }

    // AI -------------------------------------------------------------------
    case "ai_connections":
      return state.aiConnections;
    case "ai_save_connection": {
      const draft = args.draft ?? {};
      const existing = draft.id
        ? state.aiConnections.find((item) => item.id === Number(draft.id))
        : undefined;
      if (existing) {
        existing.name = String(draft.name ?? existing.name);
        existing.kind = draft.kind ?? existing.kind;
        existing.baseUrl = String(draft.baseUrl ?? existing.baseUrl);
        existing.apiKey = String(draft.apiKey ?? existing.apiKey);
        existing.model = String(draft.model ?? existing.model);
        existing.updatedAt = now();
        commit();
        return existing;
      }
      const connection: AiConnection = {
        id: nextId(),
        name: String(draft.name ?? "اتصال"),
        kind: draft.kind ?? "openai",
        baseUrl: String(draft.baseUrl ?? ""),
        apiKey: String(draft.apiKey ?? ""),
        model: String(draft.model ?? ""),
        createdAt: now(),
        updatedAt: now(),
      };
      state.aiConnections.push(connection);
      commit();
      return connection;
    }
    case "ai_delete_connection": {
      state.aiConnections = state.aiConnections.filter((item) => item.id !== Number(args.id));
      commit();
      return null;
    }
    case "ai_test_connection":
      return "الاتصال ناجح (معاينة المتصفح)";
    case "ai_chat": {
      const connection = state.aiConnections.find((item) => item.id === Number(args.id));
      const prompt = String(args.prompt ?? "");
      return [
        `🧠 رد تجريبي من المعاينة (${connection?.model ?? "no-model"}):`,
        "",
        `سؤالك كان: ${prompt.slice(0, 120)}`,
        "",
        "في التطبيق الحقيقي الرد بييجي من المزوّد المختار.",
      ].join("\n");
    }

    default:
      throw new Error(`أمر غير مدعوم في المعاينة: ${command}`);
  }
}

// ---------------------------------------------------------------------------
// Tauri internals stub
// ---------------------------------------------------------------------------

/**
 * Installs the fake Tauri internals so `@tauri-apps/api` calls land in the
 * preview backend instead of throwing "window.__TAURI_INTERNALS__ is undefined".
 */
export function installBrowserBackend() {
  if (typeof window === "undefined") return;
  // The real Tauri runtime is already there — leave it alone.
  if ("__TAURI_INTERNALS__" in window) return;
  previewMode = true;

  let callbackId = 0;
  const callbacks = new Map<number, (payload: unknown) => void>();

  const invoke = async (cmd: string, args: Args = {}, options?: unknown): Promise<unknown> => {
    // Window / webview / plugin calls the UI makes on startup.
    if (cmd.startsWith("plugin:window|")) return handleWindow(cmd, args);
    if (cmd.startsWith("plugin:webview|")) return handleWebview(cmd, args);
    if (cmd.startsWith("plugin:event|")) return handleEvent(cmd, args);
    if (cmd.startsWith("plugin:app|")) return handleApp(cmd, args);
    if (cmd.startsWith("plugin:updater|")) return handleUpdater(cmd, args);
    if (cmd.startsWith("plugin:process|")) return null;
    if (cmd.startsWith("plugin:os|")) return "windows";
    if (cmd.startsWith("plugin:fs|")) return null;
    if (cmd.startsWith("plugin:dialog|")) return null;
    if (cmd.startsWith("plugin:shell|")) return null;
    void options;
    return handle(cmd, args);
  };

  (window as any).__TAURI_INTERNALS__ = {
    metadata: {
      currentWindow: { label: "main" },
      currentWebview: { label: "main" },
    },
    transformCallback(callback: (payload: unknown) => void) {
      const id = ++callbackId;
      callbacks.set(id, callback);
      return id;
    },
    unregisterCallback(id: number) {
      callbacks.delete(id);
    },
    convertFileSrc(path: string) {
      return path;
    },
    invoke,
  };
}

function handleWindow(cmd: string, args: Args): unknown {
  switch (cmd) {
    case "plugin:window|is_maximized":
    case "plugin:window|is_minimized":
    case "plugin:window|is_fullscreen":
    case "plugin:window|is_focused":
    case "plugin:window|is_decorated":
    case "plugin:window|is_resizable":
    case "plugin:window|is_visible":
    case "plugin:window|is_always_on_top":
      return false;
    case "plugin:window|title":
      return typeof document !== "undefined" ? document.title : "NoteRust";
    case "plugin:window|theme":
      return "Dark";
    case "plugin:window|set_title":
      if (typeof document !== "undefined") document.title = String(args.title ?? "NoteRust");
      return null;
    case "plugin:window|scale_factor":
      return 1;
    default:
      return null;
  }
}

function handleWebview(cmd: string, args: Args): unknown {
  switch (cmd) {
    case "plugin:webview|get_all_webviews":
      return [];
    case "plugin:webview|on_drag_drop_event":
    case "plugin:webview|on_drag_drop":
      return 0;
    default:
      return null;
  }
}

function handleEvent(cmd: string, args: Args): unknown {
  switch (cmd) {
    case "plugin:event|listen":
      return 0;
    case "plugin:event|unlisten":
      return null;
    default:
      return null;
  }
}

function handleApp(cmd: string, args: Args): unknown {
  switch (cmd) {
    case "plugin:app|version":
      return "1.0.7 (معاينة)";
    default:
      return null;
  }
}

function handleUpdater(cmd: string, args: Args): unknown {
  switch (cmd) {
    case "plugin:updater|check":
      return null;
    default:
      return null;
  }
}

/** Resets the preview dataset to the seed. */
export function resetPreviewState() {
  state = seedState();
  commit();
}
