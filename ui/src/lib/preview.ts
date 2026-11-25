import { isTauriRuntime } from "./browserBackend";

/**
 * URL for the HTML preview frame.
 *
 * Inside the app the frame lives on the `htmlpreview://` origin so the app's
 * CSP is not inherited. In the browser preview there is no such protocol, so
 * the markup is served from a blob URL instead.
 */
export function previewFrameUrl(html: string, tauriOrigin: string): string {
  if (isTauriRuntime()) return `${tauriOrigin}/?v=${Date.now()}`;
  return URL.createObjectURL(new Blob([html], { type: "text/html;charset=utf-8" }));
}
