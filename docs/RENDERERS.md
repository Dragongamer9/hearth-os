# Two ways to draw Hearth

Hearth on Linux is one program, `nimbus` (hosts/hearth-rs, on hearth-core). Everything it does (the library,
stores, Beacon, multi-seat, Game Mode, logs) is that program's. Only the drawing of its screens (ui/index.html)
can be done two ways:

| Engine | What draws | Where |
|---|---|---|
| **WebKit** | the system's WebKitGTK, through Tauri, in the `nimbus` process | the usual one: AMD and Intel graphics |
| **Chromium** | Hearth's Electron window app (`/usr/lib/hearth-electron`, hosts/electron) | Nvidia graphics |

Why: on Nvidia, WebKit's fast drawing path (DMA-BUF/GBM) fails on RTX 50 cards (WebKit bug 303811) and its
compatible path copies every frame through the processor, which makes Hearth sluggish. Chromium's graphics work
well there.

## How the Chromium engine works

`nimbus` decides at start (`hearth_core::renderer`). For Chromium it starts the Electron app in its place
(same arguments, `HEARTH_CORE_BIN` naming itself). The Electron app starts `nimbus --core-stdio`: the whole Hearth,
without a window, talking to the Electron app over its standard input and output, one JSON object a line
(hosts/hearth-rs/src/winhost.rs describes every message). The Electron app shows the page with the same
`nimbusHost` channel the Tauri host gives it, serves `nimbus://` (the page) and `nimbus-art://` (pictures: the core
says which file), and does what the core asks of its windows: show, hide, focus, the overlay, sign-in pages, Xbox
Cloud Gaming, file pickers. Window titles are the same as Tauri's (`Hearth`, `Hearth Overlay`), so Game Mode finds
them in gamescope the same way. Nothing is duplicated: a fix in the core is a fix for both.

The session and its watchdog don't change: they start `nimbus` as always; an exit code of 75 still means "start
again", 0 still means "leave".

## Choosing

- **Automatic** (the default): Chromium when there's an Nvidia graphics card (a display-class PCI device from
  vendor 0x10de) and the Electron app is installed; WebKit otherwise.
- Settings › Display › Drawing engine: Automatic, Chromium or WebKit (`~/.config/nimbus/renderer`). Hearth restarts
  to change it.
- `HEARTH_RENDERER=webkit|chromium|auto` wins over the setting (for testing).

If Chromium doesn't bring Hearth's screen up twice within 10 minutes (each start is recorded in
`~/.config/nimbus/renderer-chromium-starts` until the page says it's ready), Hearth draws with WebKit instead until it
works again or the setting is changed. Hearth's log says which engine it used (`renderer: …`), and `host.info`
reports it (`renderer`).
