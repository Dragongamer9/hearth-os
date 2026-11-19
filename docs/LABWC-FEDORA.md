# Hearth Classic (labwc + Mantel) and Firehold on labwc on the Fedora edition (1.16, work in progress)

The owner (Nov 19, 2026): "Also port over the entire labwc as well, have it as a selection. But FIX all the problems it
had. Don't have it as a fallback either." And: "also start a Firehold version that uses labwc so I can test them all."

Status: research done, nothing built yet (paused for usage). Branch `feature/labwc-fedora`.

## The plan

- Settings › Your desktop on Fedora offers four: KDE Plasma, Firehold (Plasma, test), **Mantel (Hearth Classic,
  labwc)** (Debian's own name for it) and **Firehold on labwc** (Mantel's Firehold.qml Couch and Desk, as Debian ran it).
- Saved as session.conf's `HEARTH_PLASMA_DESKTOP=classic` / `classicfirehold` (letters only: the existing readers take
  `[a-z]*`, so a value like `firehold-labwc` would be read as Firehold on Plasma). Written only by a pick, through
  hearth_settings_io. Note: `hearth-firehold-plasma choose plasma` writes nothing when its own reader already says
  plasma, so the Rust Settings' KDE Plasma pick must also clear `classic` explicitly (data.rs desk_cmd).
- nimbus-session: a marked block before `run_plasma` on the Plasma image. labwc runs only when picked; Plasma or
  Firehold failing never starts labwc. labwc ending within 15 s or Mantel not up within 45 s (hearth-shell-up): KDE
  Plasma for that sign-in with a notice; twice in a row: Plasma until picked again. No loops.
- Firehold on labwc: Mantel on labwc with `HEARTH_FIREHOLD_SESSION=1`, which Firehold.qml reads as unlocked for that
  session (firehold.json is never written for it) and starts in Couch unless startIn says otherwise.

## Packages on Fedora 45

- labwc: in Fedora 45's repos (0.20.2-2.fc45).
- Quickshell: Fedora 45 has 0.2.1^git20260209 only; Debian builds v0.3.2 (os-bases/peppermint/builder.sh). Plan: build
  the pinned v0.3.2 in a builder stage of os-bases/anatase/Containerfile from the same base image (same Qt), and fail the
  build if the Qt it was built against isn't the image's.
- The rest of Mantel's Debian list (peppermint build.sh "Desktop Mode: labwc + Quickshell", Quickshell's QML modules,
  KDE pieces) still to be mapped to Fedora names: qt6-qtdeclarative, qt6-qtwayland, qt6-qtsvg, qt6-qt5compat, qt6ct,
  foot, wlsunset, wlopm, cliphist, wtype, xdg-desktop-portal-wlr/-gtk, a polkit agent (polkit-kde is already in).

## Problems it had (from the owner's logs, Nov 10-19, and the roadmap)

1. **The shell busy at rest, all the time.** "Desktop shell health" in the owner's logs: on 1.13 the laptop (i3-N305)
   held 6-8 % of a core with nothing open; on 1.14 13-15 % on the laptop (WYGE-6CCV-8M) and 21-27 % on the ROG Ally
   (T7CS-FRKM-59), never settling. This is the lag and near lock-up that pulled 1.14. Mantel has 111 repeating timers;
   several run always (`running: true`): Bar.qml 10 s and 15 s, Islands.qml 10 s and 15 s, Activities.qml 2 s,
   Recorder.qml 1 s and 2 s, HintStrip.qml 1 s, FireholdDesk.qml 15 s, FireholdFramesComp.qml 5 s, ServerDash.qml 4 s
   and 10 s, VaultStore.qml 15 s, OomWatch.qml 20 s, BatteryWatch.qml 30 s; many start helper programs (Python) each
   time. Fix to do: none of these runs at rest unless what it shows is on screen (event-driven where possible), and CI
   measures the shell's CPU at rest and over 10 minutes under Plasma's limits.
2. **Memory far over the known 270 MB.** 1.14: 400-670 MB on the laptop, 750 MB-1.15 GB on the Ally, swinging up and
   down (images and pages kept loaded). Fix to do: pages and windows unloaded when closed; CI fails if memory grows.
3. **Laptop freezes on the Intel N-series screen** ("Atomic commit failed: Device or resource busy", touchpad
   "event processing lagging behind by 263 ms"; G2W9-WQTH-FK, F2RL-BJY4-SV): 1.14's Screen power saving (PSR) and
   memory fixes cover the driver side; the shell's constant redraws made it worse.
4. **Slow Files** (roadmap 1.15: as slow warm as cold, 0.5 s): to measure and fix, with a CI time limit.
5. Log noise to quiet: FileView warnings for settings files that don't exist yet (Mixer, NotifyHub, BarLayout,
   Activities, Launcher, WidgetStore), `eglSwapBuffers failed`, MPRIS property errors for closed players.

## Firehold on labwc versus the 12 approved 1.16 Firehold mockups

Not compared yet (feature/firehold-plasma-1.16 has them). Firehold on labwc brings over Debian's Firehold as it was.
