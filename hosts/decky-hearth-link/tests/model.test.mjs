// Unit tests of Hearth Link's pure parts (src/model.ts), run in CI with Node's own test runner and type stripping:
//   node --experimental-strip-types --test hosts/decky-hearth-link/tests/model.test.mjs
import { test } from "node:test";
import assert from "node:assert/strict";
import * as M from "../src/model.ts";

const peer = (x = {}) => ({ id: "b".repeat(64), name: "Desk", online: true, lan: true, lastSeen: 0, ...x });
const job = (x = {}) => ({ id: "01", peer: "b", game: "steam:620", title: "Portal 2", store: "steam", state: "copying", total: 10e9, done: 4e9,
  speed: 85e6, eta: 70, error: "", from: "Desk", created: 0, relay: false, ...x });

test("a device's line: on this network, over the internet, or last seen", () => {
  const now = 10 * 3600e3;
  assert.equal(M.deviceLine(peer(), now), "Online · on this network");
  assert.equal(M.deviceLine(peer({ lan: false }), now), "Online · over the internet");
  assert.equal(M.deviceLine(peer({ lan: null }), now), "Online", "an older Hearth Link doesn't say");
  assert.equal(M.deviceLine(peer({ online: false, lastSeen: now - 2 * 3600e3 }), now), "Offline · last seen 2 hours ago");
  assert.equal(M.deviceLine(peer({ online: false, lastSeen: 0 }), now), "Offline · last seen never");
});

test("Wake only for a sleeping Hearth Storage device Hearth Link knows how to wake", () => {
  const servers = [{ id: "b".repeat(64), name: "Desk", free: 1, usable: 1, seen: 1, canWake: true }];
  assert.equal(M.canWake(peer({ online: false }), servers), true);
  assert.equal(M.canWake(peer({ online: true }), servers), false, "it's on");
  assert.equal(M.canWake(peer({ online: false }), [{ ...servers[0], canWake: false }]), false, "no address to wake yet");
  assert.equal(M.canWake(peer({ online: false, id: "c" }), servers), false, "not a storage device");
});

test("copies: progress line, percent and buttons", () => {
  assert.equal(M.percent(job()), 40);
  assert.equal(M.percent(job({ total: 0 })), 0);
  assert.equal(M.jobLine(job()), "Copying · 4.0 GB of 10 GB · 85 MB/s · 1 min left");
  assert.match(M.jobLine(job({ relay: true })), /over the internet$/);
  assert.equal(M.jobLine(job({ state: "error", error: "Not enough space" })), "Didn't finish · 4.0 GB of 10 GB · Not enough space");
  assert.deepEqual(M.jobActions(job()), ["pause", "cancel"]);
  assert.deepEqual(M.jobActions(job({ state: "paused" })), ["resume", "cancel"]);
  assert.deepEqual(M.jobActions(job({ state: "interrupted" })), ["resume", "cancel"]);
  assert.deepEqual(M.jobActions(job({ state: "done" })), ["clear"]);
  assert.equal(M.active(job({ state: "done" })), false);
  assert.equal(M.active(job({ state: "paused" })), true);
});

test("after a copy: Steam games after Steam restarts, others through Hearth Sync", () => {
  assert.equal(M.afterCopy(job()).steam, true);
  const o = M.afterCopy(job({ store: "gog" }));
  assert.equal(o.steam, false);
  assert.match(o.text, /Hearth Sync/);
});

test("only real games can be sent", () => {
  const g = (id, store) => ({ id, title: id, store, size: 1 });
  assert.deepEqual(M.sendable([g("steam:620", "steam"), g("share:x:y", "share"), g("archive:1", "archive"), g("gog:1", "gog")]).map((x) => x.id), ["steam:620", "gog:1"]);
});

test("pairing codes", () => {
  assert.equal(M.showCode("123456"), "123 456");
  assert.equal(M.showCode(null), "");
  assert.equal(M.left(65_000, 0), "1:05");
  assert.equal(M.left(0, 5), "0:00");
  assert.equal(M.lanCodeOk("123 456"), true);
  assert.equal(M.lanCodeOk("12345"), false);
  assert.equal(M.netCodeOk("AB12-CD34"), true);
  assert.equal(M.netCodeOk("123456"), false, "that's a 6-digit code");
  assert.equal(M.netCodeOk('a"b'), false);
});

test("device health summary and warnings", () => {
  const now = 100 * 24 * 3600e3;
  const d = (x) => ({ name: "Ally", me: false, online: true, lastSeen: 0, free: 50, total: 100, battery: 80, version: "1.0", upToDate: true, lastBackup: null, server: false, ...x });
  const h = M.healthSummary({ devices: [d({ me: true }), d({ name: "Deck", battery: 12 }), d({ name: "Tower", online: false, free: 5, upToDate: false, version: "0.9", lastBackup: now - 10 * 24 * 3600e3 })], serverName: "Tower", serverOffline: true }, now);
  assert.equal(h.line, "3 devices · 2 online · Tower is asleep");
  assert.deepEqual(h.warnings, ["Deck: battery 12%", "Tower: only 5 B free", "Tower: Hearth OS 0.9 has an update", "Tower: last backup 10 days ago"]);
  assert.deepEqual(M.healthSummary({ devices: [d({})], serverName: "", serverOffline: false }, now).warnings, []);
});

test("sizes and times", () => {
  assert.equal(M.bytes(0), "0 B");
  assert.equal(M.bytes(1500), "1.5 KB");
  assert.equal(M.bytes(250e9), "250 GB");
  assert.equal(M.duration(30), "less than a minute");
  assert.equal(M.duration(3 * 3600 + 600), "3 h 10 min");
  assert.equal(M.ago(0, 5), "never");
});

test("sync switches: what each says, and Steam Cloud is mentioned with saves", () => {
  const off = M.syncLines({ plugins: false, settings: false }, false, false);
  assert.match(off.plugins, /^Off/);
  assert.match(off.settings, /^Off/);
  assert.match(off.saves, /^Off/);
  assert.match(off.saves, /Steam Cloud games already sync through Steam/);
  assert.match(off.saves, /Needs a Hearth Storage device/);
  const on = M.syncLines({ plugins: true, settings: true }, true, true);
  assert.match(on.plugins, /never sign-ins/);
  assert.match(on.settings, /Screen modes, sound devices, power and network stay on each device/);
  assert.ok(!on.saves.includes("Needs a Hearth Storage"));
});

test("a plugin from another device goes to Decky's own install, only as a checked local zip", () => {
  const a = { folder: "CSS Loader", name: "CSS Loader", version: "2.1.0", from: "b", fromName: "Desk", online: true };
  assert.equal(M.availableLine(a), "Version 2.1.0 · on Desk");
  assert.equal(M.availableLine({ ...a, version: "", online: false }), "on Desk (offline)");
  const ok = { ok: true, artifact: "file:///home/you/homebrew/settings/decky-hearth-link/incoming/CSS Loader.zip", name: "CSS Loader", version: "2.1.0", hash: "f".repeat(64) };
  assert.deepEqual(M.installArgs(ok), [ok.artifact, "CSS Loader", "2.1.0", "f".repeat(64), 0]);
  assert.equal(M.installArgs({ ...ok, artifact: "https://evil.example/x.zip" }), null);
  assert.equal(M.installArgs({ ...ok, hash: "nope" }), null);
  assert.equal(M.installArgs({ ok: false, error: "x" }), null);
});

test("home server tile: shown only on a home server, and its lines", () => {
  const q = (x = {}) => ({ available: true, host: "nvidia-pc", lightMode: true, playing: true, resumed: false, paused: ["backups"], shares: 3, people: 1,
    usage: { used: 1.2e12, total: 4e12 }, backups: { count: 1, lastOk: 0, waiting: [], failed: [], running: false }, alerts: [], setupDone: true, ...x });
  assert.equal(M.homeShown(q()), true);
  assert.equal(M.homeShown(q({ setupDone: false, shares: 0, backups: { count: 0, lastOk: 0, waiting: [], failed: [], running: false } })), false, "not a home server");
  assert.equal(M.homeShown(q({ available: false })), false, "no helper");
  assert.equal(M.homeShown(null), false);
  assert.equal(M.homeLine(q()), "3 shared folders · 1.2 TB of 4.0 TB used · 1 person");
  assert.equal(M.homeLine(q({ shares: 1, people: 0, usage: { used: 0, total: 0 } })), "1 shared folder");
  const now = 10 * 3600e3;
  assert.equal(M.backupLine(q({ backups: { count: 1, lastOk: now - 2 * 3600e3, waiting: [], failed: [], running: false } }), now), "Last backup 2 hours ago");
  assert.equal(M.backupLine(q({ backups: { count: 1, lastOk: 0, waiting: ["Games"], failed: [], running: false } }), now), "Waiting for your game: Games");
  assert.equal(M.backupLine(q({ backups: { count: 2, lastOk: 0, waiting: [], failed: ["Cloud"], running: false } }), now), "Didn't finish: Cloud");
  assert.equal(M.backupLine(q({ backups: { count: 1, lastOk: 0, waiting: [], failed: [], running: true } }), now), "Backing up now");
  assert.equal(M.backupLine(q({ backups: { count: 0, lastOk: 0, waiting: [], failed: [], running: false } }), now), "No backups yet");
  assert.equal(M.pausedLine(q({ paused: ["snapshots", "backups"] })), "Paused for your game: snapshots · backups");
  assert.equal(M.pausedLine(q({ paused: [] })), "");
  assert.equal(M.aiLine(q()), "", "no AI service (older helper)");
  assert.equal(M.aiLine(q({ ai: { on: false, state: "off" } })), "");
  assert.equal(M.aiLine(q({ ai: { on: true, state: "unloaded" } })), "AI service: models unloaded while you play");
  assert.equal(M.aiLine(q({ ai: { on: true, state: "paused" } })), "AI service: stopped while you play");
  assert.equal(M.aiLine(q({ ai: { on: true, state: "ready" } })), "AI service: ready");
});
