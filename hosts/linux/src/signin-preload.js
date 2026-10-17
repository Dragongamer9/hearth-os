'use strict';
// Runs in a store's sign-in window: B on a controller (or Esc) cancels. The page itself is the store's own.
const { ipcRenderer } = require('electron');
let was = false;
function poll() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const b = pads.some(p => p.buttons[1] && p.buttons[1].pressed);
  if (b && !was) ipcRenderer.send('signin:cancel');
  was = b;
  requestAnimationFrame(poll);
}
window.addEventListener('DOMContentLoaded', () => requestAnimationFrame(poll));
