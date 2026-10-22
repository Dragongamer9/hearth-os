// Runs first in a store's sign-in window: the page doesn't see the controller. Epic's and Microsoft's sign-in pages
// have controller support of their own (the Gamepad API), so a button press moved and clicked things on the page
// while Hearth's pointer (or, in Desktop Mode, the desktop's) did too: random clicks. GOG's page has none, which is
// why it was fine. Hearth's own controller script reads the real controllers through window.__hearthGamepads.
(() => {
  if (window.__hearthGamepads || !navigator.getGamepads) return;
  const real = navigator.getGamepads.bind(navigator);
  Object.defineProperty(window, '__hearthGamepads', { value: real, enumerable: false });
  const none = function getGamepads() { return [null, null, null, null]; };
  try { Object.defineProperty(Navigator.prototype, 'getGamepads', { value: none, configurable: true, writable: true }); } catch {}
  try { Object.defineProperty(navigator, 'getGamepads', { value: none, configurable: true, writable: true }); } catch {}
  for (const t of ['gamepadconnected', 'gamepaddisconnected'])
    window.addEventListener(t, e => e.stopImmediatePropagation(), true);
})();
