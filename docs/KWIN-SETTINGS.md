# Hearth Settings for Mantel on KWin (1.14)

When Desktop Mode runs Mantel on KWin (the session exports `HEARTH_WM=kwin`), Hearth Settings has its own pages for what
KWin can do. They are Hearth's pages (controller-friendly `Set*.qml` cards); KWin keeps the settings in its own files, so
each setting has one home. Under labwc every page shows its controls greyed with "Needs Mantel on KWin" (`KwinGate.qml`).

| Page (settings-pages.js id) | File | Where the settings live |
|---|---|---|
| Window effects (`kwin-effects`) | `SetKwinEffects.qml` | kwinrc `[Plugins] <effect>Enabled`, `[Effect-*]`, `[TabBox]`; kdeglobals `[KDE] AnimationDurationFactor` |
| Virtual desktops (`kwin-desktops`) | `SetKwinDesktops.qml` | KWin's VirtualDesktopManager (D-Bus) → kwinrc `[Desktops]`; `[Windows] RollOverDesktops`; slide / fadedesktop |
| Tiling (`kwin-tiling`) | `SetKwinTiling.qml` | kwinrc `[Tiling][<screen id>] tiles`, `[Tiling] padding`, `[Windows]` quick tiling and snapping |
| Screen edges (`kwin-edges`) | `SetKwinEdges.qml` | kwinrc `[ElectricBorders]`, `[TouchEdges]`, `[Effect-overview]` / `[Effect-windowview]` edge lists, `[EdgeBarrier]` |
| Window rules (`kwin-rules`) | `SetKwinRules.qml` | kwinrulesrc (`[General] count, rules` and one group per rule) |
| Drawing tablet & touchscreen (`kwin-tablet`) | `SetKwinTablet.qml`, `KwinCalibrate.qml` | KWin's InputDevice properties (D-Bus; KWin keeps them in kcminputrc); pen buttons in kcminputrc `[ButtonRebinds][TabletTool][<pen>]` |
| Full-screen games & X11 apps (`kwin-games`) | `SetKwinGames.qml` | kwinrc `[Compositing] AllowTearing`, `[Xwayland]`; kdeglobals `[KScreen] XwaylandClientsScale` |
| Mouse & touchpad (`mouse`, extended) | `SetMouse.qml` | Hearth's `pointer.json` (shared with labwc; the KWin-only keys have no labwc part), put on KWin's devices by `hearth-kwin-settings pointer-apply` |
| Accessibility (`accessibility`, extended) | `SetAccessibility.qml` | `accessibility.json` as before (hearth-access maps the magnifier and colour filters to KWin's zoom / magnifier / invert / colorblindnesscorrection); KWin's extras in kwinrc `[Effect-zoom]` and kaccessrc `[Keyboard]`, `[Mouse]`, `[Bell]` |

## How they write

`hearth-kwin-settings` (run by `KwinSettings.qml`) writes through `hearth_kconfig.py`: only the keys changed on the page,
merged under Hearth's lock and KDE's own QLockFile, atomically with `.bak`, every other line kept; a file that can't be
parsed is never written. Then it sends KDE's "config changed" signal and asks KWin to reload (`org.kde.KWin /KWin
reconfigure`). Opening a page writes nothing. Hearth's defaults go in the system kwinrc the session ships
(`$SHARE/desktop/kwin/xdg`, on `XDG_CONFIG_DIRS`), never in the person's file; a key set back to its default is removed so
the system value counts again.

## What KWin 6.3 (Debian 13) doesn't have

Greyscale (KWin 6.6's Monochrome), slow keys (6.6) and mouse keys (6.5) show as needing a newer KWin; inverted brightness
has no KWin effect; touchpad gestures are built into KWin (shown, not changed); KWin 6 has one set of desktops for all
screens.

## Hooks for other parts of Mantel on KWin

- The session: run `hearth-access apply kwin` and `hearth-kwin-settings input-watch` (puts the mouse and touchpad settings
  on every device, also ones plugged in later) when KWin's session starts; clear `$XDG_RUNTIME_DIR/hearth-kwin-inverted`
  at login (a fresh runtime dir does).
- The window switcher: a Mantel switcher installed as a KWin tabbox package (`kwin-wayland/tabbox/org.hearth.*`) is listed
  as "(Hearth)" under Window effects › Alt+Tab.
- Hearth's own actions on hot corners (the Hearth menu, Quick Access) need a KWin script that registers screen edges; the
  page offers KWin's own actions until then.
- The window list bridge can fill "Pick the app's window" in Window rules without a click (today: KWin's queryWindowInfo).

## Tests

`hosts/linux/test/test_kwin_settings.py` (INI merge, the helper, hearth-access on KWin). `desktop-test.yml` opens every
page under labwc and in a preview (`HEARTH_KWIN_FIXTURE` + `$XDG_RUNTIME_DIR/hearth-kwin-preview`,
`hosts/linux/test/fixtures/kwin-preview.json`), checks nothing was written, and posts the pictures as `SHOT kwin-*` notices
(job `kwin-pages-shots`).
