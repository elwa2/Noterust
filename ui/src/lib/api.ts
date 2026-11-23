import { invoke } from "@tauri-apps/api/core";
import type {
  BackupEntry,
  Category,
  Counts,
  DataInfo,
  Folder,
  FolderNode,
  ImportStats,
  MonthlySummary,
  Note,
  NoteSummary,
  NoteVersion,
  OpenedFile,
  SearchHit,
  Tag,
  TaskItem,
  TransactionItem,
} from "./types";

export interface ListQuery {
  folderId?: number | null;
  trash?: boolean;
  includeArchived?: boolean;
  sort?: string;
  limit?: number;
}

export interface TaskDraft {
  noteId?: number | null;
  title: string;
  description?: string;
  dueAt?: number | null;
  priority?: number;
  repeatRule?: string | null;
}

export interface TransactionDraft {
  kind: "income" | "expense";
  amountMinor: number;
  currency?: string;
  categoryId?: number | null;
  note?: string | null;
  occurredAt: number;
}

const call = <T>(command: string, args?: Record<string, unknown>) =>
  invoke<T>(command, args).catch((error) => {
    throw new Error(typeof error === "string" ? error : String(error));
  });

export const api = {
  listNotes: (query: ListQuery) => call<NoteSummary[]>("list_notes", { query }),
  getNote: (id: number) => call<Note>("get_note", { id }),
  createNote: (folderId: number | null) => call<Note>("create_note", { folderId }),
  updateNoteContent: (id: number, content: string) =>
    call<Note>("update_note_content", { id, content }),
  updateNoteTitle: (id: number, title: string) => call<Note>("update_note_title", { id, title }),
  moveNote: (id: number, folderId: number | null) => call<Note>("move_note", { id, folderId }),
  pinNote: (id: number, pinned: boolean) => call<Note>("pin_note", { id, pinned }),
  archiveNote: (id: number, archived: boolean) => call<Note>("archive_note", { id, archived }),
  setNoteColor: (id: number, color: string | null) =>
    call<Note>("set_note_color", { id, color }),
  setNoteDirection: (id: number, direction: string) =>
    call<Note>("set_note_direction", { id, direction }),
  deleteNote: (id: number) => call<void>("delete_note", { id }),
  restoreNote: (id: number) => call<Note>("restore_note", { id }),
  purgeNote: (id: number) => call<void>("purge_note", { id }),
  emptyTrash: () => call<number>("empty_trash"),
  duplicateNote: (id: number) => call<Note>("duplicate_note", { id }),
  searchNotes: (query: string, folderId: number | null) =>
    call<SearchHit[]>("search_notes", { query, folderId, limit: 200 }),
  counts: () => call<Counts>("counts"),
  setNoteTags: (id: number, tags: string[]) => call<Tag[]>("set_note_tags", { id, tags }),
  listTags: () => call<(Tag & { count: number })[]>("list_tags"),
  deleteTag: (id: number) => call<void>("delete_tag", { id }),

  noteVersions: (id: number) => call<NoteVersion[]>("note_versions", { id }),
  restoreVersion: (versionId: number) => call<Note>("restore_version", { versionId }),
  lockNote: (id: number, password: string) => call<Note>("lock_note", { id, password }),
  unlockNote: (id: number, password: string) => call<string>("unlock_note", { id, password }),
  saveLockedNote: (id: number, password: string, content: string) =>
    call<Note>("save_locked_note", { id, password, content }),
  removeLock: (id: number, password: string) => call<Note>("remove_lock", { id, password }),

  listFolders: () => call<FolderNode[]>("list_folders"),
  createFolder: (parentId: number | null, name: string, icon?: string, color?: string) =>
    call<Folder>("create_folder", { parentId, name, icon, color }),
  updateFolder: (
    id: number,
    changes: { name?: string; icon?: string; color?: string; parentId?: number | null },
  ) => call<Folder>("update_folder", { id, ...changes }),
  deleteFolder: (id: number) => call<void>("delete_folder", { id }),
  reorderFolder: (id: number, index: number) => call<void>("reorder_folder", { id, index }),

  listTasks: (includeDone: boolean) => call<TaskItem[]>("list_tasks", { includeDone }),
  createTask: (draft: TaskDraft) => call<TaskItem>("create_task", { draft }),
  updateTask: (
    id: number,
    changes: {
      title?: string;
      description?: string;
      dueAt?: number | null;
      priority?: number;
      repeatRule?: string | null;
    },
  ) => call<TaskItem>("update_task", { id, ...changes }),
  completeTask: (id: number, done: boolean) => call<TaskItem>("complete_task", { id, done }),
  deleteTask: (id: number) => call<void>("delete_task", { id }),

  listCategories: () => call<Category[]>("list_categories"),
  createCategory: (name: string, kind: string, color?: string) =>
    call<Category>("create_category", { name, kind, color }),
  addTransaction: (draft: TransactionDraft) => call<TransactionItem>("add_transaction", { draft }),
  listTransactions: (from: number, to: number, kind?: string) =>
    call<TransactionItem[]>("list_transactions", { from, to, kind }),
  deleteTransaction: (id: number) => call<void>("delete_transaction", { id }),
  monthlySummary: (year: number, month: number) =>
    call<MonthlySummary>("monthly_summary", { year, month }),
  exportTransactionsCsv: (year: number, month: number) =>
    call<string | null>("export_transactions_csv", { year, month }),

  getSettings: () => call<Record<string, string>>("get_settings"),
  setSetting: (key: string, value: string) => call<void>("set_setting", { key, value }),
  dataInfo: () => call<DataInfo>("data_info"),
  openDataFolder: () => call<void>("open_data_folder"),
  setDefaultEditor: (enabled: boolean) => call<string>("set_default_editor", { enabled }),
  setAutostart: (enabled: boolean) => call<void>("set_autostart", { enabled }),

  backupNow: () => call<string>("backup_now"),
  listBackups: () => call<BackupEntry[]>("list_backups"),
  restoreBackup: (path: string) => call<void>("restore_backup", { path }),
  exportJson: () => call<string | null>("export_json"),
  exportZip: () => call<string | null>("export_zip"),
  importJsonFile: () => call<ImportStats | null>("import_json_file"),

  openPath: (path: string) => call<OpenedFile>("open_path", { path }),
  openFileDialog: () => call<OpenedFile | null>("open_file_dialog"),
  openStartupFiles: () => call<string[]>("open_startup_files"),
  saveExternalFile: (id: number, text: string) => call<Note>("save_external_file", { id, text }),
  saveFileAs: (
    text: string,
    suggestedName?: string,
    encoding?: string,
    lineEnding?: string,
  ) => call<string | null>("save_file_as", { text, suggestedName, encoding, lineEnding }),
  exportNoteFile: (id: number, format: string) =>
    call<string | null>("export_note_file", { id, format }),
  importDialog: (folderId: number | null) => call<ImportStats | null>("import_dialog", { folderId }),
  importFolderDialog: (folderId: number | null) =>
    call<ImportStats | null>("import_folder_dialog", { folderId }),
  stopWatching: (id: number) => call<void>("stop_watching", { id }),
  reloadExternalFile: (id: number) => call<string>("reload_external_file", { id }),
  discardExternalLink: (id: number) => call<Note>("discard_external_link", { id }),
  setWindowTitle: (title: string) => call<void>("set_window_title", { title }),
  refreshDailyBackup: () => call<void>("refresh_daily_backup"),
};
