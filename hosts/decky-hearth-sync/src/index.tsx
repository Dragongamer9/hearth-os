// Hearth Sync (docs/HEARTH-SYNC.md): keeps Hearth's non-Steam games (Epic, GOG, Battle.net, Xbox, emulators, recomps,
// Beacon, games added by hand) in Steam's library as shortcuts with their pictures, in a collection per store. Syncs
// when Steam Game Mode starts and checks every few minutes; the Quick Access panel shows how it went. No theme, no
// menus, nothing of Steam's screens is changed.
import { ButtonItem, PanelSection, PanelSectionRow, ToggleField, staticClasses } from "@decky/ui";
import { definePlugin } from "@decky/api";
import { useEffect, useState } from "react";
import { FaSyncAlt } from "react-icons/fa";
import { CHECK_EVERY, Item, Status, ago, storeList, storeOn } from "./model";
import { ImportSection } from "./import";
import { HearthOnly, hearthOs } from "./hearth-only";
import { Settings, getSettings, getStatus, isBusy, listGames, onChange, removeAll, runSync, setSettings } from "./sync";

function Panel() {
  const [st, setSt] = useState<Status | null>(null);
  const [settings, setLocal] = useState<Settings>({ stores: {} });
  const [games, setGames] = useState<Item[]>([]);
  const [busy, setBusy] = useState(isBusy());
  const [msg, setMsg] = useState("");
  const [confirm, setConfirm] = useState(false);
  const [now, setNow] = useState(Date.now());
  const reload = () => {
    getStatus().then(setSt).catch(() => {});
    getSettings().then((s) => setLocal(s || { stores: {} })).catch(() => {});
    listGames().then(setGames).catch(() => {});
  };
  useEffect(() => {
    reload();
    const off = onChange(() => { setBusy(isBusy()); getStatus().then(setSt).catch(() => {}); });
    const t = setInterval(() => setNow(Date.now()), 30000);
    return () => { off(); clearInterval(t); };
  }, []);
  const syncNow = async () => { setMsg(""); await runSync(true); reload(); };
  const toggle = async (key: string, on: boolean) => {
    const s = await setSettings({ stores: { ...settings.stores, [key]: on } });
    setLocal(s);
    await runSync(true);
    reload();
  };
  const remove = async () => {
    if (!confirm) { setConfirm(true); return; }
    setConfirm(false);
    const n = await removeAll().catch(() => 0);
    setMsg(`Removed ${n} shortcut${n === 1 ? "" : "s"}. Sync now adds them again.`);
    reload();
  };
  const stores = storeList(games);
  return (
    <>
      <PanelSection title="Status">
        <PanelSectionRow>
          <div className={staticClasses.Text}>
            {st ? `${st.synced} game${st.synced === 1 ? "" : "s"} synced · last sync ${ago(st.lastSync, now)}` : "Reading…"}
          </div>
        </PanelSectionRow>
        {st?.off && <PanelSectionRow><div className={staticClasses.Text} style={{ color: "#ffb37a" }}>{st.off}</div></PanelSectionRow>}
        {(st?.problems || []).map((p, i) => <PanelSectionRow key={i}><div className={staticClasses.Text} style={{ fontSize: 12, color: "#ffb37a" }}>{p}</div></PanelSectionRow>)}
        {msg && <PanelSectionRow><div className={staticClasses.Text} style={{ fontSize: 12 }}>{msg}</div></PanelSectionRow>}
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={busy} onClick={syncNow} description="Adds new games, updates changed ones, removes ones that are gone">
            {busy ? "Syncing…" : "Sync now"}
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
      <PanelSection title="Stores">
        {!stores.length && <PanelSectionRow><div className={staticClasses.Text}>No games from Hearth's other stores are installed.</div></PanelSectionRow>}
        {stores.map(([key, name, n]) => (
          <PanelSectionRow key={key}>
            <ToggleField label={name} description={`${n} game${n === 1 ? "" : "s"}`} checked={storeOn(settings.stores, key)} disabled={busy}
              onChange={(on: boolean) => toggle(key, on)} />
          </PanelSectionRow>
        ))}
      </PanelSection>
      <ImportSection />
      <PanelSection title="Clean up">
        <PanelSectionRow>
          <ButtonItem layout="below" disabled={busy} onClick={remove}
            description="Only the shortcuts Hearth Sync made; your other non-Steam games stay">
            {confirm ? "Press again to remove them" : "Remove all Hearth Sync shortcuts"}
          </ButtonItem>
        </PanelSectionRow>
      </PanelSection>
    </>
  );
}

export default definePlugin(() => {
  let timer: any = null;
  let first: any = null;
  let gone = false;
  // Steam's library is ready a moment after the plugin loads: the first sync then, a cheap check every few minutes.
  // Only on Hearth OS (hearth-only.tsx); anywhere else nothing starts.
  hearthOs().then((ok) => {
    if (!ok || gone) return;
    first = setTimeout(() => { runSync(true).catch(() => {}); }, 15000);
    timer = setInterval(() => { runSync(false).catch(() => {}); }, CHECK_EVERY);
  });
  return {
    name: "Hearth Sync",
    titleView: <div className={staticClasses.Title}>Hearth Sync</div>,
    content: <HearthOnly name="Hearth Sync" what="Every store's games in Steam's library, with their pictures."><Panel /></HearthOnly>,
    icon: <FaSyncAlt />,
    onDismount() {
      gone = true;
      if (first) clearTimeout(first);
      if (timer) clearInterval(timer);
    },
  };
});
