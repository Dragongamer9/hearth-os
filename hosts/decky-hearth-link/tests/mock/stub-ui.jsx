// CI only (render-panel.mjs): a stand-in for @decky/ui so Hearth Link's real Quick Access panel can be drawn in a plain
// browser, roughly the way Steam draws these building blocks (render-panel.mjs styles the classes). A dropdown here
// picks its first option when pressed (Steam opens a menu).
import { createElement as h } from "react";

export const staticClasses = { Text: "Text", Title: "Title" };
export const PanelSection = (p) => h("div", { className: "qam-section" }, p.title ? h("div", { className: "qam-section-title" }, p.title) : null, p.children);
export const PanelSectionRow = (p) => h("div", { className: "qam-row" }, p.children);
export const Field = (p) => h("div", { className: "qam-field qam-kv" },
  h("div", { className: "qam-line" }, h("span", { className: "qam-label" }, p.label), p.children ? h("span", { className: "qam-side" }, p.children) : null),
  p.description ? h("div", { className: "qam-desc" }, p.description) : null);
export const ButtonItem = (p) => h("div", { className: "qam-field" + (p.layout === "below" ? "" : " qam-inline") },
  h("button", { className: "DialogButton qam-btn", onClick: p.onClick, disabled: !!p.disabled }, p.children),
  p.description && p.layout === "below" ? h("div", { className: "qam-desc" }, p.description) : null);
export const DropdownItem = (p) => {
  const sel = (p.rgOptions || []).find((o) => o.data === p.selectedOption);
  return h("div", { className: "qam-field" }, h("div", { className: "qam-label" }, p.label),
    h("button", { className: "DialogButton qam-btn qam-drop", onClick: () => p.rgOptions?.[0] && p.onChange?.(p.rgOptions[0]) }, (sel ? sel.label : p.strDefaultLabel) + "  ▾"));
};
export const TextField = (p) => h("div", { className: "qam-field" }, h("div", { className: "qam-label" }, p.label),
  h("input", { className: "qam-input", value: p.value, onChange: p.onChange }));
export const ProgressBarWithInfo = (p) => h("div", { className: "qam-field qam-prog" },
  h("div", { className: "qam-line" }, h("span", { className: "qam-label" }, p.label), h("span", { className: "qam-desc" }, p.indeterminate ? "" : `${p.nProgress}%`)),
  h("div", { className: "qam-bar" + (p.indeterminate ? " ind" : "") }, h("i", { style: { width: `${p.indeterminate ? 30 : p.nProgress}%` } })),
  p.sOperationText ? h("div", { className: "qam-desc" }, p.sOperationText) : null,
  p.description ? h("div", { className: "qam-desc" }, p.description) : null);
export const ToggleField = (p) => h("div", { className: "qam-field qam-kv" },
  h("div", { className: "qam-line" }, h("span", { className: "qam-label" }, p.label),
    h("button", { className: "DialogButton qam-toggle" + (p.checked ? " on" : ""), role: "switch", "aria-checked": !!p.checked, disabled: !!p.disabled,
      onClick: () => p.onChange?.(!p.checked) }, p.checked ? "On" : "Off")),
  p.description ? h("div", { className: "qam-desc" }, p.description) : null);
