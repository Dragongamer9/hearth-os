// Playing and opening from the takeover's pages: Steam's games on Steam's own page, Hearth's through Hearth (a Steam
// shortcut running `nimbus --play-now`, so Steam gives it the screen, the overlay and play time), Hearth's apps as
// Steam shortcuts too.
import { callable, toaster } from "@decky/api";
import { Navigation } from "@decky/ui";
import { Game, validId } from "./model";
import { Item, play } from "./shortcuts";

const getTarget = callable<[id: string], { ok: boolean; error?: string; exe?: string; startDir?: string; options?: string }>("get_target");
const getLink = callable<[id: string], Item | null>("get_link");

export function say(body: string) { try { toaster.toast({ title: "Hearth", body }); } catch { console.log("Hearth:", body); } }

/** Opens one of Hearth's apps (hearth-app:store, :mods, :theater, :hearth, :settings). */
export async function openApp(id: string) {
  try {
    const l = await getLink(id);
    if (!l) { say("That part of Hearth isn't installed here"); return; }
    await play(l);
  } catch (e) { say(`Couldn't open it: ${e}`); }
}

/** A Steam game's page in Steam (Steam's own Play, achievements, settings). */
export function steamPage(appId: string | number) {
  try { Navigation.Navigate(`/library/app/${appId}`); } catch (e) { say(`Couldn't open Steam's page: ${e}`); }
}

/** Plays a game: Steam's from Steam's page, Hearth's through Hearth. Not installed: Hearth opens to install it. */
export async function playGame(g: Game) {
  if (!validId(g.id)) return;
  if (g.source === "steam") { steamPage(g.id.slice(6)); return; }
  if (!g.installed) { say(`${g.title} isn't installed: Hearth opens to install it`); await openApp("hearth-app:hearth"); return; }
  try {
    const t = await getTarget(g.id);
    if (!t.ok || !t.exe) { say(t.error || "Hearth can't play that here"); return; }
    await play({ id: g.id, title: g.title, source: g.source, store: g.store, cover: g.cover || "", wide: g.wide || "", icon: g.icon || "",
                 exe: t.exe, startDir: t.startDir || "", options: t.options || "" });
  } catch (e) { say(`Couldn't start ${g.title}: ${e}`); }
}

/** Hearth's page for a game (its facts and Play through Hearth); Steam's games open Steam's page. */
export function gamePage(g: Game) {
  if (g.source === "steam") { steamPage(g.id.slice(6)); return; }
  try { Navigation.Navigate(`/hearth/game/${encodeURIComponent(g.id)}`); } catch (e) { say(`Couldn't open it: ${e}`); }
}
