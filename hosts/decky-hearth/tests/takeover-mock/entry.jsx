// CI only (render-takeover.mjs): draws the takeover's real pages (src/pages.tsx) with the stand-ins for Decky.
import { createRoot } from "react-dom/client";
import { HearthHome, LibraryPage, StorePage, setBrand } from "../../src/pages";

setBrand({ css: "", logo: window.HEARTH_LOGO, wallpaper: window.HEARTH_WALL });
const steamHome = <div style={{ padding: 60, color: "#fff" }}>Steam's own Home (shown if Hearth's can't be)</div>;
const page = { home: <HearthHome steam={steamHome} />, store: <StorePage />, library: <LibraryPage /> }[window.HEARTH_PAGE] || steamHome;
createRoot(document.getElementById("root")).render(page);
