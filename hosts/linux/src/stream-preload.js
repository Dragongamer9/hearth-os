'use strict';
// Runs in the Xbox Cloud Gaming window. Its only job: holding View + Menu for ~1.5 s ends the stream.
// It reads the same Gamepad API the stream uses and doesn't touch the page.

const { ipcRenderer } = require('electron');

const VIEW = 8, MENU = 9, HOLD_MS = 1500;
let heldSince = 0;

function poll() {
  const pads = navigator.getGamepads ? [...navigator.getGamepads()].filter(Boolean) : [];
  const both = pads.some(p => p.buttons[VIEW] && p.buttons[VIEW].pressed && p.buttons[MENU] && p.buttons[MENU].pressed);
  if (!both) heldSince = 0;
  else if (!heldSince) heldSince = performance.now();
  else if (performance.now() - heldSince > HOLD_MS) { heldSince = 0; ipcRenderer.send('stream:exit'); }
  requestAnimationFrame(poll);
}
window.addEventListener('DOMContentLoaded', () => requestAnimationFrame(poll));
