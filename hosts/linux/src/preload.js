'use strict';
// Gives the shared UI the same message channel WebView2 gives it on Windows:
//   nimbusHost.postMessage(obj)                    → main process
//   nimbusHost.addEventListener('message', fn)     ← fn({data}) for each message from the main process
// The UI uses window.chrome.webview on Windows and window.nimbusHost here; everything else is identical.

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('nimbusHost', {
  platform: 'linux',
  postMessage: msg => ipcRenderer.send('nimbus:msg', msg),
  addEventListener: (type, fn) => {
    if (type !== 'message' || typeof fn !== 'function') return;
    ipcRenderer.on('nimbus:msg', (_e, data) => fn({ data }));
  },
});
