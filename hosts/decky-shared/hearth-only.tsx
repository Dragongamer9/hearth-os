// Shared by Hearth's Decky plugins (docs/PLUGIN-UPDATES.md, "Hearth OS only"). The canonical copy is
// hosts/decky-shared/hearth-only.tsx; each plugin has an identical src/hearth-only.tsx (its build resolves @decky/* from
// its own folder), and hosts/linux/test/test_plugin_updates.py fails when a copy differs.
//
// The backend's hearth_os() says whether this is Hearth OS. When it isn't, the plugin's panel is replaced by one small
// invitation to Hearth OS and nothing else of the plugin starts. A check, not DRM: no pop-ups, no nagging, nothing sent.
import { ButtonItem, Navigation, PanelSection, PanelSectionRow } from "@decky/ui";
import { callable } from "@decky/api";
import { ReactNode, useEffect, useState } from "react";

const ask = callable<[], { ok?: boolean } | null>("hearth_os");
let answer: Promise<boolean> | null = null;

// Asked once per load. Anything but a clear "not Hearth OS" counts as Hearth OS, so a hiccup never hides the plugin.
export function hearthOs(): Promise<boolean> {
  if (!answer) answer = ask().then((r) => !(r && r.ok === false)).catch(() => true);
  return answer;
}

const URL = "https://hearthos.fun";

function Flame() {
  return (
    <svg viewBox="0 0 512 512" width="44" height="44" aria-hidden="true" style={{ flex: "none" }}>
      <defs>
        <linearGradient id="hoFlame" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#ffd166" /><stop offset=".55" stopColor="#ff8a3d" /><stop offset="1" stopColor="#e5483b" />
        </linearGradient>
        <linearGradient id="hoCore" x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" stopColor="#fff6d6" /><stop offset="1" stopColor="#ffc24f" />
        </linearGradient>
      </defs>
      <path d="M112 432 V252 A144 144 0 0 1 400 252 V432" fill="none" stroke="#e8dccd" strokeWidth="30" strokeLinecap="round" />
      <path d="M76 462 H436" stroke="#e8dccd" strokeWidth="26" strokeLinecap="round" />
      <path d="M256 168 C298 222 340 262 340 332 C340 386 302 422 256 422 C210 422 172 386 172 332 C172 294 190 266 212 244 C210 276 222 298 242 306 C232 258 236 212 256 168 Z" fill="url(#hoFlame)" />
      <path d="M258 284 C280 314 300 338 300 368 C300 394 282 412 258 412 C234 412 216 394 216 368 C216 340 238 316 258 284 Z" fill="url(#hoCore)" />
    </svg>
  );
}

const GIVES = ["Steam Game Mode, like SteamOS", "Mantel, a full desktop", "Updates that can always go back"];

export function HearthOsPanel({ name, what }: { name: string; what: string }) {
  return (
    <PanelSection>
      <PanelSectionRow>
        <div style={{ display: "flex", gap: "12px", alignItems: "center", padding: "4px 0 2px" }}>
          <Flame />
          <div style={{ minWidth: 0 }}>
            <div style={{ fontSize: "16px", fontWeight: 700, color: "#fff" }}>{name}</div>
            <div style={{ fontSize: "12px", color: "#b8bcbf", lineHeight: 1.35 }}>{what}</div>
          </div>
        </div>
      </PanelSectionRow>
      <PanelSectionRow>
        <div style={{ fontSize: "13px", color: "#dcdedf", lineHeight: 1.4, padding: "2px 0" }}>
          This plugin is part of Hearth OS, the free gaming OS:
          <ul style={{ margin: "6px 0 0", paddingLeft: "18px", color: "#b8bcbf" }}>
            {GIVES.map((g) => <li key={g}>{g}</li>)}
          </ul>
        </div>
      </PanelSectionRow>
      <PanelSectionRow>
        <ButtonItem layout="below" onClick={() => { try { (Navigation as any).NavigateToExternalWeb(URL); } catch { /* Steam's browser isn't reachable */ } }}>
          Get Hearth OS
        </ButtonItem>
      </PanelSectionRow>
      <PanelSectionRow>
        <div style={{ fontSize: "11px", color: "#8b929a", textAlign: "center", padding: "2px 0 4px" }}>Free and open source · hearthos.fun</div>
      </PanelSectionRow>
    </PanelSection>
  );
}

// The plugin's panel on Hearth OS; the invitation anywhere else (nothing shown until the backend has answered).
export function HearthOnly({ name, what, children }: { name: string; what: string; children: ReactNode }) {
  const [ok, setOk] = useState<boolean | null>(null);
  useEffect(() => { let live = true; hearthOs().then((v) => { if (live) setOk(v); }); return () => { live = false; }; }, []);
  if (ok === null) return null;
  return ok ? <>{children}</> : <HearthOsPanel name={name} what={what} />;
}
