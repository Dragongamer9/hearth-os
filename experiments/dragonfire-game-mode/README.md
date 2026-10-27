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
