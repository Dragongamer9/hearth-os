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
const menuChanged = changed || window.HC_STEAM === "menuchanged";
const map = (o) => (changed ? undefined : o);
export const gamepadTabbedPageClasses = map({ GamepadTabbedPage: "gtp_Page", TabHeaderRowWrapper: "gtp_Header", Tab: "gtp_Tab", Selected: "gtp_Selected", TabCount: "gtp_Count" });
export const gamepadDialogClasses = map({ GamepadDialogContent: "gd_Content", Field: "gd_Field", FieldLabel: "qam-label", FieldDescription: "qam-desc", Button: "gd_Button", Toggle: "qam-toggle", On: "on", ToggleRail: "qam-switch" });
export const quickAccessMenuClasses = map({ Container: "qam", Title: "qam-title", Tabs: "qam-tabs", Tab: "qam-tab", Selected: "on", PanelSectionTitle: "qam-section-title", PanelSectionRow: "qam-row" });
export const quickAccessControlsClasses = map({ PanelSectionTitle: "qam-section-title" });
export const footerClasses = map({ BasicFooter: "steam-footer" });
export const gamepadLibraryClasses = map({ GamepadLibrary: "lib_Root", CollectionHeader: "lib_Header", AppGridFilterHeader: "lib_Filter" });
export const libraryAssetImageClasses = map({ Container: "cap", Image: "cap-img", GreyBackground: "cap-grey" });
export const appDetailsClasses = map({ Container: "ad_Container" });
export const appDetailsHeaderClasses = map({ TopGradient: "ad_Gradient", TopCapsule: "ad_Capsule" });
export const playSectionClasses = map({ PlayBar: "ps_PlayBar", Container: "ps_Container", GameStat: "ps_Stat", MenuButton: "ps_Menu" });
export const appActionButtonClasses = map({ PlayButton: "ps_Play", Green: "ps_Green" });
export const basicAppDetailsSectionStylerClasses = map({ AppDetailsContent: "ad_Content", GameInfoContainer: "ad_Info", GameInfoQuickLinks: "ad_Links" });
export const gamepadContextMenuClasses = map({ contextMenuContents: "cm_Contents", contextMenuItem: "cm_Item" });
export const gamepadUIClasses = menuChanged ? undefined : { MainNavMenuAnchor: "mm_Anchor" };
export const mainMenuAppRunningClasses = menuChanged ? undefined : { NavigationColumn: "mm_NavColumn", NavigationBox: "mm_NavBox", NavigationMenuItem: "mm_Item",
  Active: "mm_Active", NavigationMenuItemSeparator: "mm_Sep", AppColumn: "mm_AppColumn", CurrentGameBackground: "mm_GameBg", CurrentGameLogo: "mm_Logo", AppRunningControls: "mm_Controls" };
export function findClassModule(f) { return modules.find((m) => { try { return f(m); } catch { return false; } }); }
export function findSP() { return window; }
export const Router = { WindowStore: { GamepadUIMainWindowInstance: { BrowserWindow: window }, SteamUIWindows: [] },
  get MainRunningApp() { return window.HC_RUNNING ? { appid: 1001, display_name: "Ember Knights of Ash" } : undefined; } };
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
