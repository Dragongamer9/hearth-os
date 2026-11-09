.pragma library
// Made by tools/build-settings-index.py from settings-pages.js and the pages' cards: Hearth Settings' search.
// {t: title, d: description, i: icon, p: the page it's on}. Don't edit by hand: run the tool again.
var ENTRIES = [
{
"t": "Rename this PC",
"p": "about",
"d": "Its name on your network and your other devices · on Home",
"i": "edit"
},
{
"t": "Count me in the player count",
"p": "player-count",
"d": "Hearth OS tells its counter it's running about every 10 minutes: a number made from this device, the edition and the version. No names, games or accounts.",
"i": "user"
},
{
"t": "Logs",
"p": "backup",
"d": "Read the system's and Hearth's logs, search them, and save or send them",
"i": "file"
},
{
"t": "Restore from a backup",
"p": "backup",
"i": "restart"
},
{
"t": "Zoom in now",
"p": "accessibility",
"d": "Super+= zooms in, Super+- zooms out (and off), Super+Esc turns it off or back on. The view follows the pointer",
"i": "search"
},
{
"t": "What it magnifies",
"p": "accessibility"
},
{
"t": "Zoom when Super+Esc turns it on",
"p": "accessibility",
"i": "search"
},
{
"t": "Each Super+= or Super+- step",
"p": "accessibility"
},
{
"t": "Screen reader (Orca)",
"p": "accessibility",
"i": "vol"
},
{
"t": "Text size",
"p": "accessibility",
"d": "For apps and window titles. Apps open now use it after you sign out and in again",
"i": "file"
},
{
"t": "High contrast",
"p": "accessibility",
"d": "Apps with GTK use their high-contrast look (after signing out and in)",
"i": "sun"
},
{
"t": "Large pointer",
"p": "accessibility",
"d": "A pointer twice the size (after signing out and in)",
"i": "mouse"
},
{
"t": "Reduce motion",
"p": "accessibility",
"d": "Still wallpapers and fewer animations (animated ones also hold still on battery and behind fullscreen games)",
"i": "motion"
},
{
"t": "Emoji and symbols",
"p": "accessibility",
"d": "Super+. opens the picker, in any app",
"i": "kbd"
},
{
"t": "Mode",
"p": "appearance",
"d": "Sunset (optional): light by day and dark by night, on the times below. Off unless you choose it.",
"i": "sun"
},
{
"t": "More accent colours",
"p": "appearance",
"d": "From your wallpaper, any hue, or a colour you type",
"i": "edit"
},
{
"t": "Custom accent",
"p": "appearance"
},
{
"t": "Wallpaper",
"p": "appearance",
"i": "image"
},
{
"t": "Choose a picture…",
"p": "appearance",
"d": "Any picture on this device. In the Files app you can also right-click a picture › Set as Hearth wallpaper.",
"i": "image"
},
{
"t": "Desktop clock and weather",
"p": "appearance",
"i": "clock"
},
{
"t": "Panel transparency",
"p": "appearance",
"i": "eye"
},
{
"t": "On-screen keyboard style",
"p": "appearance",
"d": "Its looks (Ember, Frost, Steam, Warm…) are in Bluetooth & devices › Keyboard",
"i": "kbd"
},
{
"t": "Hearth's Debian box",
"p": "apps",
"i": "store"
},
{
"t": "Install a .deb file",
"p": "apps",
"d": "Open it in Files (double-click, or right-click › Install). Downloads opens now.",
"i": "folder"
},
{
"t": "Reset the Debian box…",
"p": "apps",
"d": "A fresh, empty box: everything installed in it is removed",
"i": "restart"
},
{
"t": "Reset it now",
"p": "apps",
"d": "Removes every package in the box and makes a new one (1–2 minutes)",
"i": "restart"
},
{
"t": "Remove the Debian box…",
"p": "apps",
"d": "Frees its space; it's made again the next time you install a .deb",
"i": "trash"
},
{
"t": "Remove it now",
"p": "apps",
"d": "The box, everything in it and its apps in your menu",
"i": "trash"
},
{
"t": "Cancel",
"p": "apps"
},
{
"t": "Bazaar",
"p": "apps",
"i": "store"
},
{
"t": "Fix your Flatpak folder",
"p": "apps",
"i": "info"
},
{
"t": "AppImages run from Files",
"p": "apps",
"d": "Double-click one to run it (Hearth asks once, then makes it runnable). Right-click › Add to app menu moves it to the Applications folder in your home folder and puts it in your menu; Remove from app menu takes it out again.",
"i": "info"
},
{
"t": "Bar style",
"p": "bar-dock",
"i": "window"
},
{
"t": "Right-click menu",
"p": "bar-dock",
"d": "How the menu looks on files, the desktop, the bar, the dock and the app menus",
"i": "menu"
},
{
"t": "Bluetooth",
"p": "bluetooth",
"i": "bt"
},
{
"t": "Steam's own",
"p": "startup-animation",
"d": "The animation Steam picks (or the one you chose in Steam). Choose to use it.",
"i": "motion"
},
{
"t": "Open Hearth Browser",
"p": "browser",
"i": "globe"
},
{
"t": "Make Hearth Browser the default",
"p": "browser",
"d": "Links from other apps open in it",
"i": "check"
},
{
"t": "Bigger buttons and text",
"p": "browser",
"d": "For a controller and a small screen. Applies the next time it opens."
},
{
"t": "Aloha Browser",
"p": "browser",
"i": "info"
},
{
"t": "Install Aloha Browser",
"p": "browser",
"d": "Downloads the official package from alohabrowser.com (about 150 MB) and makes it the default browser",
"i": "update"
},
{
"t": "Make Aloha the default browser",
"p": "browser",
"i": "check"
},
{
"t": "Look for an update",
"p": "browser",
"d": "Hearth also checks once a week",
"i": "restart"
},
{
"t": "Remove Aloha Browser",
"p": "browser",
"d": "Hearth won't install it again by itself",
"i": "close"
},
{
"t": "Claude Desktop",
"p": "claude",
"i": "info"
},
{
"t": "Unofficial on this edition",
"p": "claude",
"d": "Anthropic supports its Linux app on Debian and Ubuntu. Here it's Anthropic's Debian build, run by Hearth from your home folder: it should work, but Anthropic doesn't support it yet.",
"i": "info"
},
{
"t": "This system blocks the app's sandbox",
"p": "claude",
"d": "Claude Desktop needs user namespaces, which every Hearth OS edition allows. It won't start until they're allowed again.",
"i": "info"
},
{
"t": "Get Claude",
"p": "claude",
"d": "Installs Claude Desktop for you, then opens it. Sign in with your Claude account in the browser.",
"i": "update"
},
{
"t": "Open Claude",
"p": "claude",
"i": "forward"
},
{
"t": "Remove Claude Desktop",
"p": "claude",
"d": "Your chats are kept in your Claude account; settings stay in your home folder",
"i": "close"
},
{
"t": "Claude Code",
"p": "claude",
"i": "info"
},
{
"t": "Get Claude Code",
"p": "claude",
"d": "Runs Anthropic's installer (claude.ai/install.sh) for you",
"i": "update"
},
{
"t": "Open in the Terminal",
"p": "claude",
"i": "forward"
},
{
"t": "Remove Claude Code",
"p": "claude",
"d": "Your settings in ~/.claude stay",
"i": "close"
},
{
"t": "Keep clipboard history",
"p": "clipboard",
"i": "clipboard"
},
{
"t": "Open clipboard history",
"p": "clipboard",
"d": "Super+V",
"i": "clipboard"
},
{
"t": "Clear clipboard history…",
"p": "clipboard",
"i": "trash"
},
{
"t": "Yes, clear it",
"p": "clipboard"
},
{
"t": "Cancel",
"p": "clipboard"
},
{
"t": "Controller mouse",
"p": "controller",
"d": "Right stick moves the pointer, R2 clicks. Turns off by itself while a game is full screen.",
"i": "mouse"
},
{
"t": "Pointer speed",
"p": "controller",
"i": "mouse"
},
{
"t": "Controller type games see",
"p": "controller",
"d": "Some games show button prompts to match",
"i": "pad"
},
{
"t": "Nintendo layout",
"p": "controller",
"d": "Swaps A with B, and X with Y",
"i": "pad"
},
{
"t": "Reset all buttons",
"p": "controller",
"d": "Back to how the controller came",
"i": "restart"
},
{
"t": "Handheld Daemon settings",
"p": "controller",
"d": "Everything for the built-in controller, power limit, fan and lights: Settings › Handheld",
"i": "gear"
},
{
"t": "Controller buttons",
"p": "controller",
"d": "What each button does in Desktop Mode",
"i": "help"
},
{
"t": "On-screen keyboard",
"p": "controller",
"d": "Y on the controller, or here",
"i": "kbd"
},
{
"t": "Back",
"p": "controller",
"i": "back"
},
{
"t": "Clock",
"p": "datetime",
"d": "How the taskbar shows the time",
"i": "clock"
},
{
"t": "Set the time automatically",
"p": "datetime",
"i": "restart"
},
{
"t": "Restart the desktop now",
"p": "datetime",
"d": "The taskbar clock shows the new time zone after the desktop restarts (your apps stay open)",
"i": "restart"
},
{
"t": "All regions",
"p": "datetime",
"i": "back"
},
{
"t": "Reset to Hearth's default",
"p": "defaults",
"i": "restart"
},
{
"t": "Reset to Hearth's defaults…",
"p": "defaults",
"i": "restart"
},
{
"t": "Yes, reset them",
"p": "defaults"
},
{
"t": "Cancel",
"p": "defaults"
},
{
"t": "Let my other devices see this device's status",
"p": "devices",
"d": "Battery, use, temperatures, drives and what's playing, only for devices paired with this one (Hearth Link). Never the internet.",
"i": "eye"
},
{
"t": "Show other devices in Files",
"p": "devices",
"d": "Your devices' shared folders, and other computers sharing on this network or through Tailscale, in Hearth Files' sidebar and Open / Save windows",
"i": "folder"
},
{
"t": "Back to your devices",
"p": "devices",
"i": "back"
},
{
"t": "Server apps",
"p": "devices",
"i": "box"
},
{
"t": "Open its dashboard",
"p": "devices",
"d": "Its Server mode web page over Tailscale (its PIN): pause, restart, switch modes",
"i": "globe"
},
{
"t": "Wake it",
"p": "devices",
"d": "Wake-on-LAN, as Hearth Storage does",
"i": "power"
},
{
"t": "Remote desktop",
"p": "devices",
"d": "Use it from here, or play its games (Remote play)",
"i": "play"
},
{
"t": "Hearth Link",
"p": "devices",
"d": "Pairing, sync, copies and Hearth Storage",
"i": "desktops"
},
{
"t": "Which Discord",
"p": "discord",
"i": "chat"
},
{
"t": "Open Discord",
"p": "discord",
"d": "Full size. The bar's Discord button opens it in a small window you can reply to friends from",
"i": "window"
},
{
"t": "Discord application ID",
"p": "discord",
"i": "key"
},
{
"t": "Show what I'm playing on Discord",
"p": "discord",
"d": "Rich Presence: the game, its picture and how long you've played, while Discord is running",
"i": "play"
},
{
"t": "Let friends ask to join",
"p": "discord",
"d": "Friends see Ask to join / Join on your game in Discord. Joining opens the game on their Hearth, or Play together when Remote play (Sunshine) is on",
"i": "users"
},
{
"t": "Keep",
"p": "display"
},
{
"t": "Go back",
"p": "display"
},
{
"t": "Screen",
"p": "display",
"d": "The settings below are for this one",
"i": "display"
},
{
"t": "Brightness",
"p": "display",
"d": "Only for screens that let Hearth change it",
"i": "sun"
},
{
"t": "Night light",
"p": "display",
"d": "Warmer colours from sunset to sunrise, to help you sleep",
"i": "moon"
},
{
"t": "Scale",
"p": "display",
"d": "Makes text, apps and the bar bigger or smaller",
"i": "display"
},
{
"t": "Refresh rate",
"p": "display",
"d": "Higher is smoother; lower saves battery",
"i": "bolt"
},
{
"t": "Where this screen is",
"p": "display",
"i": "desktops"
},
{
"t": "Use this screen",
"p": "display",
"i": "display"
},
{
"t": "Resolution",
"p": "display",
"d": "Sharper with more pixels",
"i": "window"
},
{
"t": "Graphics card (Nvidia): how the Hearth app draws",
"p": "display",
"d": "Fast is smooth on newer drivers. Compatible is slower but works everywhere: pick it if first-time setup or the Hearth app's window shows a black screen. Steam Game Mode isn't affected.",
"i": "cpu"
},
{
"t": "Hearth Games",
"p": "games",
"d": "Your GOG, Epic Games and Battle.net games: sign in, install, play and uninstall (docs/HEARTH-GAMES.md)",
"i": "store"
},
{
"t": "Epic Games",
"p": "games",
"d": "Sign in to see, install and play your Epic games (in Hearth Games)",
"i": "lock"
},
{
"t": "GOG",
"p": "games",
"d": "Sign in to see, install and play your GOG games (in Hearth Games)",
"i": "lock"
},
{
"t": "Battle.net",
"p": "games",
"d": "Blizzard's own app, set up through Faugus Launcher (in Hearth Games)",
"i": "lock"
},
{
"t": "Xbox",
"p": "games",
"d": "Sign in for Xbox Cloud Gaming and your Xbox PC games",
"i": "lock"
},
{
"t": "Library sources",
"p": "games",
"d": "Steam, Epic, GOG and emulators: which ones Hearth lists, add a game by hand, look for games again",
"i": "grid"
},
{
"t": "Downloads",
"p": "games",
"d": "Epic and GOG installs: pause, carry on, cancel (in Hearth Games)",
"i": "update"
},
{
"t": "Store",
"p": "games",
"d": "Deals, new games and search for Steam, GOG, Epic and Xbox",
"i": "store"
},
{
"t": "Recomps",
"p": "games",
"d": "Native PC versions of classic console games: install, update and play them (in Hearth Games)",
"i": "pad"
},
{
"t": "Import",
"p": "games",
"d": "Games and files from other drives and systems",
"i": "folder"
},
{
"t": "Get a game from another Hearth",
"p": "games",
"d": "Copy an installed game from one of your Hearth devices (Settings › Hearth Link pairs them)",
"i": "desktops"
},
{
"t": "Mods",
"p": "games",
"d": "Install mods and Nexus or Wabbajack modlists, turn them on and off, choose their order",
"i": "grid"
},
{
"t": "Saves",
"p": "games",
"d": "Back up and restore your games' saves, and move them to another PC",
"i": "folder"
},
{
"t": "Controller Lab",
"p": "games",
"d": "Test every button, stick and trigger; measure stick drift and calibrate",
"i": "pad"
},
{
"t": "Discord",
"p": "games",
"d": "Friends, chats and invites in Discord; what you're playing on Discord",
"i": "chat"
},
{
"t": "Theater",
"p": "games",
"d": "Videos from the web as tiles, full screen with a controller",
"i": "display"
},
{
"t": "Benchmark",
"p": "games",
"d": "How fast this device is, with scores and history",
"i": "gear"
},
{
"t": "Game Mode: Steam Game Mode or Hearth Game Mode",
"p": "gaming",
"d": "Which Game Mode this PC goes into: Steam's or Hearth's own",
"i": "pad"
},
{
"t": "Start in",
"p": "gaming",
"d": "What this PC opens when it turns on or you sign in",
"i": "power"
},
{
"t": "Decky Loader and Hearth Sync",
"p": "gaming",
"d": "Plugins in Game Mode's Quick Access menu"
},
{
"t": "Game storage",
"p": "gaming",
"d": "Games by size, moving games between drives, low space warning · System › Storage"
},
{
"t": "Light mode while you play",
"p": "gaming",
"d": "Home server jobs wait while a game runs. Sharing keeps working.",
"i": "server"
},
{
"t": "Check again",
"p": "handheld",
"i": "restart"
},
{
"t": "Show advanced settings",
"p": "handheld",
"i": "gear"
},
{
"t": "Back",
"p": "handheld",
"i": "back"
},
{
"t": "Share this device's AI · AI devices",
"p": "hs-ai",
"d": "Let your other devices and the Command Center use this PC's AI through Tailscale, and see every device's AI in one list",
"i": "desktops"
},
{
"t": "Shows once the Command Center is seen",
"p": "hs-ai",
"i": "info"
},
{
"t": "AI service",
"p": "hs-ai",
"i": "mic"
},
{
"t": "Who can reach it",
"p": "hs-ai",
"d": "Your devices through Tailscale, and this PC. Never your home network or the internet: Hearth's firewall rule lets only Tailscale in, and without it the service listens on this PC only",
"i": "lock"
},
{
"t": "Show 18+ models",
"p": "hs-ai",
"d": "Uncensored chat models for adults. Off on a family PC. They're just models in Ollama: the Hearth AI Command Center's own limits still apply to anything you ask through it",
"i": "lock"
},
{
"t": "Download any model by name",
"p": "hs-ai",
"i": "plus"
},
{
"t": "AI pictures (ComfyUI)",
"p": "hs-ai",
"i": "palette"
},
{
"t": "Open ComfyUI",
"p": "hs-ai",
"i": "open"
},
{
"t": "Voice Studio",
"p": "hs-ai",
"i": "vol"
},
{
"t": "Open Voice Studio",
"p": "hs-ai",
"i": "open"
},
{
"t": "AI voices (Kokoro)",
"p": "hs-ai",
"i": "vol"
},
{
"t": "Voices address for the Command Center",
"p": "hs-ai",
"i": "clipboard"
},
{
"t": "Paste this into the Command Center",
"p": "hs-ai",
"i": "globe"
},
{
"t": "Voices for the Command Center",
"p": "hs-ai",
"i": "vol"
},
{
"t": "Unload models when you play",
"p": "hs-ai",
"d": "When Game Mode or a game starts (with Light mode on), loaded models leave the graphics card within a minute; the next question loads them again",
"i": "pad"
},
{
"t": "Small model while you play",
"p": "hs-ai",
"d": "On the processor, so your devices still get answers while you play (Server mode's \"switch to the small model\")",
"i": "chat"
},
{
"t": "Stop the AI service while I play",
"p": "hs-ai",
"d": "Also stops it (your other devices can't ask it then), and starts it again when you stop playing",
"i": "sleep"
},
{
"t": "Unload models now",
"p": "hs-ai",
"d": "Frees the graphics card right away",
"i": "restart"
},
{
"t": "Remove Ollama",
"p": "hs-ai",
"d": "The app goes; your downloaded models stay (in /var/lib/hearth-ollama)",
"i": "trash"
},
{
"t": "Share this device's AI",
"p": "hs-aidevices",
"i": "mic"
},
{
"t": "Who can see it",
"p": "hs-aidevices",
"i": "lock"
},
{
"t": "No AI services found on this device",
"p": "hs-aidevices",
"d": "Hearth looks for Ollama chat (port 11434), Kokoro voices (8880), ComfyUI pictures (8188) and whisper speech to text (8000 or 9000). On a home server, Home server › AI installs Ollama",
"i": "info"
},
{
"t": "Share only when plugged in",
"p": "hs-aidevices",
"i": "power"
},
{
"t": "Not while I'm playing",
"p": "hs-aidevices",
"i": "pad"
},
{
"t": "Also on my home network",
"p": "hs-aidevices",
"d": "Off: only Tailscale and this device. On: anything on your home network can read the list too (the services keep their own rules)",
"i": "wired"
},
{
"t": "Ask for a share code",
"p": "hs-aidevices",
"i": "lock"
},
{
"t": "Let the Command Center find my devices here",
"p": "hs-aidevices",
"d": "This home server lists every device's AI at one address, so the Command Center needs only that",
"i": "globe"
},
{
"t": "Command Center address",
"p": "hs-aidevices",
"i": "globe"
},
{
"t": "Looking for your devices…",
"p": "hs-aidevices",
"i": "update"
},
{
"t": "Back to the usual order",
"p": "hs-aidevices",
"d": "A device with an Nvidia card first, then the others by graphics memory",
"i": "restart"
},
{
"t": "Every device in one list",
"p": "hs-aidevices",
"d": "On your Home server, this page lists every Hearth device's AI and gives the Command Center its address",
"i": "info"
},
{
"t": "Alerts",
"p": "hs-alerts",
"i": "bell"
},
{
"t": "A drive starts to fail",
"p": "hs-alerts",
"d": "Its health check (SMART) fails, or it runs very hot (65 °C or more)",
"i": "store"
},
{
"t": "A drive is almost full",
"p": "hs-alerts",
"d": "A drive with a share or a backup on it",
"i": "store"
},
{
"t": "Almost full means",
"p": "hs-alerts"
},
{
"t": "A backup fails or is late",
"p": "hs-alerts",
"d": "It didn't finish, didn't run for days, or its weekly check found a problem",
"i": "update"
},
{
"t": "A mirror is missing a drive",
"p": "hs-alerts",
"d": "A mirrored drive (Drives & undo) lost one of its drives or has read or write errors",
"i": "restart"
},
{
"t": "Also send them to my devices",
"p": "hs-alerts",
"i": "globe"
},
{
"t": "Send a test alert",
"p": "hs-alerts",
"d": "A notification here (and a note to the devices you picked)",
"i": "bell"
},
{
"t": "Sleep when nobody needs it for",
"p": "hs-alerts",
"d": "Never while Game Mode runs, a share is open or a backup runs. While you're signed in to the desktop, Settings › Power's own timer decides"
},
{
"t": "Stay awake while my devices use it",
"p": "hs-alerts",
"d": "While a share is open or a backup runs, nothing puts it to sleep (Settings › Power's timer included)",
"i": "sleep"
},
{
"t": "Wake-on-LAN (wake from your devices)",
"p": "hs-alerts",
"i": "wired"
},
{
"t": "Sleep at night, wake in the morning",
"p": "hs-alerts",
"i": "clock"
},
{
"t": "Sleep at",
"p": "hs-alerts"
},
{
"t": "Wake at",
"p": "hs-alerts"
},
{
"t": "Spin down idle storage drives after",
"p": "hs-alerts"
},
{
"t": "Right now",
"p": "hs-alerts",
"i": "info"
},
{
"t": "Shut down cleanly on battery",
"p": "hs-alerts",
"i": "batt"
},
{
"t": "I accept Minecraft's EULA",
"p": "hs-apps",
"i": "doc"
},
{
"t": "Who can reach it",
"p": "hs-apps"
},
{
"t": "Its data",
"p": "hs-apps",
"i": "folder"
},
{
"t": "Logs",
"p": "hs-apps",
"i": "file"
},
{
"t": "Phone notifications",
"p": "hs-apps",
"d": "Hear on your phone (the ntfy app) when an alert comes up or a server app stops",
"i": "bell"
},
{
"t": "How often",
"p": "hs-backups"
},
{
"t": "Keep",
"p": "hs-backups",
"d": "Older backups are thinned out: the newest each day, week and month are kept"
},
{
"t": "Restore…",
"p": "hs-backups",
"d": "Pick a backup, then a folder: it's copied into Home › Restored from backups, never over your files",
"i": "restart"
},
{
"t": "Change its password",
"p": "hs-backups",
"d": "The new password is used from the next backup on (restic keeps the old one working too)",
"i": "lock"
},
{
"t": "Remove this backup",
"p": "hs-backups",
"i": "close"
},
{
"t": "Add a backup",
"p": "hs-backups",
"d": "To a backup drive, another Hearth device on Tailscale, or the cloud (Backblaze B2 or S3)",
"i": "plus"
},
{
"t": "The cloud",
"p": "hs-backups",
"d": "Backblaze B2 or any S3-compatible storage, with your own account. Encrypted here before it leaves",
"i": "globe"
},
{
"t": "Cloud storage",
"p": "hs-backups"
},
{
"t": "Your Hearth devices' saves and screenshots",
"p": "hs-backups",
"d": "They back up to this PC through Hearth Storage (Hearth Link)",
"i": "desktops"
},
{
"t": "Backups from your other Hearth devices",
"p": "hs-backups",
"i": "update"
},
{
"t": "Where they go",
"p": "hs-backups",
"d": "A folder on a big drive is best (each device gets its own folder inside)",
"i": "folder"
},
{
"t": "Android",
"p": "hs-backups",
"i": "cam"
},
{
"t": "iPhone",
"p": "hs-backups",
"i": "cam"
},
{
"t": "Sync a folder",
"p": "hs-backups",
"d": "Pick a folder, one of your devices on Tailscale and its sync ID. Changed or deleted files are kept 30 days first",
"i": "plus"
},
{
"t": "Snapshots",
"p": "hs-drives",
"d": "Read-only copies of your shares' drives, taken every hour, day and week, to bring back what was deleted or changed. Only on btrfs drives; with Light mode they wait while you play.",
"i": "restart"
},
{
"t": "Keep hourly",
"p": "hs-drives"
},
{
"t": "Keep daily",
"p": "hs-drives"
},
{
"t": "Keep weekly",
"p": "hs-drives"
},
{
"t": "Browse this snapshot",
"p": "hs-drives",
"d": "Opens it in Hearth Files (read only): copy back what you need",
"i": "open"
},
{
"t": "Restore into a new folder",
"p": "hs-drives",
"d": "Copies the folder as it was into Home › Restored from snapshots. Nothing of yours is overwritten.",
"i": "restart"
},
{
"t": "Use it as",
"p": "hs-drives"
},
{
"t": "Mirror of",
"p": "hs-drives"
},
{
"t": "Can't use this drive",
"p": "hs-drives",
"i": "info"
},
{
"t": "Set up this drive…",
"p": "hs-drives",
"d": "First of two confirmations",
"i": "check"
},
{
"t": "Show what it would do",
"p": "hs-drives",
"d": "A dry run: nothing is changed",
"i": "info"
},
{
"t": "Not now",
"p": "hs-drives",
"i": "close"
},
{
"t": "Make a Family folder",
"p": "hs-family",
"d": "Home › Family, read & write for you and every person you added (People): holiday photos, school things, shared notes and lists",
"i": "folder"
},
{
"t": "Family folder",
"p": "hs-family",
"i": "folder"
},
{
"t": "People",
"p": "hs-family",
"d": "Family and friends with their own password and only the folders you pick",
"i": "user"
},
{
"t": "Calendar, notes and lists",
"p": "hs-family",
"d": "Keep them as files in the Family folder (any notes app on a phone can open a shared folder). A shared calendar server isn't part of Hearth yet",
"i": "calendar"
},
{
"t": "Recording to this PC",
"p": "hs-family",
"d": "Hearth doesn't record cameras yet. Your camera's own app or page can save clips to a shared folder (most cameras can save to an SMB share: use a person's name and password from People)",
"i": "info"
},
{
"t": "Works for",
"p": "hs-files"
},
{
"t": "With a password",
"p": "hs-files",
"d": "They type it when they open the link",
"i": "lock"
},
{
"t": "Make the link",
"p": "hs-files",
"i": "globe"
},
{
"t": "No links yet",
"p": "hs-files",
"d": "Search for a file or folder above and select it to share a link",
"i": "globe"
},
{
"t": "Drop-off folder",
"p": "hs-files",
"i": "folder"
},
{
"t": "Drop-off link",
"p": "hs-files",
"i": "globe"
},
{
"t": "Put back",
"p": "hs-files",
"i": "restart"
},
{
"t": "Share a Games folder",
"p": "hs-gaming",
"d": "Home › Games, read only for your devices: put game installers (GOG, itch.io), mods and emulator files there and copy them to any Hearth device at home-network speed",
"i": "folder"
},
{
"t": "Games folder",
"p": "hs-gaming",
"i": "folder"
},
{
"t": "Steam downloads at home",
"p": "hs-gaming",
"d": "Steam copies games between your PCs and Steam Decks on the same network by itself (Steam › Settings › Downloads › Local Network Game Transfers, on each device). Hearth never signs in to Steam or changes its settings",
"i": "store"
},
{
"t": "Game archive and saves",
"p": "hs-gaming",
"d": "Hearth Storage keeps games for your Hearth devices (download once, copy at home speed) and lets saves follow you",
"i": "desktops"
},
{
"t": "Remote play and Use this PC from anywhere",
"p": "hs-gaming",
"d": "Sunshine on this PC, Moonlight on your laptop, work PC, phone or TV: your games, or your whole desktop. Away from home, add this PC in Moonlight by its Tailscale name",
"i": "wired"
},
{
"t": "Friends over Tailscale",
"p": "hs-gaming",
"d": "To play your home-network games with a friend somewhere else, share this PC with them from Tailscale's admin page (Share node). They only reach this PC, nothing else at home",
"i": "user"
},
{
"t": "Light mode while you play",
"p": "hs-gaming",
"i": "sleep"
},
{
"t": "Add a person",
"p": "hs-people",
"d": "A name (small letters, like sam or mia-k) and their own password",
"i": "plus"
},
{
"t": "Their own folder",
"p": "hs-people",
"i": "folder"
},
{
"t": "Size limit for their folder",
"p": "hs-people",
"d": "On a btrfs drive (your Home on a Hearth install is). Other drives: no limit is possible"
},
{
"t": "With their own name and password",
"p": "hs-people",
"d": "The same way as you (Shared folders › How to open them), with their name instead of yours. Hidden files are never shown, and nobody gets in without a password.",
"i": "display"
},
{
"t": "Make a new topic",
"p": "hs-phone",
"d": "If someone else got the old one. Subscribe to the new one on your phone afterwards",
"i": "restart"
},
{
"t": "Server",
"p": "hs-phone",
"i": "globe"
},
{
"t": "Send notifications to my phone",
"p": "hs-phone",
"i": "bell"
},
{
"t": "Home server alerts",
"p": "hs-phone",
"d": "What Alerts & power finds: a drive failing or very hot, a drive filling up, a backup failing or late, a mirror missing a drive (turn alerts on there too)",
"i": "warn"
},
{
"t": "Server apps that stop",
"p": "hs-phone",
"d": "A game server or app that stopped and couldn't start again",
"i": "server"
},
{
"t": "Send a test",
"p": "hs-phone",
"i": "check"
},
{
"t": "Your shares are read only",
"p": "hs-safety",
"i": "lock"
},
{
"t": "Make them writable again",
"p": "hs-safety",
"d": "It was you (a big move or clean up), or it's dealt with",
"i": "restart"
},
{
"t": "Nothing to fix",
"p": "hs-safety",
"d": "Your shares, services and backups look right",
"i": "check"
},
{
"t": "Last backup",
"p": "hs-safety",
"i": "update"
},
{
"t": "Shared folders: SMB3, no guests",
"p": "hs-safety",
"i": "lock"
},
{
"t": "Snapshots",
"p": "hs-safety",
"i": "restart"
},
{
"t": "Ransomware guard",
"p": "hs-safety",
"i": "lock"
},
{
"t": "Turn shares read only after",
"p": "hs-safety",
"d": "Files renamed or deleted through Samba within 5 minutes"
},
{
"t": "Nothing is shared or served",
"p": "hs-safety",
"i": "check"
},
{
"t": "The previous version stays startable",
"p": "hs-safety",
"d": "Hearth OS updates always keep the version before them in the boot menu. Settings › Updates",
"i": "update"
},
{
"t": "Set up your home server",
"p": "hs-overview",
"d": "A few steps, all with the controller: this PC's name, Tailscale, your first shared folder and your first backup",
"i": "check"
},
{
"t": "Light mode while you play",
"p": "hs-overview",
"d": "Big jobs (backups, snapshots, emptying recycle bins) wait while a game runs. Sharing keeps working.",
"i": "server"
},
{
"t": "Show hidden items",
"p": "hs-overview",
"i": "info"
},
{
"t": "Command Center seen within",
"p": "hs-overview"
},
{
"t": "Media server",
"p": "hs-services",
"i": "film"
},
{
"t": "Media server: reach, pause, open",
"p": "hs-services",
"d": "Who can reach it, pausing for games, opening it, your photo library",
"i": "gear"
},
{
"t": "Reachable",
"p": "hs-services"
},
{
"t": "Pause in Game Mode",
"p": "hs-services",
"d": "Stops the media server while Game Mode or a game runs (with Light mode on), and starts it again after. Off: people can keep watching while you play",
"i": "pad"
},
{
"t": "Photo library",
"p": "hs-services",
"d": "Your photos open in Jellyfin too: in its first steps (or Dashboard › Libraries) add your Photos folder as a \"Photos\" library. Phone photos get here with Backups & sync › Phone photos",
"i": "folder"
},
{
"t": "Remove Jellyfin",
"p": "hs-services",
"d": "The app goes; your libraries and its settings (in /var/lib/hearth/jellyfin) stay",
"i": "trash"
},
{
"t": "Files on the web",
"p": "hs-services",
"i": "globe"
},
{
"t": "Files on the web: password, reach, open",
"p": "hs-services",
"d": "Its web password, who can reach it (also share links and the drop-off)",
"i": "gear"
},
{
"t": "Reachable (also share links and the drop-off)",
"p": "hs-services"
},
{
"t": "Open",
"p": "hs-services",
"i": "open"
},
{
"t": "Waiting for Tailscale",
"p": "hs-services",
"d": "Turn on Tailscale and sign in (Tailscale page), or pick Home network",
"i": "globe"
},
{
"t": "Take over your own ComfyUI",
"p": "hs-services",
"i": "image"
},
{
"t": "AI pictures",
"p": "hs-services",
"i": "image"
},
{
"t": "AI pictures: open, models, while you play",
"p": "hs-services",
"d": "Who can reach it, opening ComfyUI, the models folder, updating and removing it",
"i": "gear"
},
{
"t": "Who can reach it",
"p": "hs-services",
"i": "lock"
},
{
"t": "Open ComfyUI",
"p": "hs-services",
"i": "open"
},
{
"t": "Models folder",
"p": "hs-services",
"i": "folder"
},
{
"t": "Add video (Wan 2.2)",
"p": "hs-services",
"i": "film"
},
{
"t": "Add only picture to video (Wan 2.2)",
"p": "hs-services",
"d": "The same, without text to video: about 25 GB",
"i": "image"
},
{
"t": "Remove video (Wan 2.2)",
"p": "hs-services",
"d": "Only what Hearth downloaded for it goes (and the GGUF loader, if Hearth got it). Your own models, LoRAs and pictures stay",
"i": "trash"
},
{
"t": "Stop it while I play",
"p": "hs-services",
"d": "ComfyUI holds graphics memory: with Light mode on it stops when Game Mode or a game starts and starts again after. Off: it keeps the memory while you play",
"i": "pad"
},
{
"t": "Update ComfyUI",
"p": "hs-services",
"i": "update"
},
{
"t": "Give your own ComfyUI service back",
"p": "hs-services",
"d": "Hearth's turns off and your comfyui.service comes back on, the way it was before",
"i": "back"
},
{
"t": "Remove ComfyUI",
"p": "hs-services",
"d": "ComfyUI and its Python environment go. Your models, pictures (output) and workflows stay in ~/ComfyUI",
"i": "trash"
},
{
"t": "Install Voice Studio",
"p": "hs-services",
"i": "vol"
},
{
"t": "Voice Studio",
"p": "hs-services",
"i": "vol"
},
{
"t": "Open Voice Studio",
"p": "hs-services",
"i": "open"
},
{
"t": "Voice Studio: PIN, apps, while you play",
"p": "hs-services",
"d": "The PIN for its page, the address and key for the Command Center, removing it",
"i": "gear"
},
{
"t": "PIN for Voice Studio",
"p": "hs-services",
"i": "lock"
},
{
"t": "Address for the Command Center",
"p": "hs-services",
"i": "clipboard"
},
{
"t": "API key for apps",
"p": "hs-services",
"d": "The Command Center and other apps send it with each request. Copy it into the speech server's key field",
"i": "lock"
},
{
"t": "Make a new key",
"p": "hs-services",
"d": "The old key stops working: apps using it need the new one",
"i": "restart"
},
{
"t": "Remove Voice Studio",
"p": "hs-services",
"d": "Its engines and downloaded models go (about 14 GB). Your voices, audiobooks, dubs and transcripts stay",
"i": "trash"
},
{
"t": "Take over your own Kokoro",
"p": "hs-services",
"d": "Your \"kokoro\" container is running, and it answers your home network too. Take it over: it's stopped (never deleted; Give it back starts it again) and Hearth runs its own copy, Tailscale only",
"i": "wave"
},
{
"t": "Install AI voices (Kokoro)",
"p": "hs-services",
"i": "wave"
},
{
"t": "AI voices",
"p": "hs-services",
"i": "wave"
},
{
"t": "AI voices: address, while you play",
"p": "hs-services",
"d": "The address for the Command Center, who can reach it, removing it",
"i": "gear"
},
{
"t": "Paste this into the Command Center",
"p": "hs-services",
"i": "clipboard"
},
{
"t": "Give your own Kokoro back",
"p": "hs-services",
"d": "Hearth's turns off and your \"kokoro\" container starts again if it was running",
"i": "back"
},
{
"t": "Remove Kokoro",
"p": "hs-services",
"d": "Hearth's copy and its image go. Your own \"kokoro\" container, if you have one, is never touched",
"i": "trash"
},
{
"t": "AI chat",
"p": "hs-services",
"i": "chat"
},
{
"t": "Remote play",
"p": "hs-services",
"d": "Sunshine: play this PC's games on your laptop, phone or TV with Moonlight, at home or through Tailscale; Use this PC from anywhere is there too",
"i": "play"
},
{
"t": "Folder sync",
"p": "hs-services",
"d": "Syncthing: keep a folder the same on your Hearth devices, like a private Dropbox",
"i": "sync"
},
{
"t": "Game archive",
"p": "hs-services",
"d": "Hearth Storage: move games here to free space on your Hearth devices, and bring them back any time",
"i": "archive"
},
{
"t": "Signed in ✓",
"p": "hs-setup",
"i": "check"
},
{
"t": "Tailscale isn't on this PC yet",
"p": "hs-setup",
"d": "It comes with the Hearth OS update that brings Home server. Skip this step for now.",
"i": "info"
},
{
"t": "Hearth Storage's backups",
"p": "hs-setup",
"d": "Your Hearth devices back up their saves and screenshots to this PC (Hearth Link › Hearth Storage)",
"i": "desktops"
},
{
"t": "Back",
"p": "hs-setup",
"i": "back"
},
{
"t": "Go to Home server",
"p": "hs-setup",
"i": "check"
},
{
"t": "Share a folder",
"p": "hs-shares",
"d": "A folder in your Home or on a drive, or a whole drive. It starts read only, on your home network.",
"i": "plus"
},
{
"t": "Pick it in Hearth Files",
"p": "hs-shares",
"i": "folder"
},
{
"t": "Share your whole Home",
"p": "hs-shares",
"i": "home"
},
{
"t": "Share the system drive (read only)",
"p": "hs-shares",
"i": "shield"
},
{
"t": "Shared",
"p": "hs-shares",
"i": "folder"
},
{
"t": "Access",
"p": "hs-shares"
},
{
"t": "Reachable",
"p": "hs-shares"
},
{
"t": "Recycle bin (30 days)",
"p": "hs-shares",
"i": "trash"
},
{
"t": "Who",
"p": "hs-shares",
"i": "user"
},
{
"t": "Snapshots",
"p": "hs-shares",
"i": "restart"
},
{
"t": "Stop sharing it",
"p": "hs-shares",
"i": "close"
},
{
"t": "Windows",
"p": "hs-shares",
"i": "display"
},
{
"t": "Phone or TV",
"p": "hs-shares",
"i": "pad"
},
{
"t": "Hearth devices",
"p": "hs-shares",
"i": "desktops"
},
{
"t": "Away from home",
"p": "hs-shares",
"i": "globe"
},
{
"t": "Turn on Tailscale",
"p": "hs-tailscale",
"d": "Starts it now. Whether it starts with the PC is the switch below.",
"i": "power"
},
{
"t": "This PC's name",
"p": "hs-tailscale",
"i": "display"
},
{
"t": "Sign out",
"p": "hs-tailscale",
"d": "This PC leaves your tailnet until you sign in again",
"i": "logout"
},
{
"t": "Only reachable through Tailscale",
"p": "hs-tailscale",
"d": "Your shares can't be opened from your home network, only from your devices on Tailscale (at home or away). The firewall lets sharing in only through Tailscale.",
"i": "lock"
},
{
"t": "Start Tailscale when this PC starts",
"p": "hs-tailscale",
"d": "Off: Tailscale runs only after you turn it on here. Either way, starting the PC never waits for it.",
"i": "power"
},
{
"t": "Let other devices use this PC's internet (exit node)",
"p": "hs-tailscale",
"d": "Off unless you turn it on. Your devices can send their internet through this PC (you also allow it in Tailscale's admin page).",
"i": "globe"
},
{
"t": "Receive files (Taildrop)",
"p": "hs-tailscale",
"d": "Files your devices send to this PC land in Downloads › Taildrop. A name that's taken gets a new one.",
"i": "file"
},
{
"t": "Wake this PC from away",
"p": "hs-tailscale",
"d": "Its network card wakes the PC from sleep (Wake-on-LAN). From away, another of your devices at home sends the wake-up",
"i": "sleep"
},
{
"t": "Looking for Windows, SteamOS and other systems…",
"p": "install",
"i": "update"
},
{
"t": "Erase a drive",
"p": "install",
"d": "Hearth gets the whole drive. Everything on it is erased."
},
{
"t": "Looking for drives…",
"p": "install",
"i": "update"
},
{
"t": "Can't continue yet",
"p": "install",
"i": "lock"
},
{
"t": "Check again",
"p": "install",
"i": "restart"
},
{
"t": "Room for Hearth",
"p": "install"
},
{
"t": "No password",
"p": "install",
"i": "lock"
},
{
"t": "Ask for my password",
"p": "install"
},
{
"t": "Go back",
"p": "install",
"i": "back"
},
{
"t": "Restart now",
"p": "install",
"i": "restart"
},
{
"t": "The install didn't finish",
"p": "install",
"i": "info"
},
{
"t": "Try again",
"p": "install",
"i": "restart"
},
{
"t": "Move up",
"p": "keyboard"
},
{
"t": "Move down",
"p": "keyboard"
},
{
"t": "Remove",
"p": "keyboard"
},
{
"t": "Switch layouts with",
"p": "keyboard",
"d": "Press it to go to the next layout"
},
{
"t": "Wait before a held key repeats",
"p": "keyboard",
"i": "kbd"
},
{
"t": "Repeat speed",
"p": "keyboard",
"i": "kbd"
},
{
"t": "On-screen keyboard",
"p": "keyboard",
"d": "For the controller and touch. Opens with Y, the keyboard button on the taskbar, or a swipe in from the left edge",
"i": "kbd"
},
{
"t": "Try the on-screen keyboard",
"p": "keyboard",
"d": "Y on the controller, the keyboard button on the taskbar, or a swipe in from the left edge",
"i": "kbd"
},
{
"t": "Word suggestions",
"p": "keyboard",
"d": "Words to pick above the keys, from a word list on this device"
},
{
"t": "Learn new words",
"p": "keyboard",
"d": "Remembers words you type (never passwords), only on this device"
},
{
"t": "Capital letters",
"p": "keyboard",
"d": "At the start of a sentence"
},
{
"t": "Double space types a period",
"p": "keyboard"
},
{
"t": "Fast typing",
"p": "keyboard",
"d": "Ember Wheel: tilt the left stick at a group of letters, A B X Y type one of its four. Split sticks: each stick points into its half, LT and RT type."
},
{
"t": "Size",
"p": "keyboard"
},
{
"t": "Shape",
"p": "keyboard",
"d": "Split puts the halves at the edges, under your thumbs on a handheld's touchscreen"
},
{
"t": "Position",
"p": "keyboard"
},
{
"t": "Ember trail",
"p": "keyboard",
"d": "A warm glow follows the selection"
},
{
"t": "Rumble on key presses",
"p": "keyboard"
},
{
"t": "Clear learned words",
"p": "keyboard",
"d": "Word suggestions go back to the word list (Game Mode's keyboard forgets them too)"
},
{
"t": "Share this device's AI",
"p": "link",
"d": "Its AI (chat, voices, pictures, speech to text) for your other devices and the Command Center, through Tailscale",
"i": "mic"
},
{
"t": "Pair a device",
"p": "link",
"i": "lock"
},
{
"t": "Not on the same network?",
"p": "link",
"d": "Get a code to type on a device somewhere else (over the internet)",
"i": "globe"
},
{
"t": "Stop pairing",
"p": "link",
"i": "close"
},
{
"t": "Other ways: this device's long link",
"p": "link",
"i": "info"
},
{
"t": "Your devices",
"p": "link",
"d": "Each device's battery, use, temperatures, drives and what's playing, with the last hour",
"i": "cpu"
},
{
"t": "Get a game from another Hearth",
"p": "link",
"d": "Pick the game in your library's Get from another Hearth. Opens in the Hearth app's window for now.",
"i": "store"
},
{
"t": "Keep Hearth data in sync",
"p": "link",
"d": "Favorites, collections, play time, per-game settings, profile, appearance, controller layouts. Never sign-ins.",
"i": "update"
},
{
"t": "Accept games from my devices without asking",
"p": "link",
"i": "store"
},
{
"t": "Let my other devices see this device's status",
"p": "link",
"d": "Battery, use, temperatures, drives and what's playing, only for devices paired with this one. Never the internet.",
"i": "eye"
},
{
"t": "Relay",
"p": "link",
"d": "Only when two devices can't connect directly over the internet; the data stays encrypted end to end"
},
{
"t": "Use this device as Hearth Storage",
"p": "link",
"d": "A home server for your paired Hearth devices: shared folders, games kept for them, their backups and saves. Only paired devices; nothing is opened to the internet.",
"i": "store"
},
{
"t": "Remove it",
"p": "link",
"d": "Unpaired from your devices. What Hearth Storage keeps of it stays. It can pair again any time.",
"i": "trash"
},
{
"t": "Remove it and delete its backups and archived games",
"p": "link",
"d": "Also frees their space on Hearth Storage. This can't be undone.",
"i": "trash"
},
{
"t": "Share a folder",
"p": "link",
"d": "Type its full path (in Home or on a drive). Hidden files are never shared.",
"i": "store"
},
{
"t": "Back up now",
"p": "link",
"i": "update"
},
{
"t": "Saves follow you",
"p": "link",
"d": "Games without their store's cloud saves get their newest save from Hearth Storage before they start, and send it back after. If both changed, you're asked.",
"i": "update"
},
{
"t": "Keep backups and saves for my devices",
"p": "link",
"i": "store"
},
{
"t": "Away from home",
"p": "link",
"d": "Your devices can use it when they aren't home, over Hearth Link's encrypted connection. No port is opened.",
"i": "update"
},
{
"t": "Windows sharing (SMB)",
"p": "link",
"d": "The shared folders for Windows PCs on this network (port 445, this network only). Its password, media and downloads: Hearth Storage's extras, below.",
"i": "display"
},
{
"t": "Hearth Storage's extras",
"p": "link",
"d": "The Windows sharing password, browsing the game archive, media and downloads. Opens in the Hearth app's window for now.",
"i": "open"
},
{
"t": "Pointer speed",
"p": "mouse",
"i": "mouse"
},
{
"t": "Mouse acceleration",
"p": "mouse",
"d": "The pointer goes further when you move the mouse quickly. Off is steadier for games",
"i": "mouse"
},
{
"t": "Natural scrolling",
"p": "mouse",
"d": "The page moves the way the wheel turns, like on a phone",
"i": "mouse"
},
{
"t": "Left-handed",
"p": "mouse",
"d": "Swaps the left and right buttons",
"i": "mouse"
},
{
"t": "Touchpad speed",
"p": "mouse",
"i": "mouse"
},
{
"t": "Tap to click",
"p": "mouse",
"d": "Tap with one finger to click, two fingers to right-click",
"i": "mouse"
},
{
"t": "Scroll with",
"p": "mouse",
"i": "mouse"
},
{
"t": "Ignore the touchpad while typing",
"p": "mouse",
"i": "kbd"
},
{
"t": "Do not disturb",
"p": "notifications",
"d": "Notifications go straight to the list without popping up (urgent ones still do). Right-click the bell on the taskbar to switch it, or press Super+N for the list",
"i": "bell-off"
},
{
"t": "Clear the notification list",
"p": "notifications",
"i": "bell"
},
{
"t": "Battery",
"p": "power"
},
{
"t": "Power mode",
"p": "power",
"d": "Battery saver lasts longer; Performance is faster and warmer",
"i": "bolt"
},
{
"t": "Power limit (TDP)",
"p": "power",
"i": "power"
},
{
"t": "Turn the screen off after",
"p": "power",
"i": "display"
},
{
"t": "Go to sleep after",
"p": "power",
"d": "Downloads stop while it sleeps. Videos and games that are playing keep it awake",
"i": "sleep"
},
{
"t": "Press",
"p": "power",
"d": "A quick press",
"i": "power"
},
{
"t": "Hold",
"p": "power",
"d": "Held down for a second",
"i": "power"
},
{
"t": "Game Mode",
"p": "power",
"d": "The Game Mode you picked in Gaming (Steam's or Hearth's) on the full screen",
"i": "pad"
},
{
"t": "Sleep",
"p": "power",
"i": "sleep"
},
{
"t": "Restart",
"p": "power",
"i": "restart"
},
{
"t": "Shut down",
"p": "power",
"i": "power"
},
{
"t": "The printing service isn't running",
"p": "printers",
"d": "Hearth uses CUPS for printing. It starts by itself when something prints; if this stays, restart the device.",
"i": "info"
},
{
"t": "Make it the default",
"p": "printers",
"i": "check"
},
{
"t": "Remove this printer…",
"p": "printers",
"i": "trash"
},
{
"t": "Cancel",
"p": "printers"
},
{
"t": "More printer settings",
"p": "printers",
"d": "Drivers, paper and other options (Print Settings)",
"i": "gear"
},
{
"t": "Use this PC from anywhere",
"p": "remote",
"i": "globe"
},
{
"t": "Start again",
"p": "remote",
"d": "Sunshine was stopped (Remote play's switch below, or it couldn't start). Start it with Hearth's settings again",
"i": "restart"
},
{
"t": "Tailscale isn't connected",
"p": "remote",
"d": "Away from home, Moonlight finds this PC through Tailscale. Sign in on this PC and on the other device",
"i": "globe"
},
{
"t": "Type Moonlight's PIN",
"p": "remote",
"i": "pad"
},
{
"t": "Sunshine has its own login",
"p": "remote",
"d": "Type it once under Sunshine's username and password below, then pair here",
"i": "lock"
},
{
"t": "Check the connection",
"p": "remote",
"i": "search"
},
{
"t": "Reachable",
"p": "remote",
"d": "Never from the internet. Tailscale only keeps this PC's streaming closed to your home network too"
},
{
"t": "Stay awake while connected",
"p": "remote",
"d": "This PC doesn't sleep or go idle while a device streams, and can again 2 minutes after it disconnects",
"i": "sleep"
},
{
"t": "Wake this PC from away",
"p": "remote",
"d": "If it's asleep, another device at home (a phone or a Hearth device on Tailscale) can wake it: Home server › Alerts & power",
"i": "power"
},
{
"t": "Remote play",
"p": "remote",
"i": "display"
},
{
"t": "Sunshine's username and password",
"p": "remote",
"d": "Sunshine was set up before with its own login. Type it once so Hearth can pair devices.",
"i": "lock"
},
{
"t": "Pair a device",
"p": "remote",
"i": "pad"
},
{
"t": "Moonlight",
"p": "remote",
"i": "desktops"
},
{
"t": "Use Server mode",
"p": "hs-server",
"d": "Show it next to Game Mode and Desktop Mode: in Start in, here and on the dashboard. Off by default; Game Mode stays where your PC starts unless you choose Server mode",
"i": "server"
},
{
"t": "Open Server mode now",
"p": "hs-server",
"i": "play"
},
{
"t": "Startup and schedule",
"p": "hs-server",
"i": "power"
},
{
"t": "Lower power limits when idle",
"p": "hs-server",
"i": "bolt"
},
{
"t": "Graphics card limit when idle",
"p": "hs-server"
},
{
"t": "Idle means no jobs for",
"p": "hs-server"
},
{
"t": "Spin down drives",
"p": "hs-server",
"d": "Data drives only; the system drive stays on (the same setting as Alerts & power)",
"i": "drive"
},
{
"t": "Screen dims to a clock after",
"p": "hs-server",
"d": "Dim shows the time and the status in dark grey, kind to OLED screens. Any button brings the dashboard back",
"i": "display"
},
{
"t": "Then off after",
"p": "hs-server"
},
{
"t": "Sleep",
"p": "hs-server",
"d": "Server mode keeps the PC awake unless you choose a time here. Waking it over Tailscale needs another device at home",
"i": "sleep"
},
{
"t": "Light mode while you play",
"p": "hs-server",
"d": "The same rules for Server mode's Play button, Guide and any game. Backups wait while it's on",
"i": "pad"
},
{
"t": "Pictures · ComfyUI",
"p": "hs-server",
"i": "image"
},
{
"t": "AI chat · Ollama",
"p": "hs-server",
"i": "chat"
},
{
"t": "Voices · Kokoro",
"p": "hs-server",
"i": "wave"
},
{
"t": "Media · Jellyfin",
"p": "hs-server",
"i": "film"
},
{
"t": "Shared folders, Tailscale",
"p": "hs-server",
"d": "Always keep working",
"i": "folder"
},
{
"t": "Dashboard web page",
"p": "hs-server",
"i": "globe"
},
{
"t": "PIN for the web page",
"p": "hs-server",
"i": "lock"
},
{
"t": "Restart and pause services",
"p": "hs-server",
"i": "restart"
},
{
"t": "Switch modes",
"p": "hs-server",
"d": "The screen at home counts down 10 seconds first (cancel it there), and nothing switches while a game is running or Desktop Mode is open",
"i": "pad"
},
{
"t": "Sleep or restart the PC",
"p": "hs-server",
"i": "sleep"
},
{
"t": "Show it on my other Hearth devices",
"p": "hs-server",
"d": "Hearth Link and Quick Access on your Ally can show the status (read only) without the PIN, from your Tailscale devices only",
"i": "desktops"
},
{
"t": "Sign in automatically",
"p": "signin",
"i": "user"
},
{
"t": "Ask for my password after sleep",
"p": "signin",
"d": "Locked when it wakes, on the desktop and in Game Mode. Lock any time with Super+L or the power menu.",
"i": "lock"
},
{
"t": "Start in Game Mode or Desktop Mode",
"p": "signin",
"d": "Now in Gaming › Start in",
"i": "power"
},
{
"t": "Device name",
"p": "signin",
"i": "display"
},
{
"t": "Volume",
"p": "sound"
},
{
"t": "Mute",
"p": "sound",
"i": "vol-mute"
},
{
"t": "Over-amplification",
"p": "sound",
"d": "Lets the volume go up to 150%. Sound can distort, and loud sound can damage speakers or your hearing.",
"i": "vol"
},
{
"t": "Input volume",
"p": "sound",
"i": "mic"
},
{
"t": "When this PC turns on",
"p": "startup",
"d": "Start in one mode every time, or follow a schedule",
"i": "power"
},
{
"t": "Add a change",
"p": "startup",
"d": "A new change an hour after the last one",
"i": "plus"
},
{
"t": "If a game is running",
"p": "startup",
"d": "Server mode never closes your game: the change waits until it ends",
"i": "pad"
},
{
"t": "If you're in Desktop Mode",
"p": "startup",
"d": "Unsaved work stays open: the schedule never switches away from Desktop Mode",
"i": "desktops"
},
{
"t": "In Server mode",
"p": "startup",
"d": "The dashboard counts down 10 seconds first; B cancels",
"i": "server"
},
{
"t": "Switch to Steam Game Mode",
"p": "steam",
"d": "Steam's own gamepad screens, like SteamOS: where the device starts. Back here: Steam's Power › Switch to Desktop.",
"i": "pad"
},
{
"t": "Decky Loader and Hearth Sync",
"p": "steam",
"i": "store"
},
{
"t": "Install Decky Loader",
"p": "steam",
"d": "Downloads Decky Loader from its GitHub releases and adds Hearth Sync. Decky runs as root while Steam Game Mode runs, and Steam's remote debugging is turned on (any program on this device can then control Steam).",
"i": "check"
},
{
"t": "Move games and add up sizes",
"p": "storage",
"d": "Moves a game to another drive, or adds up every game's size again. Opens in the Hearth app's window for now.",
"i": "open"
},
{
"t": "Yes, delete them",
"p": "storage",
"i": "check"
},
{
"t": "Warn when a games drive is low on space",
"p": "storage",
"d": "Before an install and in notifications"
},
{
"t": "Low means less than",
"p": "storage"
},
{
"t": "Update anyway",
"p": "updates",
"d": "Downloads anything new for Hearth and its apps, even small fixes",
"i": "update"
},
{
"t": "Decky plugins",
"p": "updates",
"d": "Hearth's plugins for Steam Game Mode, updated without a system update",
"i": "box"
},
{
"t": "Also install new Hearth plugins",
"p": "updates",
"d": "Update (and automatic updates, when on) also installs new plugins from Hearth, checked the same way. Off: they're only listed",
"i": "update"
},
{
"t": "Check now",
"p": "updates",
"i": "restart"
},
{
"t": "Update plugins automatically",
"p": "updates",
"d": "Checks once a day while online; a new plugin is put in place at the next Steam Game Mode start, or right away when no game is running",
"i": "update"
},
{
"t": "Use the built-in plugins",
"p": "updates",
"d": "The plugins that came with this Hearth OS version come back (those downloads aren't fetched again by themselves)",
"i": "restart"
},
{
"t": "Yes, go back at the next restart",
"p": "updates",
"i": "check"
},
{
"t": "The previous version always stays startable",
"p": "updates",
"d": "Every update keeps the version before it in the boot menu",
"i": "shield"
},
{
"t": "OK, hide this",
"p": "updates",
"i": "check"
},
{
"t": "Unlocked system",
"p": "updates",
"i": "lock"
},
{
"t": "Turn my changes back on",
"p": "updates",
"d": "Uses the packages you added again, now. If Hearth then doesn't start, it starts without them again by itself.",
"i": "restart"
},
{
"t": "Your packages",
"p": "updates",
"i": "update"
},
{
"t": "Put my packages back again",
"p": "updates",
"d": "Reinstalls what you added on this version, in the background",
"i": "restart"
},
{
"t": "Remove my picture",
"p": "users",
"i": "close"
},
{
"t": "Remove this account…",
"p": "users",
"i": "trash"
},
{
"t": "Remove and delete their files",
"p": "users",
"d": "Their home folder, games and saves are deleted for good"
},
{
"t": "Cancel",
"p": "users"
},
{
"t": "Account type",
"p": "users",
"d": "Administrators can change system settings, install system updates and manage accounts"
},
{
"t": "Wi-Fi",
"p": "wifi",
"i": "wifi"
},
{
"t": "Connect",
"p": "wifi"
},
{
"t": "Add a hidden network",
"p": "wifi",
"d": "A network that doesn't show its name: type it and its password",
"i": "plus"
},
{
"t": "Look again",
"p": "wifi",
"i": "restart"
},
{
"t": "Proton built into Hearth OS",
"p": "winapps",
"i": "window"
},
{
"t": "Open a Windows program",
"p": "winapps",
"d": "Open a .exe or .msi in Files (or right-click it › Open with Hearth). An installer gets its own space and its app goes in your app menu.",
"i": "folder"
},
{
"t": "Open",
"p": "winapps",
"i": "play"
},
{
"t": "Show log",
"p": "winapps",
"i": "info"
},
{
"t": "Detailed Proton log for the next start",
"p": "winapps"
},
{
"t": "Show the detailed Proton log",
"p": "winapps",
"i": "info"
},
{
"t": "Proton",
"p": "winapps"
},
{
"t": "DXVK",
"p": "winapps",
"d": "DirectX 9, 10 and 11 through Vulkan. Off: Wine's own, slower, for programs that draw wrongly."
},
{
"t": "VKD3D",
"p": "winapps",
"d": "DirectX 12 through Vulkan. Off: the program falls back to DirectX 11 if it can."
},
{
"t": "Windows version",
"p": "winapps"
},
{
"t": "In Steam",
"p": "winapps",
"d": "Hearth Sync adds it to Steam's library (Game Mode too) the next time it syncs"
},
{
"t": "Remove…",
"p": "winapps",
"d": "Deletes the app, its menu entry and everything installed in its space",
"i": "trash"
},
{
"t": "Cancel",
"p": "winapps"
},
{
"t": "Not now",
"p": "winapps"
},
{
"t": "Clean up",
"p": "home",
"d": "Old shader caches and finished downloads",
"i": "trash"
}
];
