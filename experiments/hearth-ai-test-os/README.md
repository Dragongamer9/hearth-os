# Hearth AI Test OS — isolated experiment

This folder is a **runnable concept prototype**, not a bootable ISO, full OS replacement, or a finished AI integration.

It is intentionally kept separate from the production Hearth UI and Fedora/Anatase bootc images.

## Run on your Linux PC

Requires Python 3.9+ (standard library only).

```bash
cd experiments/hearth-ai-test-os
python3 server.py
```

The tool prints and opens a loopback URL. On a remote machine, use `python3 server.py --no-browser` and open its printed URL **on that machine**. Stop with Ctrl+C.

## What works

- Dark Hearth-themed responsive Command Center dashboard based on the concept design.
- Detects Claude Code, Codex, Gemini CLI, and Ollama via executable lookup in PATH.
- Detects Steam, Heroic, Lutris, Gamescope, and MangoHud.
- Shows actual platform, hostname, and available CPU thread count.
- Sends an explicit user-entered prompt to the selected, installed CLI; no shell injection and 90-second timeout.
- Loopback-only ephemeral-token local HTTP server; no third-party dependencies.
- Does not run privileged commands, install packages, or modify the host system.

## Known limitations and safety

- This is a **test app running on an existing system**, not an ISO or a new bootable distro.
- Some CLI tools may require their own login, licensing, or changed flags; a detected executable is not proof of authentication.
- Ollama needs a local `llama3.2` model installed. The test never pulls it automatically.
- The AI CLI receives prompt text and may contact its vendor. Do **not** paste secrets.
- Since each CLI is an external program, review its local configuration and permission settings before use. Do not give it an unrestricted workspace.
- It uses the user's CLI environment, which may include provider credentials. Do not expose the local URL or run untrusted browser extensions on this session.
- Shared handoffs, real performance telemetry, Steam library controls, updates and desktop shell integration are **future work**, not simulated functionality.

Next steps: write a native host bridge for the existing `hosts/hearth-core` plus permission prompts, then wire that into `ui/index.html` behind an experimental feature flag. Keep default release unaffected.

## Live system monitoring (test branch)

The dashboard now polls `/api/status` every five seconds while visible. Linux telemetry comes directly from `/proc/stat`, `/proc/meminfo`, `/sys/class/thermal`, and `/sys/class/drm`, without privileged commands. Initial CPU usage is unavailable until a second sample. GPU utilization and FPS are intentionally marked unavailable rather than fabricated.

Run the basic telemetry tests from this experiment directory:

```bash
python3 -m unittest discover -s tests -v
```

The visual preview screenshot produced during development is illustrative; the interface reports real values only when connected to its local server.
