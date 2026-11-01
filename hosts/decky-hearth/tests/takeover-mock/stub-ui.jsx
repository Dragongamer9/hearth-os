// CI only (render-takeover.mjs): a stand-in for @decky/ui so the takeover's real pages (src/pages.tsx) can be drawn in
// a plain browser. Focusable and DialogButton become a div and a button with Steam's classes; the rest does nothing.
import { createElement } from "react";

const strip = (p) => {
  const out = {};
  for (const [k, v] of Object.entries(p)) {
    if (k === "flow-children" || k.startsWith("onGamepad") || k.startsWith("onSecondary") || k.startsWith("onOptions") || k === "focusWithinClassName") continue;
    out[k] = v;
  }
  return out;
};
export function Focusable(props) { return createElement("div", strip(props)); }
export function DialogButton(props) {
  const p = strip(props);
  p.className = ["DialogButton", p.className].filter(Boolean).join(" ");
  return createElement("button", p);
}
export const Navigation = { Navigate() {}, NavigateBack() {}, CloseSideMenus() {} };
export const staticClasses = { Text: "Text", Title: "Title" };
export const useParams = () => ({ id: encodeURIComponent("epic:alan-wake-2") });
export function afterPatch() { return { unpatch() {} }; }
export function findInReactTree() { return null; }
export function findModule() { return null; }
export const PanelSection = (p) => createElement("div", null, p.children);
export const PanelSectionRow = (p) => createElement("div", null, p.children);
export const ToggleField = () => null;
export const ButtonItem = (p) => createElement("button", null, p.children);
