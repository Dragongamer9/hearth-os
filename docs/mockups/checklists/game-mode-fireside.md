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
- [x] Source and Installed over the big name — ui/index.html: "<div class=\"fs-k\">${src} · ${state}</div>"
- [x] The description with when you last played and for how long — ui/index.html: "Last played ${ago(g.last).toLowerCase()}"
- [x] Play, Settings for this game and ⋯ — ui/index.html: "Settings for this game</button>"
- [x] The facts card: Runs on, Performance, Controller, Cloud save, Size — ui/index.html: "${fact('Runs on',runs)}${fact('Performance',perf)}${fact('Controller',pad)}${fact('Cloud save',cloud)}${fact('Size',size)}"
- [x] The big cover on the right with the name on it — ui/index.html: "<div class=\"fs-gcover\">"
- [x] "Changes here are for this game only" — ui/index.html: "Changes here are for this game only"

## 4. Store
- [ ] "New in the Hearth Store": the newest app big, with Get and See more
- [ ] Updates: how many are ready, each app with its new version, whether it needs a restart, and Update
- [ ] Apps for you: a grid of Store apps with Get
- [ ] Update all (X)

## 5. Quick settings
- [x] A panel on the right over the game — ui/index.html: "body.look-fs #qam .qd-wrap{position:fixed;right:20px"
- [x] "Quick settings" over "While you play" — ui/index.html: "S.running?'While you play':'Right now'"
- [x] Power chips Quiet, Balanced, Max with their watts — ui/index.html: "const FS_PMODES = [['efficiency','Quiet'],['balanced','Balanced'],['performance','Max']];"
- [x] Brightness and Volume sliders — ui/index.html: "if(S.sys.display.supported) h += sliderCtl('bright','sun','Brightness');"
- [x] Frame rate limit slider — ui/index.html: "if(k==='fps:limit') return fsFpsSlide(el, d);"
- [x] Performance overlay, Wi-Fi, Controller and Screenshot rows — ui/index.html: "row('shot','ovShot','Screenshot'"

## 6. Settings
- [x] The list down the left — ui/index.html: "<nav class=\"fs-snav\" aria-label=\"Settings\">"
- [x] "Settings" over the page's big name — ui/index.html: "<div class=\"fs-smain\"><div class=\"fs-k\">Settings</div>"
- [x] The rows in one card with their values and › — ui/index.html: "body.look-fs .fs-smain .sc-main{background:var(--panel)"
- [x] "Saved on this PC · nothing changes by itself" — ui/index.html: "Saved on this PC · nothing changes by itself"
- [ ] Display: "Put my screen settings back when the TV reconnects" in Hearth Game Mode
- [ ] Display: HDR in Hearth Game Mode
- [ ] Display: Night light in Hearth Game Mode

## 7. Power menu
- [x] "Power" over "Take a break?" — ui/index.html: "<div class=\"fs-ph\">Take a break?</div>"
- [x] "Your game is paused and saved where you are." — ui/index.html: "Your game is paused and saved where you are."
- [x] Sleep (Back in a second), Restart, Turn off, Desktop Mode (Mantel) as big choices — ui/index.html: "${o('p-desktop','desktop',I.desk,'Desktop Mode',LINUX?'Mantel':'Your desktop')}"
- [x] The picked one glows orange — ui/index.html: "body.look-fs .fs-po.focus{background:linear-gradient(180deg,var(--accent2),var(--accent))"
