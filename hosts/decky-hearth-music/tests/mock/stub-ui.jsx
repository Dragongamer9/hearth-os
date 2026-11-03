// CI only (render-panel.mjs): a stand-in for @decky/ui so Hearth Music's real Quick Access panel can be drawn in a plain
// browser, roughly the way Steam draws these building blocks (render-panel.mjs styles the classes).
import { createElement as h } from "react";
import "./steam-state.js";

export const staticClasses = { Text: "Text", Title: "Title" };
export const Navigation = { NavigateToExternalWeb: (u) => { window.HM_OPENED = u; } };
export const Router = window.HM_ROUTER;
export const findSP = () => window;
export const PanelSection = (p) => h("div", { className: "qam-section" }, p.title ? h("div", { className: "qam-section-title" }, p.title) : null, p.children);
export const PanelSectionRow = (p) => h("div", { className: "qam-row" }, p.children);
export const ToggleField = (p) => h("div", { className: "qam-field" },
  h("div", { className: "qam-line" }, h("span", { className: "qam-label" }, p.label), h("span", { className: "qam-switch" + (p.checked ? " on" : "") }, h("i"))),
  p.description ? h("div", { className: "qam-desc" }, p.description) : null);
export const SliderField = (p) => h("div", { className: "qam-field" },
  h("div", { className: "qam-line" }, h("span", { className: "qam-label" }, p.label), p.showValue ? h("span", { className: "qam-val" }, `${p.value}${p.valueSuffix || ""}`) : null),
  h("div", { className: "qam-slider" }, h("b", { style: { width: `${((p.value - p.min) / (p.max - p.min)) * 100}%` } })));
export const ButtonItem = (p) => h("div", { className: "qam-field" },
  h("button", { className: "DialogButton qam-btn", onClick: p.onClick, disabled: !!p.disabled }, p.children),
  p.description ? h("div", { className: "qam-desc" }, p.description) : null);
export const TextField = (p) => h("div", { className: "qam-field" },
  p.label ? h("span", { className: "qam-label" }, p.label) : null,
  h("input", { className: "qam-input", value: p.value, onChange: p.onChange }));
