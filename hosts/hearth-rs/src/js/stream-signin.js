// Runs in the Xbox Cloud Gaming window, before signin-pad.js. Microsoft's sign-in pages open inside this window
// (not in Hearth's own sign-in window), and Steam or the controller driver turned the controller's buttons into
// clicks and keys there, which clicked links while you typed. On those pages only, Hearth's controller pointer
// and on-screen keyboard take over, the same as in the store sign-in window (it drops those stray clicks and
// keys). The stream itself is never touched: there the controller goes to the game.
window.__hearthPadWanted = /(^|\.)(login\.live\.com|login\.microsoftonline\.com|account\.live\.com|signup\.live\.com|account\.microsoft\.com|login\.microsoft\.com|sisu\.xboxlive\.com)$/i.test(location.hostname);
// B on a sign-in page: back to where it came from (Xbox Cloud Gaming), not "cancel sign-in".
window.__hearthPadCancel = () => { if (history.length > 1) history.back(); else location.href = 'https://www.xbox.com/play'; };
