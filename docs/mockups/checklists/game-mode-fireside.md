# Hearth Game Mode in the installer's look: Fireside (approved Nov 28, 2026)

The owner, Nov 28: "Im liking our New install setup, we should make Hearth game mode closer to that look by default",
then on the boards: "I approve them, add them as a look, also enable them as part of the modular deal". A new look,
Fireside (Settings › Appearance › Look), whose Home and Library are made of blocks (gm-blocks.js, ✎ Edit Game Mode).
Sources: hearth-workspace `hearth/mockups/2026-11-28-game-mode-installer-look`.

Mockups: game-mode-fireside/home.png, game-mode-fireside/library.png, game-mode-fireside/game.png, game-mode-fireside/store.png, game-mode-fireside/quick.png, game-mode-fireside/settings.png, game-mode-fireside/power.png
Release: next

## The look
- [x] Fireside in the list of looks — ui/index.html: "{id:'fireside', name:'Fireside'"
- [x] The installer's warm ember ground, cream text and orange pill buttons — ui/index.html: "body.look-df.look-fs{--bg:#120d0a"
- [x] The thick orange focus ring — ui/index.html: "body.look-fs [data-f].focus{box-shadow:0 0 0 3px var(--bg),0 0 0 6px var(--accent)"
- [x] Menu along the top with the current page as a pill — ui/index.html: "else if(fsOn()) b.classList.add('L-spotlight','look-df','look-fs');"
- [x] Part of the blocks system, its own saved layout — ui/gm-blocks.js: "'nimbus.gmblocks.fireside'"

## 1. Home
- [x] "Jump back in" over your last game's big name — ui/gm-blocks.js: "Jump back in"
- [x] Its time and source under the name — ui/gm-blocks.js: "fs-meta"
- [x] Play, Game page and ⋯ — ui/gm-blocks.js: "fs-acts"
- [x] The big glowing flame on the right — ui/gm-blocks.js: "fs-fire"
- [x] The Recent shelf of covers — ui/gm-blocks.js: "{shows: 'recent', title: 'Recent', count: 6}"
- [x] Home made of blocks you can move — ui/index.html: "if(fsOn() && typeof gmbView==='function') return gmbView('home');"

## 2. Library
- [x] Big "Library" title with how many games and how many installed — ui/gm-blocks.js: "fs-lhead"
- [x] Filter chips: All, Installed and each store — ui/gm-blocks.js: "fs-filters"
- [x] Big covers six across with the store and the name — ui/index.html: ".fs-grid{display:grid;grid-template-columns:repeat(6,minmax(0,1fr))"
- [x] Not installed marked on the cover — ui/gm-blocks.js: "Not installed"

## 3. A game's page
- [ ] Source and Installed over the big name, the description with last played
- [ ] Play, Settings for this game and ⋯
- [ ] The facts card: Runs on, Performance, Controller, Cloud save, Size
- [ ] The big cover on the right

## 4. Store
- [ ] The Store page in the Fireside look

## 5. Quick settings
- [ ] A panel on the right over the game: "While you play"
- [ ] Power chips Quiet, Balanced, Max with their watts
- [ ] Brightness, Volume and Frame rate limit sliders
- [ ] Performance overlay, Wi-Fi, Controller and Screenshot rows

## 6. Settings
- [ ] Hearth Game Mode's Settings in the Fireside look

## 7. Power menu
- [ ] The power menu in the Fireside look
