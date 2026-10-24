// Runs in Game Mode › Store's store window (a store's own page: GOG, Epic, Xbox). Links that would open a new
// window open in this one: the store window is one page at a time, like the sign-in window, and a second window
// would land behind Game Mode. Leaving: B with a controller (signin-pad.js), or closing the window on the desktop.
(() => {
  if (window.__hearthStoreWeb) return;
  window.__hearthStoreWeb = true;
  document.addEventListener('click', e => {
    const a = e.target && e.target.closest ? e.target.closest('a[target]') : null;
    if (a && a.target !== '_self' && a.href && a.href.startsWith('https://')) a.target = '_self';
  }, true);
})();
