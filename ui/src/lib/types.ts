/* Types mirror the Rust models (serde `camelCase`). */

export interface Tag {
  id: number;
  name: string;
  color: string;
}

export interface Folder {
  id: number;
  parentId: number | null;
  name: string;
  icon: string;
  color: string;
  sortOrder: number;
  createdAt: number;
}

export interface FolderNode extends Folder {
  noteCount: number;
  children: FolderNode[];
}

export interface Note {
  id: number;
  folderId: number | null;
  title: string;
  content: string;
  contentFormat: string;
  isPinned: boolean;
  isArchived: boolean;
  isLocked: boolean;
  color: string | null;
  direction: "auto" | "rtl" | "ltr";
  sourcePath: string | null;
  encoding: string | null;
  lineEnding: string | null;
  createdAt: number;
  updatedAt: number;
  deletedAt: number | null;
  tags: Tag[];
}

export interface NoteSummary {
  id: number;
  folderId: number | null;
  title: string;
  preview: string;
  isPinned: boolean;
  isLocked: boolean;
  isArchived: boolean;
  color: string | null;
  createdAt: number;
  updatedAt: number;
  wordCount: number;
  tags: Tag[];
}

export interface SearchHit extends NoteSummary {
  snippet: string;
}

export interface NoteVersion {
  id: number;
  noteId: number;
  content: string;
  title: string;
  savedAt: number;
}

export interface TaskItem {
  id: number;
  noteId: number | null;
  noteTitle: string | null;
  title: string;
  description: string;
  dueAt: number | null;
  priority: number;
  isDone: boolean;
  repeatRule: string | null;
  createdAt: number;
  completedAt: number | null;
  fromChecklist: boolean;
}

export interface Snippet {
  id: number;
  title: string;
  kind: "code" | "prompt";
  language: string;
  content: string;
  tags: string;
  isPinned: boolean;
  createdAt: number;
  updatedAt: number;
}

export interface AiConnection {
  id: number;
  name: string;
  kind: "openai" | "gemini" | "anthropic";
  baseUrl: string;
  apiKey: string;
  model: string;
  createdAt: number;
  updatedAt: number;
}

export interface AiConnectionDraft {
  id?: number | null;
  name: string;
  kind: "openai" | "gemini" | "anthropic";
  baseUrl: string;
  apiKey: string;
  model: string;
}

export interface Category {
  id: number;
  name: string;
  kind: "income" | "expense";
  color: string;
}

export interface TransactionItem {
  id: number;
  kind: "income" | "expense";
  amountMinor: number;
  currency: string;
  categoryId: number | null;
  categoryName: string | null;
  categoryColor: string | null;
  note: string | null;
  occurredAt: number;
}

export interface CategoryTotal {
  categoryId: number | null;
  name: string;
  kind: string;
  color: string;
  totalMinor: number;
}

export interface DayTotal {
  day: number;
  incomeMinor: number;
  expenseMinor: number;
}

export interface MonthlySummary {
  year: number;
  month: number;
  incomeMinor: number;
  expenseMinor: number;
  balanceMinor: number;
  currency: string;
  byCategory: CategoryTotal[];
  byDay: DayTotal[];
}

export interface Counts {
  all: number;
  trash: number;
  archived: number;
  tasksOpen: number;
}

export interface OpenedFile {
  noteId: number;
  path: string;
  text: string;
  encoding: string;
  lineEnding: string;
  bytes: number;
  truncated: boolean;
}

export interface BackupEntry {
  name: string;
  path: string;
  size: number;
  createdAt: number;
}

export interface DataInfo {
  dataDir: string;
  dbPath: string;
  backupDir: string;
  portable: boolean;
}

export interface ImportStats {
  folders: number;
  notes: number;
  tasks: number;
  transactions: number;
}
