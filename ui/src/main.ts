import { mount } from "svelte";
import "@fontsource/cairo/400.css";
import "@fontsource/cairo/600.css";
import "@fontsource/cairo/700.css";
import "./app.css";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) {
  throw new Error("عنصر #app غير موجود");
}

const app = mount(App, { target });

export default app;
