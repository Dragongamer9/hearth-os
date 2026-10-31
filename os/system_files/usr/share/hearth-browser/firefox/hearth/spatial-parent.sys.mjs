/* Hearth Browser: the parent side of the D-pad navigation actors (spatial-child.sys.mjs). It only keeps what the
 * pages report (where the focus went, problems), for Firefox's console and the CI test. */

export const SpatialLog = { last: null, errors: [] };

export class HearthSpatialParent extends JSWindowActorParent {
  receiveMessage(m) {
    // From a web page's process: plain text only, never acted on.
    if (m.name === "Hearth:Moved") SpatialLog.last = { dir: String(m.data && m.data.dir).slice(0, 8), to: String(m.data && m.data.to).slice(0, 80) };
    else if (m.name === "Hearth:Error") { SpatialLog.errors.push(String(m.data && m.data.error).slice(0, 300)); SpatialLog.errors.splice(0, SpatialLog.errors.length - 20); }
  }
}
export class HearthSpatialBackParent extends JSWindowActorParent {
  receiveMessage(m) { if (m.name === "Hearth:Back") SpatialLog.last = { back: true }; }
}
