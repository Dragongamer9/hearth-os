// CI only (render.mjs): a stand-in for @decky/ui so Hearth Classic's real code (src/) runs in a plain browser.
// Focusable and DialogButton become a div and a button; Steam's class maps are made-up readable names that the
// stand-in Steam frame (render.mjs) uses, so the real skin pipeline (theme.ts, model.ts renderTheme) is what styles it.
// window.HC_STEAM === "changed" plays a Steam update that renamed its modules: the maps are gone.
import { createElement } from "react";

const changed = window.HC_STEAM === "changed";
const strip = (p) => {
  const out = {};
  for (const [k, v] of Object.entries(p)) {
    if (k === "flow-children" || k.startsWith("onGamepad") || k.startsWith("onSecondary") || k === "autoFocus") continue;
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
export const Navigation = { Navigate(r) { (window.HC_NAV = window.HC_NAV || []).push(r); }, NavigateBack() {}, CloseSideMenus() {}, NavigateToExternalWeb() {} };
export const staticClasses = { Text: "Text", Title: "Title" };
export const gamepadTabbedPageClasses = changed ? undefined : { GamepadTabbedPage: "gtp_Page", TabHeaderRowWrapper: "gtp_Header", Tab: "gtp_Tab", Selected: "gtp_Selected" };
export const gamepadDialogClasses = changed ? undefined : { GamepadDialogContent: "gd_Content", Field: "gd_Field", FieldDescription: "qam-desc", Button: "gd_Button" };
export const quickAccessMenuClasses = changed ? undefined : { Title: "qam-title", Tabs: "qam-tabs", Tab: "qam-tab", Selected: "on" };
export const footerClasses = changed ? undefined : { BasicFooter: "steam-footer" };
const modules = changed ? [] : [{ MainNavMenuContainer: "mainmenu", SteamLogo: "mainmenu-logo" }];
export function findClassModule(f) { return modules.find((m) => { try { return f(m); } catch { return false; } }); }
export function findSP() { return window; }
export const Router = { WindowStore: { GamepadUIMainWindowInstance: { BrowserWindow: window }, SteamUIWindows: [] } };
// Quick Access's building blocks, drawn roughly the way Steam draws them (render.mjs styles the classes).
export const PanelSection = (p) => createElement("div", { className: "qam-section" }, p.title ? createElement("div", { className: "qam-section-title" }, p.title) : null, p.children);
export const PanelSectionRow = (p) => createElement("div", { className: "qam-row" }, p.children);
export const ToggleField = (p) => createElement("div", { className: "qam-field gd_Field" },
  createElement("div", { className: "qam-line" }, createElement("span", { className: "qam-label" }, p.label), createElement("span", { className: "qam-switch" + (p.checked ? " on" : "") }, createElement("i"))),
  p.description ? createElement("div", { className: "qam-desc" }, p.description) : null);
export const ButtonItem = (p) => createElement("div", { className: "qam-field" },
  createElement("button", { className: "DialogButton gd_Button qam-btn", onClick: p.onClick }, p.children));
export const DropdownItem = (p) => {
  const sel = (p.rgOptions || []).find((o) => o.data === p.selectedOption);
  return createElement("div", { className: "qam-field gd_Field" },
    createElement("div", { className: "qam-label" }, p.label),
    createElement("button", { className: "DialogButton gd_Button qam-drop", "data-options": (p.rgOptions || []).map((o) => o.label).join("|"),
      onClick: () => { const o = (p.rgOptions || []).find((x) => x.data !== p.selectedOption); if (o) p.onChange(o); } }, sel ? sel.label : "", createElement("span", null, "▾")),
    p.description ? createElement("div", { className: "qam-desc" }, p.description) : null);
};
