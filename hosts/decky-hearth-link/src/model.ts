// Hearth Link's pure parts (no Steam, no Decky): the lines the panel shows and which buttons each thing gets. Unit-
// tested in CI with Node's type stripping (tests/model.test.mjs), so plain TypeScript only (no enums).

export interface Peer { id: string; name: string; online: boolean; lan: boolean | null; lastSeen: number }
export interface Job {
  id: string; peer: string; game: string; title: string; store: string; state: string; total: number | null; done: number | null;
  speed: number | null; eta: number | null; error: string; from: string; created: number | null; relay: boolean;
}
export interface Server { id: string; name: string; free: number | null; usable: number | null; seen: number | null; canWake: boolean }
export interface Offer { id: string; from: string; title: string; store: string; size: number }
export interface Status {
  running: true; me: { id: string; name: string; kind: string }; peers: Peer[];
  nearby: { id: string; name: string; pairing: boolean }[];
  pairing: { code: string | null; until: number; netCode: string | null; netUntil: number } | null; shortCodes: boolean;
  joining: { state: string; name: string } | null; pairEvent: { name: string; ok: boolean; error: string } | null;
  transfers: Job[]; offers: Offer[]; sending: { to: string; title: string; bytes: number }[]; playing: boolean; lastChange: number;
  storage: { server: boolean; savesFollow: boolean; servers: Server[]; jobs: Job[] };
}
export interface Down {
  running: false; installed: boolean; error: string;
  last: { at: number; status: { me?: Status["me"]; peers?: Peer[]; storage?: Status["storage"] } } | null;
}
export type Answer = Status | Down;
export interface Game { id: string; title: string; store: string; size: number }
export interface Health {
  devices: { name: string; me: boolean; online: boolean; lastSeen: number; free: number | null; total: number | null; battery: number | null;
             version: string; upToDate: boolean | null; lastBackup: number | null; server: boolean }[];
  serverName: string; serverOffline: boolean; error?: string;
}

export const POLL = 3000;           // the status while the panel is open
export const STORAGE_POLL = 20000;  // Hearth Storage's details
export const LOW_BATTERY = 20;      // percent
export const LOW_SPACE = 0.1;       // of the drive
export const OLD_BACKUP = 7 * 24 * 3600 * 1000;

/** "12 minutes ago" style. */
export function ago(then: number | null | undefined, now: number): string {
  if (!then) return "never";
  const s = Math.max(0, Math.round((now - then) / 1000));
  if (s < 60) return "just now";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} minute${m === 1 ? "" : "s"} ago`;
  const h = Math.round(m / 60);
  if (h < 24) return `${h} hour${h === 1 ? "" : "s"} ago`;
  const d = Math.round(h / 24);
  return `${d} day${d === 1 ? "" : "s"} ago`;
}

export function bytes(n: number | null | undefined): string {
  if (!n || n < 0) return "0 B";
  const u = ["B", "KB", "MB", "GB", "TB"];
  let i = 0, v = n;
  while (v >= 1000 && i < u.length - 1) { v /= 1000; i++; }
  return `${v >= 10 || i === 0 ? Math.round(v) : v.toFixed(1)} ${u[i]}`;
}

export function duration(s: number | null | undefined): string {
  if (s === null || s === undefined || s < 0) return "";
  if (s < 60) return "less than a minute";
  const m = Math.round(s / 60);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  return `${h} h ${m % 60} min`;
}

/** A device's line: online on this network / over the internet, or when it was last seen. */
export function deviceLine(p: Peer, now: number): string {
  if (!p.online) return `Offline · last seen ${ago(p.lastSeen, now)}`;
  if (p.lan === true) return "Online · on this network";
  if (p.lan === false) return "Online · over the internet";
  return "Online";
}

/** The Hearth Storage device a paired device is, if it is one. */
export const serverOf = (p: Peer, servers: Server[]): Server | undefined => servers.find((s) => s.id === p.id);

/** Wake is offered for a sleeping device that Hearth Link knows how to wake (a Hearth Storage device it saw before). */
export function canWake(p: Peer, servers: Server[]): boolean {
  return !p.online && !!serverOf(p, servers)?.canWake;
}

export const percent = (j: Job): number => (j.total && j.total > 0 ? Math.min(100, Math.round(((j.done || 0) * 100) / j.total)) : 0);

const STATE: Record<string, string> = {
  waiting: "Waiting", copying: "Copying", paused: "Paused", done: "Done", cancelled: "Cancelled", error: "Didn't finish",
  preparing: "Getting ready", comparing: "Checking what's there", finishing: "Finishing", interrupted: "Stopped (carries on by itself)",
};
export const stateName = (s: string): string => STATE[s] || (s ? s[0].toUpperCase() + s.slice(1) : "");

/** The progress line under a copy. */
export function jobLine(j: Job): string {
  const parts = [stateName(j.state)];
  if (j.total) parts.push(`${bytes(j.done)} of ${bytes(j.total)}`);
  if (j.state === "copying" && j.speed) parts.push(`${bytes(j.speed)}/s`);
  if (j.state === "copying" && j.eta) parts.push(`${duration(j.eta)} left`);
  if (j.relay && j.state === "copying") parts.push("over the internet");
  if (j.state === "error" && j.error) parts.push(j.error);
  return parts.join(" · ");
}

/** Which buttons a copy gets (hearth-link transfer.pause / resume / cancel; clear for finished ones). */
export function jobActions(j: Job): ("pause" | "resume" | "cancel" | "clear")[] {
  switch (j.state) {
    case "waiting": case "preparing": case "comparing": case "copying": case "finishing": return ["pause", "cancel"];
    case "paused": case "error": case "interrupted": return ["resume", "cancel"];
    case "done": case "cancelled": return ["clear"];
    default: return ["cancel"];
  }
}

export const active = (j: Job): boolean => !["done", "cancelled"].includes(j.state);

/** After a copy: how the game shows up in Steam. Steam games come with their appmanifest, which Steam reads when it
 *  starts; other games are Hearth's, and Hearth Sync adds them at its next check. */
export function afterCopy(j: Job): { steam: boolean; text: string } {
  if (j.store === "steam") return { steam: true, text: "In your Steam library after Steam restarts" };
  return { steam: false, text: "Hearth Sync adds it to Steam's library at its next check (a few minutes)" };
}

/** Games that can be sent: real games (not Hearth Storage's backups, archives or shared folders). */
export function sendable(games: Game[]): Game[] {
  return games.filter((g) => !["share", "archive", "backup", "cache"].includes(g.store) && !/^(share|archive|backup|cache|saves?):/.test(g.id));
}

export const hasStorage = (s: Status): boolean => s.storage.server || s.storage.servers.length > 0;

/** "4:32" left of a pairing code. */
export function left(until: number, now: number): string {
  const s = Math.max(0, Math.round((until - now) / 1000));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** The 6 numbers with a space in the middle, easier to read from across the room. */
export const showCode = (c: string | null): string => (c && /^\d{6}$/.test(c) ? `${c.slice(0, 3)} ${c.slice(3)}` : c || "");

export const lanCodeOk = (c: string): boolean => /^\d{6}$/.test(c.replace(/\s/g, ""));
export const netCodeOk = (c: string): boolean => /^[A-Za-z0-9 -]{4,24}$/.test(c.trim()) && !lanCodeOk(c);

/** Device health in a few lines: how many are on, and what needs a look. */
export function healthSummary(h: Health, now: number): { line: string; warnings: string[] } {
  const n = h.devices.length, on = h.devices.filter((d) => d.online).length;
  const warnings: string[] = [];
  for (const d of h.devices) {
    if (d.battery !== null && d.battery < LOW_BATTERY && d.online) warnings.push(`${d.name}: battery ${d.battery}%`);
    if (d.free !== null && d.total && d.free / d.total < LOW_SPACE) warnings.push(`${d.name}: only ${bytes(d.free)} free`);
    if (d.upToDate === false) warnings.push(`${d.name}: Hearth OS ${d.version || ""} has an update`.replace("  ", " "));
    if (d.lastBackup !== null && d.lastBackup > 0 && now - d.lastBackup > OLD_BACKUP) warnings.push(`${d.name}: last backup ${ago(d.lastBackup, now)}`);
  }
  return { line: `${n} device${n === 1 ? "" : "s"} · ${on} online${h.serverOffline ? ` · ${h.serverName || "Hearth Storage"} is asleep` : ""}`, warnings };
}
