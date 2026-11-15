# Hearth Games, stage two: Amazon, cloud saves and DLC (approved Nov 15, 2026)

Mockups: next-update/games-1-library.png, next-update/games-2-game-page.png, next-update/games-3-stores.png
Release: 1.13

Store art in the pictures is sample art; real games show their own.

## Library (games-1)
- [x] Amazon chip beside All, GOG, Epic Games, Battle.net, Installed — hosts/hearth-gaming/ui/games.js: "['amazon', 'Amazon'], ['battlenet', 'Battle.net']"
- [x] Amazon games in the library with their store and Installed — hosts/hearth-core/src/library.rs: "amazon"
- [x] a download's percentage badge on its tile — hosts/hearth-gaming/ui/games.js: "Math.round(g.progress * 100) + '%'"
- [x] Library, Downloads, Stores, Recomps tabs with LB / RB, and the button hints — hosts/hearth-gaming/ui/games.js: "Library, Downloads, Stores, Recomps</span>"

## A game's page (games-2)
- [x] Cloud saves: "Sync with <store>" switch with what it does and when it last synced — hosts/hearth-gaming/ui/games.js: "Before you play, your newest saves come down; after, they go up."
- [x] Bring saves down now / Send saves up now — hosts/hearth-gaming/ui/games.js: "Bring saves down now</button><button class=\"btn f\" data-x=\"sync-up\">Send saves up now</button>"
- [x] where the saves are — hosts/hearth-gaming/ui/games.js: "<p class=\"note\">Saves are in <span class=\"path\">"
- [x] saves really synced before and after playing — hosts/hearth-core/src/game_extras.rs: "pub fn sync_now("
- [x] DLC list: Installed / Not installed with sizes — hosts/hearth-gaming/ui/games.js: "x.owned === false ? 'Not on your account' : 'Not installed'"
- [x] Install and Uninstall per DLC — hosts/hearth-gaming/ui/games.js: "data-x=\"dlc-remove\" data-dlc="
- [x] See it in the store for DLC you don't own — hosts/hearth-gaming/ui/games.js: "See it in the store</button>"
- [x] Install all N — hosts/hearth-gaming/ui/games.js: "Install all ${missing}</button>"
- [x] DLC really installed in the background — hosts/hearth-core/src/game_extras.rs: "pub fn install_dlc("
- [x] Play, Uninstall, Back — hosts/hearth-gaming/ui/games.js: "<button class=\"btn f\" data-g=\"close\">Back</button>"

## Stores (games-3)
- [x] Amazon card: "prime" mark, "Signed in as …" — hosts/hearth-gaming/ui/games.js: "amazon: 'prime'"
- [x] games on your account, installed, when it last looked — hosts/hearth-gaming/ui/games.js: "on your account · ${a.installed || 0} installed"
- [x] "The games you claimed with Prime Gaming and the ones you bought on Amazon Games." — hosts/hearth-gaming/ui/games.js: "The games you claimed with Prime Gaming and the ones you bought on Amazon Games."
- [x] Look for new games, Open the store, Sign out — hosts/hearth-gaming/ui/games.js: "Look for new games</button><button class=\"btn f\" data-act=\"shop\""
- [x] signing in on Amazon's own page (nile) — hosts/hearth-core/src/amazon.rs: "pub fn sign_in()"
- [x] GOG, Epic, Battle.net cards and Where new games go, as before — hosts/hearth-gaming/ui/games.js: "${storeCard('gog')}${storeCard('epic')}${storeCard('amazon')}${battlenetCard()}"
