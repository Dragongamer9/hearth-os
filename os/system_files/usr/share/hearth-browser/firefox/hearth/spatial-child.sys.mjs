/* Hearth Browser in Game Mode: moving between a page's links, buttons and fields with the D-pad (the controller
 * reaches Firefox as arrow keys, A as Enter and B as Esc, through Hearth Input on Linux and Ember's ControllerBridge on
 * Windows). A window actor (registered by hearth.sys.mjs, only in Game Mode), running in each page's own process.
 *
 *  - Up / Down / Left / Right: the focus goes to the nearest link, button or field in that direction on the screen
 *    (spatial navigation); spatial.css draws Hearth's focus ring on it. When there's nothing that way on (or just
 *    past) the screen, the key does what it always does: the page scrolls.
 *  - A (Enter): opens the link or presses the button (Firefox's own); check boxes, radio buttons, <summary> and
 *    ARIA buttons are clicked; a drop-down opens.
 *  - B (Esc): back to the previous page, when the page didn't use Esc itself (closing its own dialog or video).
 *  - In a text field Left / Right move in the text and Up / Down leave it; while Hearth's on-screen keyboard is open
 *    (hearth.osk.open) the keys are the keyboard's.
 */

const CANDIDATES = [
  "a[href]", "area[href]", "button", "input:not([type=hidden])", "select", "textarea", "summary", "iframe",
  "[role=button]", "[role=link]", "[role=checkbox]", "[role=radio]", "[role=tab]", "[role=menuitem]", "[role=option]", "[role=switch]",
  "[contenteditable=''], [contenteditable=true]", "[tabindex]:not([tabindex='-1'])",
].join(",");
const DIRS = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right" };
const TEXT_TYPES = new Set(["", "text", "search", "email", "url", "tel", "password", "number"]);

function editable(el) {
  if (!el) return false;
  if (el.isContentEditable) return true;
  const tag = el.localName;
  if (tag === "textarea") return true;
  if (tag === "input") return TEXT_TYPES.has((el.getAttribute("type") || "").toLowerCase());
  return false;
}

function usable(el, win) {
  if (el.disabled || el.inert || el.closest("[inert],[aria-hidden=true]")) return false;
  const r = el.getBoundingClientRect();
  if (r.width < 2 || r.height < 2) return false;
  const cs = win.getComputedStyle(el);
  if (!cs || cs.visibility === "hidden" || cs.display === "none" || parseFloat(cs.opacity) === 0) return false;
  return true;
}

/** The best next element from `from` (a rect) in `dir`, among those on the screen or within half a screen past it. */
export function pick(rects, from, dir, view) {
  const cx = from.left + from.width / 2, cy = from.top + from.height / 2;
  const reachX = view.width * 0.5, reachY = view.height * 0.5;
  let best = null, bestScore = Infinity;
  for (const c of rects) {
    const r = c.rect;
    // Within the screen (plus half a screen in the direction of travel).
    if (r.bottom < -(dir === "up" ? reachY : 0) || r.top > view.height + (dir === "down" ? reachY : 0)) continue;
    if (r.right < -(dir === "left" ? reachX : 0) || r.left > view.width + (dir === "right" ? reachX : 0)) continue;
    const x = r.left + r.width / 2, y = r.top + r.height / 2;
    let along, across;
    if (dir === "down") { if (r.top < from.bottom - 2 && y <= cy + 1) continue; along = Math.max(0, r.top - from.bottom); across = Math.max(0, Math.max(r.left - from.right, from.left - r.right)); }
    else if (dir === "up") { if (r.bottom > from.top + 2 && y >= cy - 1) continue; along = Math.max(0, from.top - r.bottom); across = Math.max(0, Math.max(r.left - from.right, from.left - r.right)); }
    else if (dir === "right") { if (r.left < from.right - 2 && x <= cx + 1) continue; along = Math.max(0, r.left - from.right); across = Math.max(0, Math.max(r.top - from.bottom, from.top - r.bottom)); }
    else { if (r.right > from.left + 2 && x >= cx - 1) continue; along = Math.max(0, from.left - r.right); across = Math.max(0, Math.max(r.top - from.bottom, from.top - r.bottom)); }
    // Straight ahead first; then near; sideways costs more than distance.
    const centre = dir === "up" || dir === "down" ? Math.abs(x - cx) : Math.abs(y - cy);
    const score = along + across * 3 + centre * 0.05 + (across > 0 ? 40 : 0);
    if (score < bestScore) { bestScore = score; best = c; }
  }
  return best;
}

export class HearthSpatialChild extends JSWindowActorChild {
  handleEvent(e) {
    if (e.type !== "keydown") return;
    try {
      if (Services.prefs.getBoolPref("hearth.osk.open", false)) return;
      if (e.ctrlKey || e.altKey || e.metaKey || e.isComposing) return;
      const win = this.contentWindow;
      const doc = this.document;
      if (!win || !doc || !doc.documentElement || doc.fullscreenElement) return;
      if (e.key in DIRS) return this.move(e, DIRS[e.key], win, doc);
      if (e.key === "Enter" && !e.shiftKey) return this.press(e, doc);
    } catch (err) {
      try { Cu.reportError("Hearth Browser (spatial): " + err); this.sendAsyncMessage("Hearth:Error", { error: String(err) }); } catch (_) {}
    }
  }

  /** B: back, when the page itself didn't take Esc (it runs after the page's own handlers). */
  backIfUnused(e) {
    if (e.defaultPrevented || e.ctrlKey || e.altKey || e.metaKey || e.shiftKey) return;
    if (Services.prefs.getBoolPref("hearth.osk.open", false)) return;
    const doc = this.document, win = this.contentWindow;
    if (!doc || doc.fullscreenElement || editable(doc.activeElement)) return;
    if (doc.querySelector("dialog[open]")) return;
    if (win.history.length > 1) { e.preventDefault(); win.history.back(); try { this.sendAsyncMessage("Hearth:Back", {}); } catch (_) {} }
  }

  move(e, dir, win, doc) {
    const active = doc.activeElement && doc.activeElement !== doc.body && doc.activeElement !== doc.documentElement ? doc.activeElement : null;
    if (active && editable(active)) {
      if (dir === "left" || dir === "right") return;   // moving in the text
      if (active.localName === "textarea" || active.isContentEditable) {
        // Up/Down move between lines until the first/last line: leave only from the edges.
        const v = active.value;
        if (typeof v === "string") {
          const at = active.selectionStart;
          if (dir === "up" && v.lastIndexOf("\n", at - 1) >= 0) return;
          if (dir === "down" && v.indexOf("\n", at) >= 0) return;
        } else return;
      }
    }
    const view = { width: win.innerWidth, height: win.innerHeight };
    const all = [];
    for (const el of doc.querySelectorAll(CANDIDATES)) {
      if (el === active || !usable(el, win)) continue;
      all.push({ el, rect: el.getBoundingClientRect() });
    }
    // Nothing focused yet: start from an edge of the screen, so the first press picks what's nearest it.
    const from = active ? active.getBoundingClientRect()
      : dir === "down" ? { left: 0, right: view.width, top: -1, bottom: -1, width: view.width, height: 0 }
      : dir === "up" ? { left: 0, right: view.width, top: view.height + 1, bottom: view.height + 1, width: view.width, height: 0 }
      : dir === "right" ? { left: -1, right: -1, top: 0, bottom: view.height, width: 0, height: view.height }
      : { left: view.width + 1, right: view.width + 1, top: 0, bottom: view.height, width: 0, height: view.height };
    const next = pick(all, from, dir, view);
    if (!next) return;   // nothing that way: the page scrolls as usual
    e.preventDefault();
    e.stopPropagation();
    try { next.el.focus({ focusVisible: true, preventScroll: true }); } catch (_) { next.el.focus(); }
    next.el.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
    try { this.sendAsyncMessage("Hearth:Moved", { dir, to: next.el.id || next.el.localName }); } catch (_) {}
  }

  press(e, doc) {
    const el = doc.activeElement;
    if (!el || el === doc.body) return;
    const tag = el.localName, type = (el.getAttribute("type") || "").toLowerCase(), role = el.getAttribute("role") || "";
    if (tag === "input" && (type === "checkbox" || type === "radio")) { e.preventDefault(); el.click(); return; }
    if (tag === "summary" || (!["a", "button", "input", "textarea", "select"].includes(tag) && /^(button|link|checkbox|radio|tab|menuitem|option|switch)$/.test(role))) {
      e.preventDefault(); el.click(); return;
    }
    if (tag === "select") { e.preventDefault(); try { el.showPicker(); } catch (_) { el.click(); } }
  }
}

/** B (Esc) runs in the system group after the page's own handlers; registered as a second actor's event. */
export class HearthSpatialBackChild extends JSWindowActorChild {
  handleEvent(e) {
    if (e.type !== "keydown" || e.key !== "Escape") return;
    try { HearthSpatialChild.prototype.backIfUnused.call(this, e); } catch (err) { try { Cu.reportError("Hearth Browser (back): " + err); } catch (_) {} }
  }
}
