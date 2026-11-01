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
// Quick Access's building blocks, drawn roughly the way Steam draws them (render-takeover.mjs styles the classes).
export const PanelSection = (p) => createElement("div", { className: "qam-section" }, p.title ? createElement("div", { className: "qam-section-title" }, p.title) : null, p.children);
export const PanelSectionRow = (p) => createElement("div", { className: "qam-row" }, p.children);
export const ToggleField = (p) => createElement("div", { className: "qam-field qam-toggle" },
  createElement("div", { className: "qam-line" }, createElement("span", { className: "qam-label" }, p.label), createElement("span", { className: "qam-switch" + (p.checked ? " on" : "") }, createElement("i"))),
  p.description ? createElement("div", { className: "qam-desc" }, p.description) : null);
export const ButtonItem = (p) => createElement("div", { className: "qam-field" },
  p.label ? createElement("div", { className: "qam-label" }, p.label) : null,
  createElement("button", { className: "DialogButton qam-btn", onClick: p.onClick }, p.children),
  p.description ? createElement("div", { className: "qam-desc" }, p.description) : null);
export const DropdownItem = (p) => {
  const sel = (p.rgOptions || []).find((o) => o.data === p.selectedOption);
  return createElement("div", { className: "qam-field" },
    createElement("div", { className: "qam-label" }, p.label),
    createElement("button", { className: "DialogButton qam-drop" }, sel ? sel.label : "", createElement("span", { className: "qam-caret" }, "▾")),
    p.description ? createElement("div", { className: "qam-desc" }, p.description) : null);
};
// The game Steam runs (Decky's Router.MainRunningApp), set by the picture's page.
export const Router = { get MainRunningApp() { return window.HEARTH_RUNNING || null; } };
export const scrollClasses = { ScrollPanel: "ScrollPanel", ScrollY: "ScrollY" };
export const showModal = () => {};
export const ConfirmModal = () => null;
