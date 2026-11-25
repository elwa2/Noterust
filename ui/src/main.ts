import { mount } from "svelte";
import "@fontsource/cairo/400.css";
import "@fontsource/cairo/600.css";
import "@fontsource/cairo/700.css";
import "@fontsource/ibm-plex-sans-arabic/400.css";
import "@fontsource/ibm-plex-sans-arabic/600.css";
import "@fontsource/ibm-plex-sans-arabic/700.css";
import "./app.css";
import App from "./App.svelte";
import { installBrowserBackend } from "./lib/browserBackend";

// Outside the Tauri webview (a plain browser) the UI talks to the preview
// backend instead of the Rust one. No-op in the real app.
installBrowserBackend();

const target = document.getElementById("app");
if (!target) {
  throw new Error("عنصر #app غير موجود");
}

const app = mount(App, { target });

export default app;
