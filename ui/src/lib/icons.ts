/* Minimal 20x20 stroke icons drawn in-house (stroke = currentColor). */

export const ICONS: Record<string, string> = {
  notes:
    '<rect x="3.4" y="2.8" width="13.2" height="14.4" rx="2.2"/><path d="M6.6 7h6.8M6.6 10h6.8M6.6 13h4.2"/>',
  folder:
    '<path d="M2.7 6.3c0-1 .8-1.9 1.9-1.9h3.2l1.8 1.9h6c1 0 1.9.8 1.9 1.9v6.2c0 1-.9 1.9-1.9 1.9H4.6c-1.1 0-1.9-.9-1.9-1.9z"/>',
  folderPlus:
    '<path d="M2.7 6.3c0-1 .8-1.9 1.9-1.9h3.2l1.8 1.9h6c1 0 1.9.8 1.9 1.9v6.2c0 1-.9 1.9-1.9 1.9H4.6c-1.1 0-1.9-.9-1.9-1.9z"/><path d="M10 9v4.4M7.8 11.2h4.4"/>',
  trash:
    '<path d="M3.9 5.7h12.2M8 5.7V4h4v1.7M5.5 5.7l.8 10c.1.8.7 1.4 1.5 1.4h4.4c.8 0 1.4-.6 1.5-1.4l.8-10M8.4 8.9v5.2M11.6 8.9v5.2"/>',
  pin: '<path d="M7.3 2.9h5.4M8 3.1v4.3L5.7 10.5h8.6L12 7.4V3.1M10 10.5v6.4"/>',
  search: '<circle cx="8.7" cy="8.7" r="5"/><path d="M12.4 12.4l4 4"/>',
  settings:
    '<path d="M3.2 6.1h8.3M15.1 6.1h1.7M3.2 13.9h1.7M8.5 13.9h8.3"/><circle cx="13.4" cy="6.1" r="1.8"/><circle cx="6.6" cy="13.9" r="1.8"/>',
  plus: '<path d="M10 4.3v11.4M4.3 10h11.4"/>',
  checklist:
    '<path d="M3.4 5.2l1.4 1.4 2.3-2.5M3.4 12.2l1.4 1.4 2.3-2.5M9.6 5.6h7M9.6 12.6h7"/>',
  compose:
    '<path d="M16.1 10.5v4.1c0 1.2-.9 2.1-2.1 2.1H5.2c-1.2 0-2.1-.9-2.1-2.1V6.1c0-1.2.9-2.1 2.1-2.1h4.2"/><path d="M14.3 2.9l2.7 2.7-6.8 6.8-3.2.6.6-3.2z"/>',
  tasks: '<circle cx="10" cy="10" r="7.1"/><path d="M7 10.2l2 2 4.1-4.4"/>',
  finance: '<path d="M10 2.9a7.1 7.1 0 1 0 7.1 7.1H10z"/><path d="M12.4 3a7.2 7.2 0 0 1 4.6 4.6h-4.6z"/>',
  wallet:
    '<rect x="2.9" y="5.1" width="14.2" height="10.8" rx="2.2"/><path d="M2.9 8.4h14.2M12.7 12.1h1.7"/>',
  calendar:
    '<rect x="3.1" y="4.5" width="13.8" height="12.4" rx="2.1"/><path d="M3.1 8.4h13.8M7 2.9v3.1M13 2.9v3.1"/>',
  flag: '<path d="M5.3 17.2V3.4M5.3 4h9.1l-2.1 3.3 2.1 3.4H5.3"/>',
  close: '<path d="M5.3 5.3l9.4 9.4M14.7 5.3l-9.4 9.4"/>',
  restore: '<path d="M4.7 8.3a6 6 0 1 1 .9 5.3"/><path d="M4.4 4.7v3.7h3.7"/>',
  move:
    '<path d="M2.7 6.3c0-1 .8-1.9 1.9-1.9h3.2l1.8 1.9h6c1 0 1.9.8 1.9 1.9v6.2c0 1-.9 1.9-1.9 1.9H4.6c-1.1 0-1.9-.9-1.9-1.9z"/><path d="M11.6 9.1l-2.2 2.2 2.2 2.2M9.8 11.3h4"/>',
  archive:
    '<rect x="2.9" y="3.4" width="14.2" height="4" rx="1.4"/><path d="M4.2 7.4v7.1c0 1.2.9 2.1 2.1 2.1h7.4c1.2 0 2.1-.9 2.1-2.1V7.4M8.2 10.4h3.6"/>',
  lock:
    '<rect x="4.4" y="8.6" width="11.2" height="8.2" rx="2.1"/><path d="M6.8 8.6V6.4a3.2 3.2 0 0 1 6.4 0v2.2M10 12v2.4"/>',
  history:
    '<path d="M4.6 8.2a6 6 0 1 1 .9 5.4"/><path d="M4.3 4.6v3.8h3.8M10 6.7v3.6l2.6 1.5"/>',
  more: '<circle cx="5" cy="10" r=".7" fill="currentColor" stroke="none"/><circle cx="10" cy="10" r=".7" fill="currentColor" stroke="none"/><circle cx="15" cy="10" r=".7" fill="currentColor" stroke="none"/>',
  sun: '<circle cx="10" cy="10" r="3.4"/><path d="M10 2.7v1.6M10 15.7v1.6M2.7 10h1.6M15.7 10h1.6M4.9 4.9l1.1 1.1M14 14l1.1 1.1M4.9 15.1l1.1-1.1M14 6l1.1-1.1"/>',
  moon: '<path d="M16 12.2A6.6 6.6 0 0 1 7.8 4a6.8 6.8 0 1 0 8.2 8.2z"/>',
  check: '<path d="M4.7 10.4l3.3 3.3 7.3-7.5"/>',
  chevronDown: '<path d="M4.8 7.6l5.2 5.2 5.2-5.2"/>',
  chevronRight: '<path d="M7.8 4.5l5.5 5.5-5.5 5.5"/>',
  chevronLeft: '<path d="M12.2 4.5L6.7 10l5.5 5.5"/>',
  sidebar:
    '<rect x="2.7" y="3.9" width="14.6" height="12.2" rx="2.5"/><path d="M12.3 3.9v12.2"/>',
  exportIcon:
    '<path d="M10 12.6V3.4M6.7 6.6L10 3.3l3.3 3.3"/><path d="M4.5 10.6v4.5c0 1 .8 1.8 1.8 1.8h7.4c1 0 1.8-.8 1.8-1.8v-4.5"/>',
  importIcon:
    '<path d="M10 3.4v9.2M6.7 9.4l3.3 3.3 3.3-3.3"/><path d="M4.5 10.6v4.5c0 1 .8 1.8 1.8 1.8h7.4c1 0 1.8-.8 1.8-1.8v-4.5"/>',
  alert:
    '<path d="M10 3.3l7.5 13H2.5z"/><path d="M10 8.3v3.5"/><circle cx="10" cy="13.9" r=".7" fill="currentColor" stroke="none"/>',
  sparkle: '<path d="M10 2.9l1.6 4.3 4.5 1.6-4.5 1.6L10 14.8l-1.6-4.4-4.5-1.6 4.5-1.6z"/>',
  data:
    '<ellipse cx="10" cy="5.3" rx="6" ry="2.4"/><path d="M4 5.3v9.4c0 1.3 2.7 2.4 6 2.4s6-1.1 6-2.4V5.3M4 10c0 1.3 2.7 2.4 6 2.4s6-1.1 6-2.4"/>',
  code: '<path d="M7.4 6.6L3.6 10l3.8 3.4M12.6 6.6L16.4 10l-3.8 3.4M11.3 4.3l-2.6 11.4"/>',
  prompt:
    '<rect x="2.9" y="4.2" width="14.2" height="11.6" rx="2.4"/><path d="M6.4 8.1l2.3 2-2.3 2M11 12.5h3"/>',
  copy: '<rect x="7.1" y="7.1" width="9.4" height="9.4" rx="1.8"/><path d="M12.7 7.1V4.9c0-1-.8-1.8-1.8-1.8H4.7c-1 0-1.8.8-1.8 1.8v6.2c0 1 .8 1.8 1.8 1.8h2.4"/>',
  book:
    '<path d="M3.2 4.4c0-.8.7-1.5 1.5-1.5h3.6c.8 0 1.5.7 1.5 1.5v12.2c0-.8-.7-1.5-1.5-1.5H4.7c-.8 0-1.5.7-1.5 1.5z"/><path d="M16.8 4.4c0-.8-.7-1.5-1.5-1.5h-3.6c-.8 0-1.5.7-1.5 1.5v12.2c0-.8.7-1.5 1.5-1.5h3.6c.8 0 1.5.7 1.5 1.5z"/>',
};

export function iconBody(name: string): string {
  return ICONS[name] ?? ICONS.notes;
}
