// Hearth's icon set (hosts/linux/desktop/shell/icons, the light tone), for Mantel on Windows: the same shapes as
// Mantel on Hearth OS. Each is drawn in currentColor. Generated from the Linux icons; keep them in step.
window.HEARTH_ICONS = (() => {
  const p = {
    "back": "<path d=\"M15 5l-7 7 7 7\"/>",
    "batt-charging": "<rect x=\"3\" y=\"7.5\" width=\"16\" height=\"9\" rx=\"2\"/><path d=\"M21 10.5v3M12 9l-2.5 3.2h3L10 15\"/>",
    "batt": "<rect x=\"3\" y=\"7.5\" width=\"16\" height=\"9\" rx=\"2\"/><path d=\"M21 10.5v3\"/><rect x=\"5\" y=\"9.5\" width=\"9\" height=\"5\" rx=\"1\" fill=\"currentColor\" stroke=\"none\"/>",
    "batt-low": "<rect x=\"3\" y=\"7.5\" width=\"16\" height=\"9\" rx=\"2\"/><path d=\"M21 10.5v3\"/><rect x=\"5\" y=\"9.5\" width=\"3\" height=\"5\" rx=\"1\" fill=\"currentColor\" stroke=\"none\"/>",
    "bt": "<path d=\"M7 7l10 10-5 4V3l5 4L7 17\"/>",
    "cam": "<path d=\"M4 8h3l1.5-2h7L17 8h3v11H4z\"/><circle cx=\"12\" cy=\"13\" r=\"3.2\"/>",
    "check": "<path d=\"M5 12.5l4.5 4.5L19 7.5\"/>",
    "close": "<rect x=\"4\" y=\"5\" width=\"16\" height=\"14\" rx=\"2.5\"/><path d=\"M9.5 9.5l5 5M14.5 9.5l-5 5\"/>",
    "display": "<rect x=\"3\" y=\"4\" width=\"18\" height=\"12\" rx=\"2\"/><path d=\"M8 20h8M12 16v4\"/>",
    "dot": "<circle cx=\"12\" cy=\"12\" r=\"3\"/>",
    "gear": "<circle cx=\"12\" cy=\"12\" r=\"3\"/><path d=\"M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z\"/>",
    "globe": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M3 12h18M12 3c2.6 2.7 3.8 5.7 3.8 9s-1.2 6.3-3.8 9c-2.6-2.7-3.8-5.7-3.8-9S9.4 5.7 12 3z\"/>",
    "grid": "<rect x=\"4\" y=\"4\" width=\"6\" height=\"6\" rx=\"1.5\"/><rect x=\"14\" y=\"4\" width=\"6\" height=\"6\" rx=\"1.5\"/><rect x=\"4\" y=\"14\" width=\"6\" height=\"6\" rx=\"1.5\"/><rect x=\"14\" y=\"14\" width=\"6\" height=\"6\" rx=\"1.5\"/>",
    "help": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M9.6 9.3a2.5 2.5 0 0 1 4.8.8c0 1.7-2.4 2-2.4 3.6M12 17h.01\"/>",
    "info": "<circle cx=\"12\" cy=\"12\" r=\"9\"/><path d=\"M12 11v6M12 7.5v.5\"/>",
    "kbd": "<rect x=\"2.5\" y=\"6\" width=\"19\" height=\"12\" rx=\"2.5\"/><path d=\"M6 10h.01M9.5 10h.01M13 10h.01M16.5 10h.01M7 14h10\"/>",
    "lock": "<rect x=\"5\" y=\"11\" width=\"14\" height=\"10\" rx=\"2\"/><path d=\"M8 11V8a4 4 0 0 1 8 0v3\"/>",
    "logout": "<path d=\"M15 4h3a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-3\"/><path d=\"M10 8l-4 4 4 4M6 12h10\"/>",
    "menu": "<path d=\"M5 7h14M5 12h14M5 17h14\"/>",
    "mic": "<rect x=\"9\" y=\"3\" width=\"6\" height=\"11\" rx=\"3\"/><path d=\"M5 11a7 7 0 0 0 14 0M12 18v3\"/>",
    "moon": "<path d=\"M19 14.5A7.5 7.5 0 0 1 9.5 5a7.5 7.5 0 1 0 9.5 9.5z\"/>",
    "motion": "<path d=\"M3 12h4l2-6 4 12 2-6h6\"/>",
    "mouse": "<rect x=\"7\" y=\"3.5\" width=\"10\" height=\"17\" rx=\"5\"/><path d=\"M12 7v3\"/>",
    "offline": "<path d=\"M3 9.5a13 13 0 0 1 4-2.6M10.5 6.1A13 13 0 0 1 21 9.5M6 12.8a8.5 8.5 0 0 1 3.5-2M18 12.8a8.5 8.5 0 0 0-2.2-1.6M9 16a4 4 0 0 1 6 0M12 19h.01M4 4l16 16\"/>",
    "pad": "<path d=\"M7 8h10a4 4 0 0 1 4 4.5l-.6 3.4a2.2 2.2 0 0 1-3.8 1L15 15H9l-1.6 1.9a2.2 2.2 0 0 1-3.8-1L3 12.5A4 4 0 0 1 7 8z\"/><path d=\"M8 11v3M6.5 12.5h3M15.5 12h.01M17.5 13.5h.01\"/>",
    "palette": "<path d=\"M12 3a9 9 0 1 0 0 18c1.1 0 1.7-.9 1.4-1.9-.3-1 .4-2.1 1.5-2.1H17a4 4 0 0 0 4-4c0-5.5-4-10-9-10z\"/><circle cx=\"7.5\" cy=\"11\" r=\"1.2\"/><circle cx=\"10\" cy=\"7\" r=\"1.2\"/><circle cx=\"15\" cy=\"7.5\" r=\"1.2\"/>",
    "power": "<path d=\"M12 3v8\"/><path d=\"M6.3 6.5a8 8 0 1 0 11.4 0\"/>",
    "restart": "<path d=\"M4 12a8 8 0 1 0 2.3-5.7\"/><path d=\"M4 4v5h5\"/>",
    "sleep": "<path d=\"M18 15a7 7 0 0 1-9-9 7 7 0 1 0 9 9z\"/><path d=\"M15 4h4l-4 4h4\"/>",
    "sun": "<circle cx=\"12\" cy=\"12\" r=\"3.5\"/><path d=\"M12 3v2M12 19v2M3 12h2M19 12h2M5.6 5.6l1.4 1.4M17 17l1.4 1.4M5.6 18.4L7 17M17 7l1.4-1.4\"/>",
    "update": "<path d=\"M20 12a8 8 0 1 1-2.3-5.7\"/><path d=\"M20 4v5h-5\"/><path d=\"M12 8v5l3 2\"/>",
    "user": "<circle cx=\"12\" cy=\"8\" r=\"4\"/><path d=\"M4 21c1.2-4 4.3-6 8-6s6.8 2 8 6\"/>",
    "vol": "<path d=\"M4 9.5h3.5L12 5.5v13l-4.5-4H4z\"/><path d=\"M15.5 9a4 4 0 0 1 0 6M18 6.5a7.5 7.5 0 0 1 0 11\"/>",
    "vol-low": "<path d=\"M4 9.5h3.5L12 5.5v13l-4.5-4H4z\"/><path d=\"M15.5 9a4 4 0 0 1 0 6\"/>",
    "vol-mute": "<path d=\"M4 9.5h3.5L12 5.5v13l-4.5-4H4z\"/><path d=\"M16 9.5l5 5M21 9.5l-5 5\"/>",
    "wifi": "<path d=\"M3 9.5a13 13 0 0 1 18 0M6 12.8a8.5 8.5 0 0 1 12 0M9 16a4 4 0 0 1 6 0M12 19h.01\"/>",
    "wired": "<rect x=\"9\" y=\"3.5\" width=\"6\" height=\"5\" rx=\"1\"/><path d=\"M12 8.5v4M5 12.5h14M5 12.5v3M19 12.5v3M12 12.5v3\"/><rect x=\"3\" y=\"15.5\" width=\"4\" height=\"4\" rx=\"1\"/><rect x=\"10\" y=\"15.5\" width=\"4\" height=\"4\" rx=\"1\"/><rect x=\"17\" y=\"15.5\" width=\"4\" height=\"4\" rx=\"1\"/>",
  };
  const o = {};
  for (const k in p) o[k] = '<svg viewBox="0 0 24 24">' + p[k] + '</svg>';
  return o;
})();
