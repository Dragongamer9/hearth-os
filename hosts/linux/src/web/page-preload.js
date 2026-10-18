'use strict';
// Hearth's own pages (hearth://start, history, …) get a way to ask the browser for their data. Ordinary
// websites get nothing from here.
const { contextBridge, ipcRenderer } = require('electron');
if (location.protocol === 'hearth:') {
  contextBridge.exposeInMainWorld('hearth', { get: m => ipcRenderer.invoke('hb:page', m) });
}
