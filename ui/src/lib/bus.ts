import { writable } from "svelte/store";

/** Commands sent from the global shortcut handler to the focused editor. */
export type EditorCommand =
  | "find"
  | "replace"
  | "go-to-line"
  | "duplicate-line"
  | "insert-date"
  | "toggle-wrap"
  | "save"
  | "save-as"
  | "focus";

let sequence = 0;

export const editorCommand = writable<{ name: EditorCommand; seq: number } | null>(null);

export function sendEditorCommand(name: EditorCommand) {
  editorCommand.set({ name, seq: ++sequence });
}
