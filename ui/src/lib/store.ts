import { derived, get, writable } from "svelte/store";
import { api } from "./api";
import type { Counts, FolderNode, Note, NoteSummary, SearchHit } from "./types";

export type View =
  | "notes"
  | "tasks"
  | "finance"
  | "snippets"
  | "board"
  | "ai"
  | "html"
  | "settings"
  | "trash";

export const folders = writable<FolderNode[]>([]);
export const notes = writable<(NoteSummary | SearchHit)[]>([]);
export const counts = writable<Counts>({ all: 0, trash: 0, archived: 0, tasksOpen: 0 });
export const view = writable<View>("notes");
export const activeFolder = writable<number | null>(null);
export const searchQuery = writable("");
export const sortBy = writable("updated");
export const tabs = writable<Note[]>([]);
export const activeTab = writable<number | null>(null);
export const settings = writable<Record<string, string>>({});
export const saveStatus = writable<"saved" | "saving" | "unsaved" | "error">("saved");
export const lastSavedAt = writable<number>(Date.now());
export const fileChangeNotice = writable<number | null>(null);
export const loading = writable(false);

export const activeNote = derived(
  [tabs, activeTab],
  ([$tabs, $activeTab]) => $tabs.find((note) => note.id === $activeTab) ?? null,
);

export interface ToastAction {
  label: string;
  run: () => void;
}

export interface Toast {
  id: number;
  text: string;
  kind: "ok" | "error";
  action?: ToastAction;
}

export const toasts = writable<Toast[]>([]);
let toastId = 0;

export function toast(text: string, kind: "ok" | "error" = "ok", action?: ToastAction) {
  const id = ++toastId;
  toasts.update((list) => [...list, { id, text, kind, action }]);
  setTimeout(() => {
    toasts.update((list) => list.filter((item) => item.id !== id));
  }, action ? 6500 : 3800);
}

/** Window pinned above every other program on the desktop. */
export const alwaysOnTop = writable(false);
/** Small "sticky note" mode: only the open note, always on top. */
export const compact = writable(false);

export async function setAlwaysOnTop(value: boolean) {
  try {
    await api.setAlwaysOnTop(value);
    alwaysOnTop.set(value);
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

export async function setCompact(value: boolean) {
  try {
    await api.setCompactMode(value);
    compact.set(value);
    // Entering sticky mode pins the window; leaving it unpins.
    alwaysOnTop.set(value);
    toast(value ? "وضع الملاحظة البسيطة — زر ⤢ للخروج" : "رجعت للنافذة الكاملة");
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

export function dismissToast(id: number) {
  toasts.update((list) => list.filter((item) => item.id !== id));
}

/** Storage key for the restored session. */
const SESSION_KEY = "noterust.session";

interface Session {
  view: View;
  folderId: number | null;
  tabs: number[];
  activeTab: number | null;
  sortBy: string;
}

export function loadSession(): Session | null {
  try {
    const raw = localStorage.getItem(SESSION_KEY);
    return raw ? (JSON.parse(raw) as Session) : null;
  } catch {
    return null;
  }
}

export function saveSession() {
  const session: Session = {
    view: get(view),
    folderId: get(activeFolder),
    tabs: get(tabs).map((note) => note.id),
    activeTab: get(activeTab),
    sortBy: get(sortBy),
  };
  localStorage.setItem(SESSION_KEY, JSON.stringify(session));
}

export async function refreshFolders() {
  folders.set(await api.listFolders());
}

export async function refreshCounts() {
  counts.set(await api.counts());
}

export async function refreshNotes() {
  const query = get(searchQuery).trim();
  if (query.length > 0) {
    notes.set(await api.searchNotes(query, get(activeFolder)));
    return;
  }
  const list = await api.listNotes({
    folderId: get(activeFolder),
    trash: get(view) === "trash",
    sort: get(sortBy),
    limit: 1000,
  });
  notes.set(list);
}

export async function refreshAll() {
  loading.set(true);
  try {
    await Promise.all([refreshFolders(), refreshNotes(), refreshCounts()]);
  } catch (error) {
    toast(errorMessage(error), "error");
  } finally {
    loading.set(false);
  }
  saveSession();
}

export function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export async function selectFolder(folderId: number | null) {
  activeFolder.set(folderId);
  view.set("notes");
  searchQuery.set("");
  await refreshNotes();
  saveSession();
}

export async function setView(next: View) {
  view.set(next);
  searchQuery.set("");
  // The trash and the other views are global, never scoped to a folder —
  // keeping a selected folder here hid every note deleted outside it.
  activeFolder.set(null);
  if (next === "notes" || next === "trash") {
    await refreshNotes();
  }
  saveSession();
}

export async function openNote(id: number, silent = false) {
  try {
    const note = await api.getNote(id);
    tabs.update((list) => {
      const index = list.findIndex((item) => item.id === id);
      if (index === -1) return [...list, note];
      const copy = [...list];
      copy[index] = note;
      return copy;
    });
    activeTab.set(id);
    saveSession();
    updateWindowTitle(note);
  } catch (error) {
    if (!silent) toast(errorMessage(error), "error");
  }
}

export function updateTab(note: Note) {
  tabs.update((list) => list.map((item) => (item.id === note.id ? note : item)));
  updateWindowTitle(note);
  saveSession();
}

export function updateWindowTitle(note: Note | null) {
  const title = note?.title?.trim();
  void api
    .setWindowTitle(title ? `${title} — NoteRust` : "NoteRust")
    .catch(() => undefined);
}

export async function closeTab(id: number) {
  const remaining = get(tabs).filter((note) => note.id !== id);
  tabs.set(remaining);
  if (get(activeTab) === id) {
    const next = remaining[remaining.length - 1] ?? null;
    activeTab.set(next ? next.id : null);
    updateWindowTitle(next);
  }
  saveSession();
  void api.stopWatching(id).catch(() => undefined);
}

export async function createNote() {
  try {
    const folder = get(view) === "notes" ? get(activeFolder) : null;
    const note = await api.createNote(folder);
    await openNote(note.id);
    await refreshAll();
    return note;
  } catch (error) {
    toast(errorMessage(error), "error");
    return null;
  }
}

export async function removeNoteFromUi(id: number) {
  await closeTab(id);
  await refreshAll();
}

export async function loadSettings() {
  try {
    settings.set(await api.getSettings());
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

export async function updateSetting(key: string, value: string) {
  try {
    await api.setSetting(key, value);
    settings.update((current) => ({ ...current, [key]: value }));
  } catch (error) {
    toast(errorMessage(error), "error");
  }
}

export function setting(key: string, fallback = ""): string {
  return get(settings)[key] ?? fallback;
}

/** Flattens the folder tree for selects and pickers. */
export function flattenFolders(nodes: FolderNode[], depth = 0): (FolderNode & { depth: number })[] {
  return nodes.flatMap((node) => [
    { ...node, depth },
    ...flattenFolders(node.children, depth + 1),
  ]);
}

export function findFolder(nodes: FolderNode[], id: number | null): FolderNode | null {
  if (id === null) return null;
  for (const node of nodes) {
    if (node.id === id) return node;
    const found = findFolder(node.children, id);
    if (found) return found;
  }
  return null;
}
