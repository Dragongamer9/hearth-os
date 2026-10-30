# Nimbus bridge contract

The UI (`ui/index.html`) is shared by every platform. It talks to its host only through the messages
below, so a host is "done" when it answers all of them. If you add a method, add it here, to both hosts,
and to the browser-preview `MOCK` in the UI.

| Host | Channel the UI sees | Implementation |
|---|---|---|
| Windows | `window.chrome.webview` (WebView2) | `hosts/windows/Nimbus.Host/SystemBridge.cs`, `ShellForm.cs` |
| Linux | `window.nimbusHost` (Electron preload, same API) | `hosts/linux/src/bridge.js`, `main.js` |
| Android | `window.nimbusHost` with `platform: 'android'` (a shim over the WebView's `HearthAndroid` interface); a subset, see [ANDROID.md](ANDROID.md) | `hosts/android/app/src/main/java/dev/hearthos/android/Bridge.kt`, `MainActivity.kt` |
| Browser preview | none, the UI uses its built-in `MOCK` | `ui/index.html` |
| Linux, Qt renderer (Experimental) | no web page: native QML screens make the same calls over the core channel | `hosts/hearth-qml/qml/App.qml` ([RENDERERS.md](RENDERERS.md)) |

Both channels have the same shape: `postMessage(obj)` and `addEventListener('message', e => e.data)`.

## Messages

**UI → host**

| Message | Meaning |
|---|---|
| `{type:'ready'}` | The page (re)loaded. |
| `{type:'call', id, method, args}` | Request; answered by a `result` with the same `id`. |
| `{type:'log', text}` | Line for the host's `bridge.log`. |
| `{type:'power', action}` | `sleep` · `restart` · `shutdown` · `signout` · `desktop` (Windows: open File Explorer; Linux: leave the Nimbus session). |
| `{type:'resume'}` | Quick Access closed while a game runs: give the screen back to the game. Linux only; Windows ignores it. |
| `{type:'setting', key, value}` | Settings with no host support yet (frame limit, TDP). Ignored. |

**Host → UI**

| Message | Meaning |
|---|---|
| `{type:'result', id, ok, value \| error}` | Answer to a `call`. |
| `{type:'event', name, data}` | Push (list below). |
| `{type:'toast', text}` | Show a short notice. |

## Methods

Values are camelCase JSON. "—" means the method returns `null`.

| Method | Args | Returns |
|---|---|---|
| `host.info` | | `{platform:'windows'\|'linux', name, host, version, session, user}` |
| `sys.status` | | `{wifi:{available,on,ssid,bars}, bt:{available,on}, airplane, battery:{present,pct,charging}, audio:{volume,muted}, display:{supported,brightness}, powerMode}` (+ `wired:{connected,name}\|null` on Windows) |
| `radios.set` | `{kind:'wifi'\|'bt', on}` | status |
| `airplane.set` | `{on}` | status |
| `wifi.scan` | | `[{ssid, bars, secure, saved, connected}]` |
| `wifi.connect` | `{ssid, password?}` | `{ok, error?}` |
| `wifi.disconnect` · `wifi.forget` | `{ssid}` for forget | — |
| `bt.paired` | | `[{id, name, kind, connected, paired}]` |
| `bt.scanStart` · `bt.scanStop` | `{seconds?}` | — (devices arrive as `bt.found`, end as `bt.scanDone`) |
| `bt.pair` · `bt.unpair` | `{id}` | `{ok, error?}` |
| `bt.confirmPin` | `{accept}` | — |
| `audio.get` | | `{volume, muted, devices:[{id,name,isDefault}]}` |
| `audio.setVolume` · `audio.setMute` · `audio.setDefault` | `{v}` · `{m}` · `{id}` | — · — · audio |
| `display.get` | | `{supported, brightness, resolution, scale}` |
| `display.setBrightness` | `{v}` | — |
| `display.get` (mode fields, optional) | | `modesSupported, output, resolutions:[{w,h,rates}], current:{w,h,rate}, refresh, scalePct, scales, scaleIsInterface, appliesOnRestart` (Linux; hosts without them show resolution and scale read-only) |
| `display.setMode` · `display.setScale` | `{w,h,rate}` · `{pct}` | display (mode fields) |
| `power.get` · `power.setMode` | `{mode:'efficiency'\|'balanced'\|'performance'}` | `{present, pct, charging, mode}` (+ `secondsLeft` on Windows, -1 when unknown) |
| `library.get` | `{rescan?}` | `{games:[Game], sources, counts, runningId, steamBackground}` |
| `library.setFav` · `library.setHidden` | `{id, on}` | — |
| `library.setSources` | `{steam, epic, gog, xbox}` | library |
| `library.addManual` | | Game, or `null` if cancelled (the host shows a file picker) |
| `library.remove` | `{id}` | — |
| `library.downloads` | | `{active:[Game], changed}` |
| `library.setSteamBackground` | `{on}` | — |
| `game.launch` | `{id}` | `{tracked}` (false: started but can't be watched) |
| `game.install` | `{id}` | — |
| `game.close` | | `{ok}` |
| `update.status` · `update.check` · `update.download` | | `{current, state, latest, progress, error, canInstallHere, installFolder, autoCheck, lastChecked}` |
| `update.install` | | — (the host restarts) |
| `update.setAuto` | `{on}` | update status |
| `pad.status` · `pad.setMouseMode` | `{mode}` | `{connected, mouseMode, mouseOn}` (Windows only; Linux reports none) |
| `game.uninstall` · `game.cancelInstall` | `{id}` | library · `{ok}` (Windows: games Nimbus installed; Linux: not offered) |
| `accounts.status` | | `{installRoot, accounts:[{store, signedIn, name, avatar, owned, installed, fetched, busy, error, note?}]}` |
| `accounts.signIn` · `accounts.signOut` · `accounts.refresh` | `{store}` | `{ok}` · — · — (Linux: opens Steam, Heroic, or the Xbox cloud window) |
| `accounts.pickFolder` | | `{installRoot}` (Windows only) |
| `profile.get` · `profile.set` | `{name?, status?, avatar?, accent?, background?}` for set | `{name, status, avatar, accent, background, customAvatar, customBackground, storeAvatars}` |
| `profile.pickImage` | `{kind:'avatar'\|'background'}` | profile, or `null` if cancelled |
| `shell.status` | | `{running, registered, value, exe}` (+ `gamescope` on Linux) |
| `shell.register` · `shell.unregister` · `shell.returnToDesktop` | | |
| `login.get` (Linux) | | `{canAdmin, autologin, startMode, hostname, user}`; `canAdmin`: Hearth, the OS (root options available) |
| `login.setAutologin` · `login.setStartMode` · `login.setHostname` | `{on}` · `{mode: game\|desktop}` · `{name}` | login |
| `login.password` | `{old, new}` | `{ok}`; checks the current password first |
| `audio.input` · `audio.setInputVolume` (Windows, Mantel's Sound page) | — · `{v}` | `{present, volume, devices:[{id,name,isDefault}]}` · — (`audio.setDefault` also takes a microphone's id) |
| `desktop.get` · `desktop.set` (Windows, Mantel) | — · `{key: nightLight\|reduceMotion\|pointerSpeed\|lockOnWake\|powerPress, value}` | `{nightLight, nightAvailable, reduceMotion, pointerSpeed, lockOnWake, powerPress}` (`lockOnWake`/`powerPress` null when Windows can't tell) |
| `browser.status` (Windows, Mantel) | | `{installed, default, version}` for Aloha Browser |
| `claude.status` · `claude.act` (Hearth OS) | — · `{action: install\|update\|check\|remove\|open\|install-code\|remove-code}` | hearth-claude's status (`{installed, ours, apt, version, latest, updateAvailable, pending, running, official, sandbox, code: {installed, version}}`) with the running job's `state`, `progress`, `line`, `error`; jobs push `claude` |
| `shell.pickWallpaper` (Windows) | | shell status, or `null` if cancelled; `shell.status` has `pictures` (file names) and `shell.setWallpaper` takes `custom:<file>` |
| `display.renderer` (Linux) | `{set?: auto\|webkit\|chromium\|qt, once?: web\|webkit\|chromium\|qt}` | `{pick, running, electron, qt, nvidia, qtCrashes, env}`; `set` from the next start, `once` for the next start only ([RENDERERS.md](RENDERERS.md)) |
| `system.update.status` · `.check` · `.install` (Hearth OS) | | `{canAdmin, state: idle\|checking\|updating, line, error, status: {booted, staged, available, rollback, rollbackQueued}}`; check and install return at once and push `systemUpdate` |
| `system.update.rollback` (Hearth OS images) | | the same; makes the previous version (`status.rollback`, bootc's rollback deployment) the one that starts next, and again before restarting undoes it (`hearth-admin rollback`). Refused while an update waits for a restart |
| `system.recovery.nextStart` (Hearth OS) | | `{ok: true}`; Hearth Recovery's menu opens once at the next start (`hearth-admin recovery-next-start`, which runs `hearth-recovery --next-start`). Settings › Updates & Recovery (Dragonfire look) restarts after it. |

**Game** = `{id, title, source, fav, hidden, minutes, lastPlayed, added, sizeBytes, art:{cover,hero,logo,wide}, path, canRemove, installed, state, cloud, progress, bytesDone, bytesTotal, bytesPerSec?}`

- `source`: `steam` · `epic` · `gog` · `xbox` · `manual`.
- `state`: `installed` · `notInstalled` · `downloading` · `queued` · `paused` · `updating` · `cloud`.
- `cloud: true` means the game streams (Xbox Cloud Gaming on Linux). The UI shows **Stream** instead of Install.
- `art` values are URLs the page can load: store CDNs, or the host's art scheme for local files
  (`https://nimbus-art.local/{id}/{kind}` on Windows, `nimbus-art://art/{id}/{kind}` on Linux).

## Events

`status` · `game.started {id}` · `game.exited Game` · `game.timeout {id}` · `update` (update status) ·
`bt.found` (device) · `bt.scanDone` · `bt.pin {pin, confirm}` · `pad {b}` (a controller button the host read; `guide` toggles Quick Access) ·
`padMouse {on}` · `padHost {connected}` (when > 0 the UI stops reading controllers itself) · `accounts.changed` ·
`systemUpdate` (system update status, Linux) · `desktopFailed {text}` (Desktop Mode couldn't start, Linux) · `open {view}` (show a page, Linux).
