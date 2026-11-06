# Share this device's AI

Each of your Hearth devices runs its own AI jobs, whole: the Nvidia PC does big chat (Ollama, `qwen3:30b-a3b`) and
pictures (ComfyUI), the ROG Ally does voices (Kokoro) or a small chat model when docked, a laptop does speech to text
(whisper). **Share this device's AI** lets your other devices use them through Tailscale. Your **Home server** lists
every device's AI in one place, and the **Hearth AI Command Center** reads that list to pick the best device for each
job: the Nvidia PC first, the others when it's busy, playing or off.

It's **off by default** on every device, and nothing new starts with the PC.

## What you see

Settings › Home server › **AI devices** (`SetHomeAIDevices.qml`), also opened from Home server › AI and from
Settings › Hearth Link (on a device that isn't a server):

- **Share this device's AI** (off). Its line says what's happening: shared through Tailscale as `nvidia-pc`, paused
  while you play, paused on battery, waiting for Tailscale or for Hearth's firewall rule, or no AI found here.
- **Who can see it**: your devices through Tailscale, and this device. Never your home network or the internet.
- **What to share**: one switch per AI service found on this device (chat, voices, pictures, speech to text), with
  its models or voices. A service that only listens on this device says so (other devices can't use it yet).
- **When to share**: **Share only when plugged in** (only shown on a device with a battery; on by default) and **Not
  while I'm playing** (on by default). **Also on my home network** (off). **Ask for a share code** (off; set a code
  of 8 to 64 characters, only its hash is kept; use the same code on every device).
- **Your AI devices** (on a Home server, or wherever "Let the Command Center find my devices here" is on):
  - **Let the Command Center find my devices here** (off) and the **Command Center address**
    `http://<this server's Tailscale name>:8189` with **Copy**: the one address the Command Center needs.
  - Every device with its state (online or offline, playing, busy, battery and plugged in, its graphics card) and its
    shared chat models, voices and picture engine, numbered in the order the Command Center tries them.
  - Select a device, then **Move up** / **Move down** (works with a controller). **Back to the usual order**: a device
    with an Nvidia card first, then the others by graphics memory.

## The protocol (version 1)

Every sharing device answers on **port 8189**. Read only, JSON, no secrets, GET only (anything else is 405, any other
path 404). It answers only this device and Tailscale addresses (`100.64.0.0/10`, `fd7a:115c:a1e0::/48`) unless "Also
on my home network" is on.

### `GET http://<tailscale-name>:8189/v1/hearth/ai`

What this device has:

```json
{
  "protocol": 1,
  "device": "nvidia-pc",
  "name": "Nvidia-PC",
  "address": "nvidia-pc.tail4f2a.ts.net",
  "os": "hearth",
  "version": "1.4",
  "gaming": false,
  "busy": false,
  "available": true,
  "reason": "",
  "pluggedIn": true,
  "battery": {"percent": 64, "charging": false},
  "gpu": {"vendor": "nvidia"},
  "services": [
    {"kind": "chat", "engine": "ollama", "url": "http://nvidia-pc.tail4f2a.ts.net:11434",
     "openai": "http://nvidia-pc.tail4f2a.ts.net:11434/v1", "healthy": true, "reachable": true, "busy": false,
     "models": ["qwen3:30b-a3b", "qwen3:14b"], "loaded": ["qwen3:30b-a3b"]},
    {"kind": "voice", "engine": "kokoro-fastapi", "url": "http://nvidia-pc.tail4f2a.ts.net:8880",
     "openai": "http://nvidia-pc.tail4f2a.ts.net:8880/v1", "healthy": true, "reachable": true, "busy": false,
     "models": ["kokoro"], "voices": ["af_bella", "am_adam"]},
    {"kind": "image", "engine": "comfyui", "url": "http://nvidia-pc.tail4f2a.ts.net:8188", "healthy": true,
     "reachable": true, "busy": false, "models": ["sdxl.safetensors"]},
    {"kind": "stt", "engine": "whisper", "url": "http://nvidia-pc.tail4f2a.ts.net:8000",
     "openai": "http://nvidia-pc.tail4f2a.ts.net:8000/v1", "healthy": true, "reachable": true, "busy": false,
     "models": ["Systran/faster-whisper-small"]}
  ],
  "updated": 1762354800000
}
```

| Field | Meaning |
|---|---|
| `protocol` | `1`. A client reads the fields it knows and ignores new ones; a breaking change gets a new number. |
| `device` | The Tailscale short name (small letters, digits, `-`): the device's id in lists and in the priority order. |
| `name`, `address` | The device's name as people see it, and its full Tailscale (MagicDNS) name. |
| `os`, `version` | Always `"hearth"`; Hearth OS's version (`/usr/share/nimbus/version`). |
| `gaming` | `true` while Game Mode or a game runs (gamescope, or Hearth's `hearth-playing` file), whatever the settings: route elsewhere when you can. |
| `busy` | A shared service is working for another device now (connections from other devices to Ollama, Kokoro or whisper; ComfyUI's queue). |
| `available` | Use this device now: sharing is on, not paused, and at least one service is healthy. |
| `reason` | Why not: `off` (not sharing), `playing` (Not while I'm playing), `battery` (Share only when plugged in), `nothing` (no service found); the Home server's list adds `offline` and `not-sharing`. |
| `pluggedIn`, `battery` | `battery` only on a device with one (`percent`, `charging`); controllers' batteries don't count. A desktop is always plugged in. |
| `gpu` | The strongest card's maker (`nvidia`, `amd`, `intel`), and `vramMB` when the driver says (AMD). |
| `services[]` | Only the services you share. `kind`: `chat`, `voice`, `image`, `stt`. `engine`: `ollama`, `kokoro-fastapi`, `comfyui`, `whisper`. `url`: the service's own address on Tailscale (clients talk to it directly; Hearth doesn't proxy). `openai`: its OpenAI-compatible base where it has one. `healthy`: it answered and other devices can reach it. `reachable`: false when it listens on that device only. `models`, `voices`, `loaded` (Ollama's models in memory). |

### `GET http://<home-server>:8189/v1/hearth/ai/devices`

On the device where **Let the Command Center find my devices here** is on (your Home server). Every Hearth device,
best first:

```json
{
  "protocol": 1,
  "server": "nvidia-pc",
  "updated": 1762354800000,
  "priority": ["nvidia-pc", "rog-ally", "laptop", "living-room"],
  "devices": [
    {"rank": 1, "device": "nvidia-pc", "online": true, "via": "this device", "lastSeen": 1762354800000, "...": "the document above"},
    {"rank": 2, "device": "rog-ally", "online": true, "via": "tailscale", "gaming": true, "available": false, "reason": "playing", "...": "..."},
    {"rank": 3, "device": "laptop", "online": false, "via": "seen before", "reason": "offline", "services": ["what it shared last time"]},
    {"rank": 4, "device": "living-room", "online": false, "via": "hearth link", "reason": "offline", "services": []}
  ]
}
```

Each entry is the device's own document (checked: only the fields above, URLs only `http://host:port`) plus `rank`,
`online`, `via` (`this device`, `tailscale`, `seen before`: offline now, with what it shared last; `hearth link`: a
paired device that isn't sharing through Tailscale) and `lastSeen`. The server asks each online Linux device on its
tailnet (2 seconds each, in parallel) and caches the answer for 10 seconds.

### The share code

Off unless you turn it on. Then both addresses want the code in `X-Hearth-Share-Code: <code>` (or `Authorization:
Bearer <code>`) and answer **401** without it (wrong codes slow down). Only a PBKDF2-SHA256 hash is kept
(`~/.local/state/nimbus/ai-share.json`, 0600). The Home server passes the code it was given on to the devices it asks,
so use one code everywhere. The code guards the list; the services themselves keep their own rules (Ollama's port, for
example, is behind the home server's Tailscale-only rule).

### What the Command Center does with it

1. Ask `GET <Command Center address>/v1/hearth/ai/devices` (refresh every 30–60 seconds, or when a job fails).
2. For a job of kind K, walk `devices` in order and take the first with `online && available && !gaming && !busy`
   and a `services[]` entry of kind K with `healthy`; fall back to one that is `busy`, then one that is `gaming`
   (only if its `available` is still true).
3. Talk to that service's `url` (or `openai`) directly.

## How it works

- **The program** `hosts/linux/desktop/shell/hearth-ai-share` (Python's standard library only): Settings' commands
  (`status`, `devices`, `set`, `code-set`, `code-clear`, `move`, `order-reset`, `apply`) and the server (`serve`).
- **Finding services**: by asking them on this device (127.0.0.1, 1.5 seconds each, never by assuming what installed
  them): Ollama `GET /api/tags` (and `/api/ps`) on 11434; Kokoro-FastAPI `GET /v1/audio/voices` on 8880; ComfyUI `GET
  /system_stats` (models from `/models/checkpoints`, busy from `/queue`) on 8188; whisper on 8000 (an OpenAI-style
  `/v1/models` with a whisper model: speaches / faster-whisper-server) or 9000 (whisper-asr-webservice's
  `/openapi.json`). A Hearth-installed app (`/var/lib/hearth/apps/<name>`), or the AI, AI pictures or AI voices that
  Settings › Home server runs here (`home-server.json`, read only), that isn't answering (stopped while you play, still
  starting) is listed as not healthy rather than missing. Whether a service can be reached from Tailscale comes from `/proc/net/tcp` (listening on every address or the
  Tailscale address, not only 127.0.0.1).
- **The service** `hearth-ai-share.service` is your own user service (as you; Python), enabled by Settings only while
  "Share this device's AI" or "Let the Command Center find my devices here" is on, and disabled again when both are
  off. It reads your settings on every request (read only) and checks every 20 seconds where to listen: 127.0.0.1
  always; the Tailscale addresses only once the firewall rule is loaded (`/run/hearth/ai-share-firewall.json` says so);
  every address with "Also on my home network". It also refuses any client that isn't this device or Tailscale.
- **The firewall rule** (Hearth's nftables approach, like Ollama's): `hearth-ai-share-helper firewall on` (root, via
  pkexec and `49-hearth-storage.rules`) writes `/etc/hearth/ai-share.nft`, Hearth's own table `inet hearth_ai_share`
  (input hook, priority -10: `lo` and `tailscale0` accepted, TCP 8189 dropped), loads it, and enables
  `hearth-ai-share-firewall.timer` (15 seconds after the PC starts, then every 5 minutes, so it's back if something
  removed it). `firewall off` deletes the table, the file and the timer. A rule that can't load is said in Settings and
  the service stays on this device only.
- **Playing**: `gaming` is reported from gamescope or `hearth-playing`, as Home server's Light mode sees it. "Not while
  I'm playing" and "Share only when plugged in" only change what the device reports (`available: false` and why); no
  service is stopped and no setting changes.
- **Every device**: the Tailscale peers (`tailscale status --json`, online, Linux), each asked for
  `/v1/hearth/ai`; devices that answered before are remembered in `~/.cache/nimbus/ai-share-devices.json` (cache, not a
  setting) and listed offline; paired Hearth Link devices (`hearth-link call status`) not seen through Tailscale are
  listed by name.

### Built on Hearth Link

Hearth Link already shares a device's local model server between paired devices over its own encrypted connection
(`hosts/hearth-link/src/ai.rs`, Settings › Hearth Link › Local AI in the old Game Mode: `ai.json` `serve`). This
doesn't replace it: Hearth Link's share is for apps on your other Hearth devices (through `127.0.0.1:11435`); this
is for anything on your tailnet, the Command Center most of all, and covers voices, pictures and speech to text too.
The Home server's list uses Hearth Link's paired devices so a paired device that isn't sharing still shows up.

What Rust work would make it nicer later: Hearth Link's `ai.info` could answer with this same document (so the Home
server could list a paired device's AI without Tailscale), and `ai.status` could include each peer's Tailscale name to
match devices by id instead of by name.

## Settings and state

- **Settings**: `~/.config/nimbus/ai-share.json` through `hearth_settings_io` (docs/SETTINGS-PERSISTENCE.md): `on`,
  `hub`, `lan`, `pluggedOnly`, `notWhilePlaying`, `codeOn`, `share` {chat, voice, image, stt} and `priority` (device
  names). Written only when you change something and only that key (`share.voice` writes only that key of the group;
  a value equal to what you have, default included, isn't written), atomic with `.bak`, merged under the lock;
  defaults are filled in when read. A damaged file is put back from `.bak` (or kept aside), never overwritten with
  defaults; the server only ever reads it.
- **State**: the share code's hash (`~/.local/state/nimbus/ai-share.json`, 0600), devices seen before
  (`~/.cache/nimbus/ai-share-devices.json`), the firewall rule's result (`/run/hearth/ai-share-firewall.json`).

## Never stopping the PC from starting

`hearth-ai-share.service` (user) and `hearth-ai-share-firewall.timer` are disabled in the image (the build fails if one
is on, and base-peppermint.yml's image check looks again); `hearth-ai-share-firewall.service` has no `[Install]`, starts
only from its timer and only with its rule file, and `/etc/hearth/ai-share.nft` is never in the image. Nothing has
`Requires=` or is wanted by the session, Game Mode or Steam Game Mode; nothing touches the bootloader, the sessions or
nimbus-session.

## Tests

`hosts/linux/test/test_hearth_ai_share.py` (run by home-server.yml): settings off by default and per key, never
defaults over a damaged file, the share code as a hash only; finding each service by asking it (and not mistaking
something else on its port), a service on this device only, busy; the document (playing, on battery, plugged in, the
per-service switches, no secrets); every device in order, offline ones from before, Hearth Link's, moving and resetting
the order, another device's answer checked; the server over a real socket (GET only, 404s, the share code, who may
ask), where it listens; the root helper's rule, on and off; the units, the image checks and the page's wiring.
