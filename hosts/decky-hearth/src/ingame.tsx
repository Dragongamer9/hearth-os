// Hearth for the game you're playing, as a page of its own (/hearth/now, from Steam's main menu while a game runs and
// from Quick Access › Hearth): the game's picture, how it plays (through Hearth or Steam), its Hearth Input layout,
// its Proton for Play through Hearth, its mods in Hearth Mods and Hearth's facts. And Hearth's home as a page
// (/hearth/home) for Steam's main menu when Hearth's home screen isn't taking Steam's Home over.
import { PanelSection } from "@decky/ui";
import { C, CoverArt, Page, go } from "./ui";
import { ThisGame } from "./qam";
import { useFront } from "./core";
import { HearthHome, brand } from "./pages";

export function NowPlayingPage() {
  const { run } = useFront();
  const wall = brand()?.wallpaper || "";
  if (!run) {
    return <Page title="Now playing" wallpaper={wall}><div className="ht-dim">No game is running. Start one from Steam's library or Hearth's.</div></Page>;
  }
  const g = { id: run.id || "", title: run.title, store: run.hearth ? "Hearth" : "Steam" };
  return (
    <Page wallpaper={wall}>
      <div style={{ display: "flex", gap: 24, alignItems: "flex-start", flexWrap: "wrap" }}>
        <div style={{ position: "relative", width: 300, height: 150, borderRadius: 12, overflow: "hidden", background: C.panel2, flex: "none" }}>
          <CoverArt game={g} shape="wide" badge="none" where="now-playing" />
        </div>
        <div style={{ flex: 1, minWidth: 280 }}>
          <div style={{ fontSize: 12, fontWeight: 700, letterSpacing: ".08em", textTransform: "uppercase", color: C.accent }}>{run.hearth ? "Plays through Hearth" : "Steam game"}</div>
          <div style={{ fontSize: 28, fontWeight: 700, margin: "4px 0 12px" }}>{run.title || "This game"}</div>
          <div className="ht-card" style={{ maxWidth: 560 }}>
            <PanelSection><ThisGame header={false} /></PanelSection>
          </div>
        </div>
      </div>
    </Page>
  );
}

export function HearthHomePage() {
  return <HearthHome steam={<Page title="Hearth Home"><div className="ht-dim">Hearth didn't answer. <span style={{ color: C.accent }} onClick={() => go("/library/home")}>Steam's Home</span> still works.</div></Page>} />;
}
