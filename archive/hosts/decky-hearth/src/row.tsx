// The older Hearth row over Steam's Home (when Hearth's home screen is off): Hearth's wallpaper and logo, then your
// Hearth games like Hearth's home. Each card is CoverArt (ui.tsx): the picture as an <img>, because Hearth's look over
// Steam paints buttons' backgrounds with !important and that hid every cover in 0.6.60 (blank dark-blue cards); the
// title is always shown, and a picture that fails gives way to the next one or to the title.
import { DialogButton, Focusable } from "@decky/ui";
import { ReactNode } from "react";
import { C, CoverArt } from "./ui";

export interface RowGame { id: string; title: string; store: string; cover?: string | null; wide?: string | null }

export function HearthRowView<T extends RowGame>(props: { games: T[]; brand: { logo?: string; wallpaper?: string } | null; onPlay: (g: T) => void; children?: ReactNode }) {
  const list = props.games;
  const brand = props.brand;
  if (!list.length) return <>{props.children}</>;
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minHeight: 0 }}>
      <div style={{ flex: "none", padding: "18px 2.8vw 10px", background: brand?.wallpaper
          ? `linear-gradient(180deg,rgba(12,16,22,.25),rgba(12,16,22,.95)),url("${brand.wallpaper}") center/cover` : C.bg2 }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10, marginBottom: 12 }}>
          {brand?.logo && <img src={brand.logo} style={{ width: 30, height: 30 }} />}
          <div style={{ fontSize: 15, fontWeight: 600, letterSpacing: ".02em", textTransform: "uppercase", color: C.dim }}>From Hearth</div>
        </div>
        <Focusable {...({ "flow-children": "horizontal" } as any)} style={{ display: "flex", gap: 14, overflow: "hidden" }}>
          {list.slice(0, 12).map((g) => (
            <DialogButton key={g.id} onClick={() => props.onPlay(g)} className="hearth-row-card"
              style={{ flex: "none", width: 120, height: 170, minWidth: 0, padding: 0, borderRadius: 10, overflow: "hidden", position: "relative" }}>
              <CoverArt game={g} shape="portrait" badge="top" where="home row" />
            </DialogButton>
          ))}
        </Focusable>
      </div>
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>{props.children}</div>
    </div>
  );
}
