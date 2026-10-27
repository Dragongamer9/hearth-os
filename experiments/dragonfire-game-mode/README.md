# Dragonfire Game Mode — experimental Gamescope UI

This directory contains an **opt-in controller-first interface experiment** inspired by the dragonfire Game Mode concept.

Open `index.html` in a normal browser for a clearly labeled **preview**. To use a real Hearth library, the HTML must be hosted by Hearth's existing UI engine with the supported `window.nimbusHost` (or `window.chrome.webview`) bridge. The page calls `library.get` and `game.launch` only; a normal browser is **never allowed to launch sample games**.

The real Hearth Game Mode remains at `ui/index.html`; this experiment **does not replace it** or alter Gamescope startup settings.

## Controls

Arrow keys, Tab and controller D-pad navigate focus; Enter or controller A selects. Escape takes focus back to Play. Gamepad input depends on browser support for the Gamepad API.

## Compatibility requirements before enabling in an OS image

1. Serve the page through the existing trusted Hearth host, not an arbitrary public web page.
2. Verify all `library.get` and `game.launch` response shapes and native bridge permissions.
3. Test on Gamescope with a controller and screen sizes from handheld 1280×800 to desktop 4K.
4. Ensure the game launch path hides Hearth correctly, resumes after exit, and preserves Quick Access and Quick Resume.
5. Never ship sample preview titles as real installed games.

## ISO status

The repository's maintained installer is `.github/workflows/live-iso.yml`, which consumes an already-published Anatase OS image and boot-tests its output. Its normal `latest` / `testing` tags point to public Hearth releases, **not** this experimental branch.

A real Dragonfire ISO requires adding the tested UI to an experimental OS image, building that image in CI, and building and boot-testing a live ISO against that exact image digest. A plain screenshot or the HTML experiment is not an ISO. Do not run the archived Bazzite workflow.

## Gamescope preview (opt-in; does not replace the game session)

On a Linux desktop with Gamescope, Python 3, and Chromium or Firefox available, run:

```sh
cd experiments/dragonfire-game-mode
sh preview-in-gamescope.sh
```

This opens the **browser-only UI preview inside a nested 1280×800 Gamescope window**. It does not carry Hearth's host bridge, does not show your real library, cannot start games, and does not modify session settings. The real Gamescope boot session and watchdog remain unchanged. Close the browser with Alt+F4 to exit the preview.

The experiment now has explicit D-pad/left-stick focus traversal, A to activate, and B to return focus to Play. Native controller testing is still required on real hardware.

## Current blocker for a new experimental ISO

The existing live ISO workflow pulls `ghcr.io/dragongamer9/hearth-os:testing` or `:latest`, not this branch's UI. There is no verified branch-specific Anatase image digest carrying Dragonfire. Do not claim an ISO was produced until this separate branch has been integrated into a bootc image, built successfully, and passed the repository's QEMU boot check. No image was published and no ISO build was started by this commit.

## Manual Dragonfire live ISO workflow (new)

Open **Actions → Dragonfire experimental live ISO (manual, never published)** on the `experiment/hearth-ai-test-os` branch and choose **Run workflow**.

The workflow:
- builds an isolated preview overlay atop the existing Anatase `:testing` image;
- adds Dragonfire as an **optional launcher** without replacing the normal Hearth Game Mode;
- creates a live ISO using Hearth's pinned Titanoboa workflow;
- checks that the generated ISO contains the expected boot kernel and initramfs; and
- retains the ISO artifact for seven days, without uploading it to releases, Cloudflare R2, or the production registry.

**Important:** This is a preview-only live ISO pipeline, not a release installer for a fully integrated Dragonfire OS. The included Hearth installer retains its normal upstream installation/update targets; installing from this experiment could install ordinary Hearth instead of this experimental overlay. **Use live mode or a disposable virtual machine; don't install over your main PC.**

The GitHub Actions workflow has been committed but has **not been dispatched**, and there is no verified ISO artifact yet. The structural check is not a boot test. Complete QEMU boot validation and actual controller/game-launch testing remain mandatory before calling the result usable.
