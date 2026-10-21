'use strict';
// Hearth's page gets the same message channel as in the Tauri host (and WebView2 on Windows):
//   nimbusHost.postMessage(obj)                    → the main process → the Hearth core
//   nimbusHost.addEventListener('message', fn)     ← fn({data}) for each reply or event from the core
const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('nimbusHost', {
  platform: 'linux',
  postMessage: msg => ipcRenderer.send('nimbus:msg', msg),
  addEventListener: (type, fn) => {
    if (type !== 'message' || typeof fn !== 'function') return;
    ipcRenderer.on('nimbus:msg', (_e, data) => fn({ data }));
  },
});
