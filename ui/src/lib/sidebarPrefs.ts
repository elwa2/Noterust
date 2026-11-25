import { writable } from "svelte/store";

/** Sidebar entries the user can show or hide, in display order. */
export interface SidebarItem {
  id: string;
  name: string;
}

export const SIDEBAR_ITEMS: SidebarItem[] = [
  { id: "notes", name: "كل الملاحظات" },
  { id: "trash", name: "المحذوفة مؤخراً" },
  { id: "tasks", name: "المهام" },
  { id: "finance", name: "الدخل والمصروفات" },
  { id: "snippets", name: "الأكواد والبرومبت" },
  { id: "board", name: "السبورة" },
  { id: "ai", name: "الذكاء الاصطناعي" },
  { id: "html", name: "معمل HTML" },
  { id: "calculator", name: "الآلة الحاسبة" },
  { id: "money", name: "عدّ الفلوس" },
];

export interface SidebarPrefs {
  /** IDs of entries the user chose to hide. */
  hidden: string[];
  /** Icon-only rail instead of the full sidebar. */
  collapsed: boolean;
}

const KEY = "noterust.sidebar";

function read(): SidebarPrefs {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Partial<SidebarPrefs>;
      return {
        hidden: Array.isArray(parsed.hidden)
          ? parsed.hidden.filter((id): id is string => typeof id === "string")
          : [],
        collapsed: parsed.collapsed === true,
      };
    }
    // Builds before this preference existed only had a fully hidden sidebar.
    const ui = localStorage.getItem("noterust.ui");
    if (ui) {
      const saved = JSON.parse(ui) as { sidebarHidden?: boolean };
      if (saved.sidebarHidden === true) return { hidden: [], collapsed: true };
    }
  } catch {
    /* ignore corrupt preferences */
  }
  return { hidden: [], collapsed: false };
}

export const sidebarPrefs = writable<SidebarPrefs>(read());

sidebarPrefs.subscribe((prefs) => {
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs));
  } catch {
    /* storage full: keep the choice for this run */
  }
});

export function setSidebarCollapsed(collapsed: boolean) {
  sidebarPrefs.update((prefs) => ({ ...prefs, collapsed }));
}

export function toggleSidebarItem(id: string) {
  sidebarPrefs.update((prefs) => ({
    ...prefs,
    hidden: prefs.hidden.includes(id)
      ? prefs.hidden.filter((item) => item !== id)
      : [...prefs.hidden, id],
  }));
}
