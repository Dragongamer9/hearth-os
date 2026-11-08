// The YouTube mini player (docs/HEARTH-MUSIC.md): when a game's song is a YouTube video, YouTube's own embed
// (www.youtube-nocookie.com/embed/<id>) shows in the bottom-left corner of Steam Game Mode, small but at least
// 200x200, visible and not covered (Steam's Quick Access menu opens on the right), with YouTube's own controls and
// branding. It streams in YouTube's player: nothing of it is downloaded, extracted, cached or kept. It fades in and
// out with the volume the panel sets, the same way the previews do (player.ts), and closes when they would stop.
// It is never part of Steam's controller navigation: not focusable, and if it ever takes the focus (a touch), the
// focus goes straight back to Steam, so B/back behave as before.
import { useEffect, useRef, useState } from "react";
import { CROSSFADE, MINI, YT_EMBED_ORIGIN, fadeAt, ytErrorLine, ytMessage, ytVolume, youtubeEmbedUrl } from "./model";

interface Shown { key: string; id: string; seq: number; closing: boolean }

const READY_WAIT = 12000;   // no answer from YouTube's player by then: it can't play here

/** What the mini player shows, and the volume it fades; the component (MiniPlayer) draws it and talks to YouTube's
 *  player. Without the component on screen (Steam's page can't take it), nothing plays. */
class VideoControl {
  shown: Shown | null = null;
  vol = 0;               // the volume now (0-1), as it fades
  target = 0;
  mounted = 0;           // MiniPlayers on screen
  private seq = 0;
  private timer: any = null;
  private listeners = new Set<() => void>();
  private waiting: { seq: number; ok: () => void; fail: (e: Error) => void; timer: any } | null = null;
  send: ((func: string, args: unknown[]) => void) | null = null;
  onFail: (line: string) => void = () => {};

  on(f: () => void) { this.listeners.add(f); return () => { this.listeners.delete(f); }; }
  private changed() { this.listeners.forEach((f) => { try { f(); } catch { /* gone */ } }); }

  playing(): string { return this.shown && !this.shown.closing ? this.shown.key : ""; }

  /** Shows the video and fades it in; resolves when YouTube's player plays it, throws when it can't. */
  async play(key: string, id: string, volume: number, fadeMs = CROSSFADE): Promise<void> {
    for (let i = 0; i < 20 && !this.mounted; i++) await new Promise((r) => setTimeout(r, 100));   // just added to Steam's page
    if (!this.mounted) throw new Error("the mini player can't show on Steam's page");
    if (!youtubeEmbedUrl(id, "")) throw new Error("not a video");
    if (this.shown && !this.shown.closing && this.shown.id === id) { this.fade(volume, fadeMs); return; }
    this.cancelWait(new Error("another song"));
    const seq = ++this.seq;
    this.shown = { key, id, seq, closing: false };
    this.vol = 0;
    this.target = volume;
    this.changed();
    return new Promise<void>((ok, fail) => {
      this.waiting = { seq, ok: () => { ok(); }, fail, timer: setTimeout(() => this.failed(seq, "YouTube's player didn't answer (it may not load in Steam's page)"), READY_WAIT) };
    });
  }

  /** YouTube's player is ready: the volume set (0 first), playing, faded in. */
  ready(seq: number, fadeMs = CROSSFADE) {
    if (!this.shown || this.shown.seq !== seq) return;
    this.cmd("setVolume", [0]);
    this.cmd("unMute", []);
    this.cmd("playVideo", []);
    this.fade(this.target, fadeMs);
    if (this.waiting?.seq === seq) { clearTimeout(this.waiting.timer); const w = this.waiting; this.waiting = null; w.ok(); }
  }

  /** YouTube's player can't play it (an error code or no answer): closed at once, told to the log. */
  failed(seq: number, line: string) {
    if (!this.shown || this.shown.seq !== seq) return;
    clearInterval(this.timer);
    this.shown = null;
    this.changed();
    if (this.waiting?.seq === seq) { clearTimeout(this.waiting.timer); const w = this.waiting; this.waiting = null; w.fail(new Error(line)); }
    else this.onFail(line);
  }

  private cancelWait(e: Error) {
    if (this.waiting) { clearTimeout(this.waiting.timer); const w = this.waiting; this.waiting = null; w.fail(e); }
  }

  private cmd(func: string, args: unknown[]) { try { this.send?.(func, args); } catch { /* gone */ } }

  private fade(to: number, ms: number, done?: () => void) {
    clearInterval(this.timer);
    this.target = to;
    const from = this.vol, start = Date.now();
    this.timer = setInterval(() => {
      const el = Date.now() - start;
      this.vol = fadeAt(from, to, el, ms);
      this.cmd("setVolume", [ytVolume(this.vol)]);
      if (el >= ms) { clearInterval(this.timer); done?.(); }
    }, 80);
  }

  /** Fades out and closes (the player fades away with it). */
  stop(fadeMs = CROSSFADE) {
    this.cancelWait(new Error("stopped"));
    const s = this.shown;
    if (!s || s.closing) return;
    s.closing = true;
    this.changed();
    this.fade(0, fadeMs, () => {
      this.cmd("pauseVideo", []);
      if (this.shown === s) { this.shown = null; this.changed(); }
    });
  }

  setVolume(v: number) { if (this.shown && !this.shown.closing) this.fade(v, 250); else this.target = v; }
}

export const video = new VideoControl();

/** The window the mini player is drawn in. Decky's code runs in Steam's SharedJSContext, but global components are
 *  drawn in Steam's main window: YouTube's player talks to that window, so that's the one listened to. */
function hostWindow(el: Element | null): Window {
  return (el?.ownerDocument?.defaultView as Window) || window;
}

/** That page's origin (https://steamloopback.host in Game Mode), told to YouTube's embed. */
function pageOrigin(el: Element | null): string {
  try { const o = hostWindow(el).location.origin; return o.startsWith("https://") ? o : ""; } catch { return ""; }
}

/** The mini player itself, added to Steam's pages with Decky's routerHook.addGlobalComponent (index.tsx). */
export function MiniPlayer() {
  const [, redraw] = useState(0);
  const frame = useRef<HTMLIFrameElement | null>(null);
  const anchor = useRef<HTMLDivElement | null>(null);
  const [origin, setOrigin] = useState<string | null>(null);
  useEffect(() => {
    setOrigin(pageOrigin(anchor.current));
    video.mounted++;
    const off = video.on(() => redraw((n) => n + 1));
    return () => { video.mounted--; off(); video.send = null; };
  }, []);
  const s = origin === null ? null : video.shown;
  const src = s ? youtubeEmbedUrl(s.id, origin || "") : "";
  useEffect(() => {
    if (!s || !src) return;
    const seq = s.seq;
    const post = (msg: object) => { try { frame.current?.contentWindow?.postMessage(JSON.stringify(msg), YT_EMBED_ORIGIN); } catch { /* gone */ } };
    video.send = (func, args) => post({ event: "command", func, args, id: seq, channel: "widget" });
    let isReady = false;
    // YouTube's embed answers once the page says it listens (the handshake its own IFrame API does).
    const hello = setInterval(() => { if (!isReady) post({ event: "listening", id: seq, channel: "widget" }); }, 250);
    const onMsg = (e: MessageEvent) => {
      if (e.source !== frame.current?.contentWindow) return;
      const m = ytMessage(e.origin, e.data);
      if (!m) return;
      if ((m.event === "onReady" || m.event === "initialDelivery") && !isReady) { isReady = true; clearInterval(hello); video.ready(seq); }
      else if (m.event === "onError") video.failed(seq, ytErrorLine(m.info));
    };
    const win = hostWindow(anchor.current);
    win.addEventListener("message", onMsg);
    if (win !== window) window.addEventListener("message", onMsg);
    // Never part of Steam's controller navigation: a focus that lands in the player goes back to Steam at once.
    const guard = () => setTimeout(() => {
      if (win.document.activeElement === frame.current) { try { frame.current?.blur(); win.focus(); } catch { /* gone */ } }
    }, 0);
    win.addEventListener("blur", guard);
    const iv = setInterval(guard, 1000);
    return () => {
      clearInterval(hello); clearInterval(iv); win.removeEventListener("message", onMsg); window.removeEventListener("message", onMsg);
      win.removeEventListener("blur", guard);
    };
  }, [s?.seq, src]);
  if (!s || !src) return <div ref={anchor} style={{ display: "none" }} />;
  return (
    <div ref={anchor} className="hearth-music-mini" aria-hidden={false} tabIndex={-1}
      style={{ position: "fixed", left: 24, bottom: 24, width: MINI.width, height: MINI.height, zIndex: 7000, borderRadius: 8,
        overflow: "hidden", background: "#000", boxShadow: "0 6px 24px rgba(0,0,0,.6)", opacity: s.closing ? 0 : 1,
        transition: `opacity ${s.closing ? CROSSFADE : 600}ms ease` }}>
      <iframe key={s.seq} ref={frame} src={src} width={MINI.width} height={MINI.height} title="YouTube video player" tabIndex={-1}
        frameBorder={0} allow="autoplay; encrypted-media; picture-in-picture" referrerPolicy="strict-origin-when-cross-origin"
        style={{ border: 0, width: MINI.width, height: MINI.height, display: "block" }} />
    </div>
  );
}
