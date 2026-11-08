// CI only (render-panel.mjs): Hearth Music's real Quick Access panel (src/index.tsx) in a stand-in Quick Access frame,
// running its real loop (src/music.ts) against a stand-in of Steam's state (stub-api.js).
import { createRoot } from "react-dom/client";
import { createElement as h } from "react";
import plugin from "../../src/index.tsx";

const p = plugin();
if (window.HM_PANEL !== false) {
  createRoot(document.getElementById("root")).render(
    h("div", { className: "qam" },
      h("div", { className: "qam-tabs" }, h("div", { className: "qam-tab" }, "⚙"), h("div", { className: "qam-tab on" }, p.icon), h("div", { className: "qam-tab" }, "⟳")),
      h("div", { className: "qam-body" }, h("div", { className: "qam-title" }, p.titleView), h("div", { className: "ScrollPanel" }, p.content))));
}
