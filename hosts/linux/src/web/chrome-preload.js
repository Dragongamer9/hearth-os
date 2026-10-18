'use strict';
// The top bar's link to the browser: what it can ask for, and what it hears back.
const { contextBridge, ipcRenderer } = require('electron');
contextBridge.exposeInMainWorld('hb', {
  act: m => ipcRenderer.send('hb:act', m),
  suggest: text => ipcRenderer.invoke('hb:suggest', String(text || '')),
  on: fn => ipcRenderer.on('hb', (_e, m) => fn(m)),
});
