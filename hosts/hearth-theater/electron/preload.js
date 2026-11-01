'use strict';
// Theater's home screen (ui/index.html) only: the calls it had from Tauri (window.__TAURI__.core.invoke and
// .event.listen), now answered by Theater's core through the window app; and theaterHost, for the viewer's
// controller (ui/viewer-pad.js). Sites in the viewer get no preload at all.
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('__TAURI__', {
  core: {
    invoke: async (cmd, args) => {
      const r = await ipcRenderer.invoke('theater:call', String(cmd), args || {});
      if (r && r.ok) return r.value;
      throw (r && r.error) || 'Theater ran into a problem';
    },
  },
  event: {
    listen: (name, fn) => {
      const h = (_e, n, payload) => { if (n === name) fn({ event: name, payload }); };
      ipcRenderer.on('theater:event', h);
      return Promise.resolve(() => ipcRenderer.removeListener('theater:event', h));
    },
  },
});

contextBridge.exposeInMainWorld('theaterHost', {
  /** The viewer: close, back, forward, media {do: toggle|seek|fill, s}, hints, toast {text}, osk {on, top, h}, type {text}. */
  viewer: (op, args) => ipcRenderer.send('theater:viewer', String(op), args || {}),
  /** The pointer: {op: move, dx, dy} | {op: button, button, down} | {op: wheel, dx, dy} | {op: key, key, n}. */
  ptr: m => ipcRenderer.send('theater:ptr', m),
  /** fn({type: 'open' | 'closed'}) when the viewer opens or closes. */
  on: fn => ipcRenderer.on('theater:viewer-event', (_e, m) => fn(m)),
});
