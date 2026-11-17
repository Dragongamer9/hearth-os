.pragma library
// Hearth Settings' page registry (docs/MANTEL.md, "Settings"): the 12 categories plus Home, and every page with its one
// home. Adding a page is one entry in PAGES:
//   { id: "hs-thing", file: "SetHomeThing.qml", cat: "hs", label: "Thing", icon: "grid", desc: "One line for its card",
//     words: "other words people search for", old: "Its old name" }
// Optional: parent (a page it sits under: breadcrumbs and Back), props (set on the page once loaded, e.g. a part of a page
// shown on its own), hidden (not listed on its category's page: opened from another page), live (only on the live USB),
// status (a live value on its card: SetCategory.qml knows these), link (a card that opens another page's one real home).
// Old page ids keep working: every page id from before the redesign is still an id here (deep links, notifications, the
// launcher's search, `hearth-settings <page>`). tools/build-settings-index.py reads this file and every page's cards to
// make settings-index.js (the search); hosts/linux/test/test_settings_redesign.py checks both.

var CATEGORIES = [
    { id: "home", icon: "home", label: "Home", landing: "home" },
    { id: "system", icon: "display", label: "System", landing: "system" },
    { id: "net", icon: "wifi", label: "Network & internet", landing: "wifi" },
    { id: "bt", icon: "bt", label: "Bluetooth & devices", landing: "bt" },
    { id: "link", icon: "desktops", label: "Hearth Link", landing: "link" },
    { id: "pers", icon: "palette", label: "Personalization", landing: "appearance" },
    { id: "apps", icon: "store", label: "Apps", landing: "apps-home" },
    { id: "gaming", icon: "pad", label: "Gaming", landing: "gaming" },
    { id: "accounts", icon: "user", label: "Accounts", landing: "accounts" },
    { id: "hs", icon: "server", label: "Home server", landing: "hs-overview" },
    { id: "privacy", icon: "shield", label: "Privacy & security", landing: "privacy" },
    { id: "a11y", icon: "access", label: "Accessibility", landing: "accessibility" },
    { id: "updates", icon: "update", label: "Updates & recovery", landing: "updates" }
];

var PAGES = [
    // ---- Home
    { id: "home", file: "SetHome.qml", cat: "home", label: "Home", icon: "home", words: "device name rename status recommended start", landing: true },
    { id: "quick", file: "SetQuick.qml", cat: "home", parent: "home", label: "Quick", icon: "bolt", desc: "Big tiles for the settings you change most; X picks which", words: "quick tiles pin brightness volume wifi" },
    { id: "changes", file: "SetChangesPage.qml", cat: "home", parent: "home", label: "All changes", icon: "undo", desc: "What was changed in Settings, with Undo", words: "history undo changed lately recent" },

    // ---- System
    { id: "system", file: "SetCategory.qml", cat: "system", label: "System", icon: "display", landing: true },
    { id: "install", file: "SetInstall.qml", cat: "system", label: "Install Hearth", icon: "update", desc: "Put Hearth OS on this PC, next to Windows or on its own", words: "install live usb windows dual boot", old: "Install Hearth", live: true },
    { id: "display", file: "SetDisplay.qml", cat: "system", label: "Display", icon: "display", desc: "Brightness, night light, scale, resolution and refresh rate", words: "screen resolution refresh scale monitor brightness night light", old: "Display", status: "display" },
    { id: "sound", file: "SetSound.qml", cat: "system", label: "Sound", icon: "vol", desc: "Volume, speakers and headphones, microphone", words: "volume audio speakers output microphone", old: "Sound" },
    // 1.14, more of KDE's Sound (docs/SETTINGS-MORE-1.14.md)
    { id: "sound-speakers", file: "SetSoundSpeakers.qml", cat: "system", parent: "sound", label: "Speakers & balance", icon: "vol", desc: "Left and right balance, and a test tone from each speaker", words: "balance left right speaker test surround 5.1 channels tone" },
    { id: "sound-apps", file: "SetSoundApps.qml", cat: "system", parent: "sound", label: "Apps' sound", icon: "headphones", desc: "Which speakers or headphones each app plays on, which microphone each app records from, and scrolling on the dock", words: "per app output device move stream play on dock scroll volume microphone mic input record recording per app input" },
    { id: "push-to-talk", file: "SetPushToTalk.qml", cat: "system", parent: "sound", label: "Push to talk", icon: "mic", desc: "The microphone stays muted unless you hold a key", words: "push to talk ptt microphone mute hold key discord voice" },
    { id: "notifications", file: "SetNotifications.qml", cat: "system", label: "Notifications", icon: "bell", desc: "Do not disturb, quiet in full screen and on a schedule, each app, the history", words: "do not disturb mute alerts full screen fullscreen screen sharing schedule quiet hours history keep after restart", old: "Notifications", status: "dnd" },
    { id: "sounds", file: "SetSounds.qml", cat: "system", parent: "sound", label: "Sounds", icon: "bell", desc: "Hearth's own short sounds or the freedesktop set, and which events play one", words: "system sounds sound theme event sounds notification sound beep click volume feedback device plugged battery low freedesktop" },
    { id: "power", file: "SetPower.qml", cat: "system", label: "Power & battery", icon: "power", desc: "Screen off and sleep times, the power button, battery", words: "battery sleep screen off suspend power button hibernate hybrid sleep", old: "Power" },
    { id: "power-profiles", file: "SetPowerProfiles.qml", cat: "system", parent: "power", label: "Plugged in, battery & lid", icon: "batt", desc: "Different times and power mode on battery, and what closing the lid does", words: "ac battery profile lid close laptop docked on battery plugged in power mode" },
    { id: "power-awake", file: "SetPowerAwake.qml", cat: "system", parent: "power", label: "Apps keeping it awake", icon: "sleep", desc: "What's holding the screen on or the PC awake right now", words: "inhibit inhibitors blocking sleep awake screen on caffeine" },
    { id: "battery", file: "SetBattery.qml", cat: "system", parent: "power", label: "Battery health & history", icon: "bolt", desc: "Charge cycles, how much it holds, graphs, your devices' batteries", words: "battery health cycles cycle count capacity wear history graph energy bluetooth mouse controller headphones battery" },
    { id: "memory", file: "SetMemory.qml", cat: "system", label: "Memory", icon: "cpu", desc: "Compressed memory (zram), keep memory free, the swap file, what closes when memory runs out", words: "ram memory swap zram swappiness swap file pagefile virtual memory oom out of memory freeze compress keep free" },
    { id: "storage", file: "SetStorage.qml", cat: "system", label: "Storage", icon: "drive", desc: "Space on your drives, games by size, clean up", words: "disk space clean games by size move games low space", old: "Storage" },
    { id: "drives", file: "SetDrives.qml", cat: "system", label: "Drives & folders", icon: "drive", desc: "Hearth (C:) with Windows folder names, the address bar, Copy path, drive letters", words: "c: drive letter program files users windows appdata linux folders address bar copy path hearth files explorer" },
    { id: "your-desktop", file: "SetYourDesktop.qml", cat: "system", label: "Your desktop", icon: "desktops", desc: "Mantel, the desktop Desktop Mode opens, and its window manager", words: "desktop mantel hearth classic kwin labwc window manager kde plasma gnome retired safe start", old: "Desktop" },
    { id: "startup", file: "SetStartup.qml", cat: "system", label: "Startup", icon: "power", desc: "Start in Game Mode, Desktop Mode or Server mode, and an optional schedule", words: "start in boot mode schedule server mode game mode desktop mode turn on" },
    { id: "boot", file: "SetBoot.qml", cat: "system", label: "Boot splash", icon: "sparkle", desc: "The boot splash Hearth OS shows while it starts", words: "boot splash startup plymouth", old: "Boot & startup", props: { part: "splash" } },
    { id: "remote-desktop", file: "SetRemoteDesktop.qml", cat: "system", label: "Remote desktop", icon: "desktops", desc: "This PC's desktop in a VNC app on your other devices, through Tailscale", words: "vnc remote desktop control screen share tigervnc remmina krdc krfb tailscale view only password" },
    { id: "datetime", file: "SetDateTime.qml", cat: "system", label: "Date & time", icon: "clock", desc: "Time zone, 24-hour clock", words: "clock time zone 24-hour", old: "Date & time" },
    { id: "region", file: "SetRegion.qml", cat: "system", label: "Region & language", icon: "globe", desc: "Language, and how dates, numbers, money and measurements are written", words: "language locale formats region country numbers currency money date format measurement metric imperial paper a4 letter translation" },
    { id: "languages", file: "SetLanguages.qml", cat: "system", parent: "region", label: "Languages in order", icon: "globe", desc: "More than one language: apps try them one after another", words: "languages several order fallback LANGUAGE translations" },
    { id: "spellcheck", file: "SetSpellcheck.qml", cat: "system", parent: "region", label: "Spell check", icon: "check", desc: "Dictionary, checking as you type, words to leave alone", words: "spell check spelling dictionary hunspell sonnet typo ignore words" },
    { id: "input-methods", file: "SetInputMethods.qml", cat: "system", parent: "region", label: "Chinese, Japanese & Korean typing", icon: "kbd", desc: "Input methods (fcitx5), off unless you turn them on", words: "fcitx fcitx5 input method ime cjk chinese japanese korean pinyin mozc hangul" },
    { id: "clipboard", file: "SetClipboard.qml", cat: "system", label: "Clipboard", icon: "clipboard", desc: "Clipboard history (Super+V)", words: "copy paste history", old: "Clipboard" },
    { id: "search", file: "SetSearch.qml", cat: "system", label: "Search", icon: "search", desc: "The file index, searching inside documents, and what the app menus' search finds", words: "file index indexing baloo krunner find files contents documents calculator units currency web shortcuts emoji commands" },
    { id: "about", file: "SetAbout.qml", cat: "system", label: "About", icon: "info", desc: "Version, this PC's parts and name", words: "system version hardware name", old: "About", props: { part: "about" } },
    // Info Center (1.14, KDE Info Center's technical pages): one file, a page per part.
    { id: "info-cpu", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center", icon: "cpu", desc: "Processor, graphics, memory, Vulkan and OpenGL, PCI and USB devices", words: "info center system information cpu processor cores threads cache hardware specs", props: { part: "cpu" } },
    { id: "info-gpu", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center · Graphics card", icon: "display", desc: "Graphics cards, their drivers and memory", words: "gpu graphics card vram driver nvidia amd intel", props: { part: "gpu" }, hidden: true },
    { id: "info-memory", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center · Memory", icon: "cpu", desc: "Memory and swap", words: "ram memory swap zram meminfo", props: { part: "memory" }, hidden: true },
    { id: "info-graphics", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center · Vulkan & OpenGL", icon: "display", desc: "Vulkan and OpenGL versions and drivers", words: "vulkan opengl egl glx mesa renderer wayland", props: { part: "graphics" }, hidden: true },
    { id: "info-pci", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center · PCI", icon: "cpu", desc: "Every PCI device and its driver", words: "pci lspci devices drivers kernel modules", props: { part: "pci" }, hidden: true },
    { id: "info-usb", file: "SetInfoCenter.qml", cat: "system", parent: "about", label: "Info Center · USB", icon: "link", desc: "Every USB device", words: "usb lsusb devices", props: { part: "usb" }, hidden: true },
    { id: "info-energy", file: "SetBattery.qml", cat: "system", parent: "about", label: "Info Center · Energy", icon: "batt", desc: "The battery's health and history", words: "energy battery upower", hidden: true },

    // ---- Network & internet (Wi-Fi is the landing, as in the mockup; Tailscale follows it under "Reach your devices from anywhere")
    { id: "wifi", file: "SetWifi.qml", cat: "net", label: "Wi-Fi", icon: "wifi", desc: "Connect to wireless networks", words: "wireless network internet hotspot", old: "Wi-Fi", status: "wifi", landing: true },
    { id: "hs-tailscale", file: "SetHomeTailscale.qml", cat: "net", label: "Tailscale", icon: "tailscale", desc: "Reach your devices from anywhere, privately", words: "vpn remote away taildrop exit node wake", old: "Home server › Tailscale", status: "tailscale" },
    { id: "vpn", file: "SetVpn.qml", cat: "net", label: "VPN", icon: "shield", desc: "WireGuard and OpenVPN: add one from its file, connect", words: "vpn wireguard openvpn ovpn conf tunnel private network" },
    { id: "hotspot", file: "SetHotspot.qml", cat: "net", label: "Mobile hotspot", icon: "share", desc: "Share this PC's internet with your other devices over Wi-Fi", words: "hotspot tethering share internet access point ap" },
    { id: "proxy", file: "SetProxy.qml", cat: "net", label: "Proxy", icon: "globe", desc: "A proxy for apps and browsers, by hand or from a setup script", words: "proxy http https socks pac wpad automatic manual" },
    { id: "net-details", file: "SetNetDetails.qml", cat: "net", label: "Connection details", icon: "gear", desc: "DNS, metered, random MAC address, IP address and auto-connect for each network", words: "dns metered mac address random privacy static ip manual ipv4 ipv6 gateway ethernet wired properties advanced" },

    // ---- Bluetooth & devices
    { id: "bt", file: "SetCategory.qml", cat: "bt", label: "Bluetooth & devices", icon: "bt", landing: true },
    { id: "bluetooth", file: "SetBluetooth.qml", cat: "bt", label: "Bluetooth", icon: "bt", desc: "Headphones, controllers and other wireless devices", words: "headphones headset speaker pair devices", old: "Bluetooth", status: "bt" },
    { id: "bluetooth-devices", file: "SetBtDevices.qml", cat: "bt", parent: "bluetooth", label: "Your Bluetooth devices", icon: "bt", desc: "Trust, block, batteries and sending files", words: "bluetooth trust block send file obex battery paired devices" },
    { id: "keyboard", file: "SetKeyboard.qml", cat: "bt", label: "Keyboard", icon: "kbd", desc: "Layouts, typing, the on-screen keyboard, shortcuts", words: "layout on-screen osk typing shortcuts", old: "Keyboard" },
    { id: "mouse", file: "SetMouse.qml", cat: "bt", label: "Mouse & touchpad", icon: "mouse", desc: "Pointer speed, scrolling, tapping", words: "touchpad pointer cursor speed", old: "Mouse & touchpad" },
    { id: "mouse-buttons", file: "SetMouseButtons.qml", cat: "bt", parent: "mouse", label: "Mouse buttons", icon: "mouse", desc: "What your mouse's side and extra buttons do: press one to change it", words: "mouse buttons extra side back forward thumb remap rebind map key shortcut middle button gaming mouse" },
    // ---- Phone link (Hearth Connect, KDE Connect's protocol: docs/PHONE-LINK.md)
    { id: "phone", file: "SetPhone.qml", cat: "bt", label: "Phone", icon: "phone", desc: "Your phone linked with the KDE Connect app: notifications, messages, files, calls", words: "kde connect kdeconnect android iphone mobile sms text messages notifications reply clipboard share send files ring find my phone touchpad remote input presentation remote calls mute ringer run commands battery browse phone files sftp contacts photo" },
    { id: "phone-messages", file: "SetPhoneMessages.qml", cat: "bt", parent: "phone", label: "Messages", icon: "chat", desc: "Your phone's text messages: read and send", words: "sms text messages conversations send a text", hidden: true },
    // ---- end Phone link
    { id: "kwin-tablet", file: "SetKwinTablet.qml", cat: "bt", label: "Drawing tablet & touchscreen", icon: "edit", desc: "Pen tablet's screen and area, pressure, pen buttons; touchscreen screen and calibration", words: "kwin tablet wacom pen stylus pressure buttons touchscreen calibrate calibration map screen" },
    { id: "printers", file: "SetPrinters.qml", cat: "bt", label: "Printers", icon: "printer", desc: "Printers and scanners", words: "print scanner", old: "Printers" },
    { id: "print-queue", file: "SetPrintQueue.qml", cat: "bt", parent: "printers", label: "Print queue", icon: "list", desc: "What's waiting to print: hold, go on or cancel", words: "print queue jobs cancel printing waiting cups" },
    { id: "scanners", file: "SetScanners.qml", cat: "bt", parent: "printers", label: "Scanners", icon: "printer", desc: "The scanners found, and a test scan", words: "scanner scan sane airscan test" },
    { id: "usb-devices", file: "SetUsbDevices.qml", cat: "bt", label: "USB devices & drives", icon: "drive", desc: "What happens when a drive is plugged in, new USB devices, USBGuard", words: "usb removable drive memory card sd plug in automount autoplay usbguard new device allow block" },
    { id: "thunderbolt", file: "SetThunderbolt.qml", cat: "bt", label: "Thunderbolt", icon: "link", desc: "Docks, drives and screens on Thunderbolt or USB4: allow them, or forget them", words: "thunderbolt usb4 dock egpu bolt boltctl authorize allow device security" },

    // ---- Hearth Link (its own category; the page is the landing)
    { id: "link", file: "SetLink.qml", cat: "link", label: "Hearth Link", icon: "desktops", words: "phone devices sync pair storage backups saves", old: "Hearth Link", landing: true },
    { id: "devices", file: "SetDevices.qml", cat: "link", label: "Your devices", icon: "cpu", desc: "Your other devices' battery, use, temperatures, drives and what's playing", words: "devices status battery ram memory cpu gpu temperature vram storage uptime other pc handheld laptop monitor" },

    // ---- Personalization
    { id: "appearance", file: "SetAppearance.qml", cat: "pers", label: "Personalization", icon: "palette", words: "look theme accent colour color wallpaper dark light mode sunset transparency clock", old: "Appearance", landing: true },
    { id: "bar-dock", file: "SetBarDock.qml", cat: "pers", label: "Bar and dock", icon: "window", desc: "Where the bar goes, the dock, auto-hide, the app menu and the right-click menu", words: "taskbar panel dock autohide islands bar layout edge app menu start tiles classic centred right-click context menu windows 11 compact", status: "bar" },
    { id: "fonts", file: "SetFonts.qml", cat: "pers", label: "Fonts, cursor & icons", icon: "doc", desc: "The font and its size, font smoothing, the mouse cursor, the icon theme", words: "font fonts size monospace fixed width terminal antialiasing smoothing hinting dpi cursor pointer size icon theme icons qt gtk subpixel sub-pixel order rgb bgr lcd" },
    { id: "wallpaper-more", file: "SetWallpaperMore.qml", cat: "pers", label: "Wallpaper per screen, slideshow & picture of the day", icon: "image", desc: "A picture for each screen, a slideshow from a folder, the picture of the day, and a light and a dark wallpaper", words: "wallpaper per screen monitor slideshow folder shuffle change every picture of the day bing nasa apod astronomy wikimedia commons daily light dark pair" },
    { id: "my-looks", file: "SetMyLooks.qml", cat: "pers", label: "My looks", icon: "palette", desc: "Save how it all looks now, and put it back in one press", words: "save as my look global theme preset save look restore look" },
    { id: "look-more", file: "SetLookMore.qml", cat: "pers", label: "Window frames & sign-in splash", icon: "window", desc: "The accent in window frames, whether apps follow Hearth's colours, and a splash after you sign in", words: "frame colour intensity title bar tint accent window borders splash screen sign in login splash apps follow hearth qt gtk kde colours colors non-qt applications" },
    // Mantel on KWin (1.14, docs/KWIN-SETTINGS.md): KWin's own settings, Hearth's pages (greyed under labwc)
    { id: "kwin-effects", file: "SetKwinEffects.qml", cat: "pers", label: "Window effects", icon: "sparkle", desc: "Animation speed, overview, Alt+Tab, blur, wobbly windows, minimize and open effects, dimming", words: "kwin effects animation speed overview present windows desktop grid alt tab switcher blur translucency transparent wobbly magic lamp squash fade glide scale dim inactive shake cursor mouse click track" },
    { id: "kwin-desktops", file: "SetKwinDesktops.qml", cat: "pers", label: "Virtual desktops", icon: "desktops", desc: "How many desktops, their names, rows, switching animation", words: "kwin virtual desktops workspaces rows names wrap slide switch" },
    { id: "kwin-tiling", file: "SetKwinTiling.qml", cat: "pers", label: "Tiling", icon: "grid", desc: "Each screen's tile layout, dragging windows to edges, snapping", words: "kwin tiling tiles layout columns quick tile snap edges corners maximize" },
    { id: "kwin-edges", file: "SetKwinEdges.qml", cat: "pers", label: "Screen edges", icon: "display", desc: "Hot corners, edges, touchscreen swipes, sticky edges between screens", words: "kwin hot corners screen edges electric borders touch swipe edge barrier" },
    { id: "kwin-behaviour", file: "SetKwinWindows.qml", cat: "pers", label: "Window behaviour", icon: "window", desc: "Which window gets the keyboard, raise on hover, title bar clicks, Super+drag, where new windows open", words: "kwin window behaviour behavior focus policy follows mouse under mouse click to focus raise hover auto raise click raises focus stealing prevention title bar double click middle click wheel super alt drag move resize placement new windows centered smart" },

    // ---- Apps
    { id: "apps-home", file: "SetCategory.qml", cat: "apps", label: "Apps", icon: "store", landing: true },
    { id: "apps", file: "SetApps.qml", cat: "apps", label: "Installed apps", icon: "store", desc: "Apps from Debian packages, Flathub and AppImages; uninstall", words: "install uninstall packages deb flatpak appimage", old: "Apps" },
    { id: "store", file: "SetStore.qml", cat: "apps", label: "App store", icon: "store", desc: "Find and install apps from Flathub, updates and sources", words: "discover flathub flatpak install get apps software center store updates sources remotes reviews ratings screenshots" },
    { id: "winapps", file: "SetWinApps.qml", cat: "apps", label: "Windows apps", icon: "window", desc: "Windows programs (.exe, .msi) with the Proton built into Hearth OS: open, Proton, remove", words: "windows exe msi proton wine dxvk vkd3d ge-proton program installer" },
    { id: "defaults", file: "SetDefaults.qml", cat: "apps", label: "Default apps", icon: "grid", desc: "Which app opens web links, pictures, music and more", words: "default apps browser terminal", old: "Default apps" },
    { id: "file-types", file: "SetFileTypes.qml", cat: "apps", label: "File types", icon: "doc", desc: "Every kind of file, the app it opens in, and its endings", words: "file associations mime types open with extension endings default app" },
    { id: "autostart", file: "SetAutostart.qml", cat: "apps", label: "Autostart", icon: "power", desc: "Apps and commands that start when you sign in", words: "autostart startup login apps start at sign in command script" },
    { id: "menu-editor", file: "SetMenuEditor.qml", cat: "apps", label: "App menu editor", icon: "edit", desc: "Change an app's name, icon or command, hide it, or add your own", words: "menu editor kmenuedit edit app desktop entry rename icon command hide add to menu" },
    { id: "app-permissions", file: "SetAppPermissions.qml", cat: "apps", label: "App permissions", icon: "shield", desc: "What apps from Flathub may reach: internet, files, devices", words: "flatpak permissions flatseal override sandbox network files home devices" },
    { id: "services", file: "SetServices.qml", cat: "apps", label: "Background services", icon: "gear", desc: "Programs that run in the background for you, and whether they start when you sign in", words: "background services systemd daemons startup autostart user services" },
    { id: "kwin-rules", file: "SetKwinRules.qml", cat: "apps", label: "Window rules", icon: "window", desc: "Make an app's windows open where, how big and how you want", words: "kwin window rules position size desktop keep above no border opacity tearing app class" },
    { id: "browser", file: "SetBrowser.qml", cat: "apps", label: "Web browser", icon: "globe", desc: "Hearth's browser and its settings", words: "web firefox", old: "Web browser" },
    { id: "claude", file: "SetClaude.qml", cat: "apps", label: "Claude", icon: "chat", desc: "Claude on this PC", words: "ai assistant", old: "Claude" },

    // ---- Gaming
    { id: "gaming", file: "SetGaming.qml", cat: "gaming", label: "Gaming", icon: "pad", words: "game mode start in steam", landing: true, ownList: true },
    { id: "savedgames", file: "SetSavedGames.qml", cat: "gaming", label: "Saved games", icon: "sync", desc: "Every game's saves, backed up and put back (Ludusavi)", words: "saves save games backup restore ludusavi cloud saves appdata documents my games" },
    { id: "gameperf", file: "SetGamePerf.qml", cat: "gaming", label: "Performance", icon: "rocket", desc: "Game Mode's resolution, refresh rate, frame limit and HDR", words: "game mode resolution refresh hz frame limit fps cap hdr gamescope" },
    { id: "steam", file: "SetSteam.qml", cat: "gaming", label: "Steam Game Mode and Decky", icon: "steam", desc: "Steam's big-screen mode, Decky Loader and Hearth Sync", words: "game mode decky plugins steam", old: "Steam Game Mode" },
    { id: "startup-animation", file: "SetBoot.qml", cat: "gaming", label: "Startup animation", icon: "film", desc: "What Steam Game Mode plays while it starts", words: "boot startup movie animation", old: "Boot & startup", props: { part: "animation" } },
    { id: "games", file: "SetGames.qml", cat: "gaming", label: "Games & stores", icon: "store", desc: "Epic Games, GOG and Xbox accounts, library sources, mods and saves", words: "steam stores library epic gog xbox", old: "Games & stores" },
    // Discord (1.6): replaced Hearth Friends (Matrix); "friends" is its old id
    { id: "discord", file: "SetDiscord.qml", cat: "gaming", label: "Discord", icon: "chat", desc: "Which Discord the bar opens, what you're playing on Discord, friends joining", words: "friends chat invite rich presence playing join dorion matrix", old: "Friends" },
    { id: "remote", file: "SetRemote.qml", cat: "gaming", label: "Remote play", icon: "play", desc: "Stream this PC's games to your phone or TV, play another PC here, or use this PC from anywhere", words: "moonlight sunshine stream anywhere remote desktop work tailscale", old: "Remote play" },
    { id: "controller", file: "SetController.qml", cat: "gaming", label: "Controller", icon: "pad", desc: "Controller mouse, button layout, what games see, remapping", words: "gamepad buttons pad", old: "Controller" },
    { id: "kwin-games", file: "SetKwinGames.qml", cat: "gaming", label: "Full-screen games & X11 apps", icon: "pad", desc: "Tearing in full-screen games, keys X11 apps can see, scaling X11 apps", words: "kwin tearing vsync latency xwayland x11 push to talk eavesdrop scaling blurry" },
    { id: "handheld", file: "SetHandheld.qml", cat: "gaming", label: "Handheld", icon: "bolt", desc: "Power limit, fans, lights and back buttons (Handheld Daemon)", words: "hhd tdp rgb lights fans", old: "Handheld" },

    // ---- Accounts
    { id: "accounts", file: "SetCategory.qml", cat: "accounts", label: "Accounts", icon: "user", landing: true },
    { id: "users", file: "SetUsers.qml", cat: "accounts", label: "Your account and other people", icon: "users", desc: "Your picture and name, other people on this PC", words: "accounts users people family", old: "Users" },
    { id: "signin", file: "SetSignin.qml", cat: "accounts", label: "Sign-in options", icon: "lock", desc: "Sign in automatically, password, lock after sleep", words: "password login autologin lock", old: "Sign-in" },
    { id: "online-accounts", file: "SetAccounts.qml", cat: "accounts", label: "Online accounts", icon: "cloud", desc: "Nextcloud, Google, Microsoft, CalDAV, CardDAV, WebDAV and IMAP: sign in once for calendar, contacts, files and mail", words: "online accounts nextcloud google microsoft outlook gmail caldav carddav webdav imap mail calendar contacts cloud sync sign in kaccounts" },
    { id: "login-screen", file: "SetLoginScreen.qml", cat: "accounts", label: "Login screen", icon: "image", desc: "The picture behind the login screen", words: "login screen sddm greeter background picture wallpaper lock" },

    // ---- Home server (always its own category; Overview is the landing)
    { id: "hs-overview", file: "SetHomeServer.qml", cat: "hs", label: "Home server", icon: "server", words: "nas home server light mode overview", old: "Home server › Overview", landing: true, ownList: true },
    { id: "hs-server", file: "SetServerMode.qml", cat: "hs", label: "Server mode", icon: "server", desc: "A dashboard on the screen, one press to play, the web page over Tailscale", words: "server mode dashboard idle power screen dim web page pin phone laptop schedule light mode play" },
    { id: "hs-aiengine", file: "SetAIEngine.qml", cat: "hs", parent: "hs-server", label: "AI engine", icon: "chat", desc: "Ollama, Colibri or both: Colibri runs much bigger models from the graphics card, memory and drive", words: "ai engine colibri ollama big models moe mixture of experts nvme vram kimi deepseek glm qwen mimo download catalog openai" },
    { id: "hs-setup", file: "SetHomeSetup.qml", cat: "hs", label: "Set up your home server", icon: "check", desc: "A few steps: name, Tailscale, a shared folder, a backup", words: "nas setup steps", old: "Home server › Setup", hidden: true },
    { id: "hs-shares", file: "SetHomeShares.qml", cat: "hs", label: "Shared folders", icon: "folder", desc: "What your devices can open, and how", words: "nas smb samba share network windows folder", old: "Home server › Shared folders" },
    { id: "hs-files", file: "SetHomeFiles.qml", cat: "hs", label: "Files & links", icon: "link", desc: "Search every share, share links, drop-off folder", words: "nas search find links share link drop-off upload duplicates big files screenshots clean up", old: "Home server › Files & links" },
    { id: "hs-people", file: "SetHomePeople.qml", cat: "hs", label: "People", icon: "users", desc: "Family and friends with their own password", words: "nas family users accounts guest quota", old: "Home server › People" },
    { id: "hs-family", file: "SetHomeFamily.qml", cat: "hs", label: "Family & home", icon: "family", desc: "A Family folder, calendar and cameras", words: "nas family cameras onvif folder", old: "Home server › Family & home" },
    { id: "hs-services", file: "SetHomeServices.qml", cat: "hs", label: "Services", icon: "window", desc: "Media server, files on the web, AI chat, pictures and voices", words: "nas jellyfin media server photos web files apps comfyui kokoro", old: "Home server › Services" },
    { id: "hs-ai", file: "SetHomeAI.qml", cat: "hs", parent: "hs-services", label: "AI chat", icon: "chat", desc: "Ollama for the Hearth AI Command Center: models, address, Light mode", words: "nas ollama ai llm models command center gpu qwen", old: "Home server › AI" },
    { id: "hs-aidevices", file: "SetHomeAIDevices.qml", cat: "hs", label: "AI devices", icon: "desktops", desc: "Share this PC's AI with your devices, and every Hearth device's AI in one list", words: "ai share devices command center tailscale ollama kokoro comfyui whisper voices speech priority", old: "Home server › AI devices" },
    { id: "hs-apps", file: "SetHomeApps.qml", cat: "hs", label: "Server apps", icon: "box", desc: "Game servers and your own apps, one click each", words: "server apps catalog minecraft valheim terraria palworld game server vaultwarden bitwarden passwords immich photos navidrome music subsonic ntfy podman containers self-hosted" },
    { id: "hs-phone", file: "SetHomePhone.qml", cat: "hs", label: "Phone notifications", icon: "bell", desc: "Alerts and stopped apps on your phone, through ntfy", words: "ntfy phone notifications push alerts android iphone topic" },
    { id: "hs-gaming", file: "SetHomeGaming.qml", cat: "hs", label: "Gaming", icon: "pad", desc: "A Games folder for your devices, Steam downloads at home", words: "nas games steam lan install remote play light mode", old: "Home server › Gaming" },
    { id: "hs-drives", file: "SetHomeDrives.qml", cat: "hs", label: "Drives & undo", icon: "drive", desc: "Health of every drive, snapshots to bring files back", words: "nas disk smart health snapshot undo raid mirror format", old: "Home server › Drives & undo" },
    { id: "hs-backups", file: "SetHomeBackups.qml", cat: "hs", label: "Backups & sync", icon: "cloud", desc: "To a drive, another Hearth device or the cloud", words: "nas backup restore restic cloud b2 s3 sync syncthing photos phone", old: "Home server › Backups & sync" },
    { id: "hs-safety", file: "SetHomeSafety.qml", cat: "hs", label: "Safety", icon: "shield", desc: "Who can reach what, open ports, ransomware guard", words: "nas security ransomware guard ports firewall", old: "Home server › Safety" },
    { id: "hs-alerts", file: "SetHomeAlerts.qml", cat: "hs", label: "Alerts & power", icon: "bell", desc: "Failing or full drives, sleep and wake, UPS", words: "nas alerts smart notifications sleep wake on lan spin down ups power schedule", old: "Home server › Alerts & power" },

    // ---- Privacy & security
    { id: "privacy", file: "SetCategory.qml", cat: "privacy", label: "Privacy & security", icon: "shield", landing: true },
    { id: "player-count", file: "SetAbout.qml", cat: "privacy", label: "Player count", icon: "users", desc: "Hearth OS's anonymous count of running devices", words: "privacy counter telemetry anonymous", old: "About › Privacy", props: { part: "privacy" }, status: "countMe" },
    { id: "recent-files", file: "SetRecentFiles.qml", cat: "privacy", label: "Recent files", icon: "clock", desc: "Remember the files you open, for how long, forget them, folders and apps left out", words: "recent files history recently used forget privacy activities jump list recommended" },
    { id: "firmware", file: "SetFirmware.qml", cat: "privacy", label: "Firmware & device security", icon: "shield", desc: "Secure Boot, the TPM security chip, firmware updates for this PC's parts", words: "firmware fwupd lvfs bios uefi update secure boot tpm hsi security chip" },
    { id: "passwords", file: "SetPasswords.qml", cat: "privacy", label: "Passwords", icon: "key", desc: "Sign-ins apps keep in the password store, keyrings, lock", words: "keyring gnome keyring kwallet wallet secret service saved passwords sign-ins remove lock unlock" },

    // ---- Accessibility (the page is the landing)
    { id: "accessibility", file: "SetAccessibility.qml", cat: "a11y", label: "Accessibility", icon: "access", words: "magnifier contrast text size screen reader colour filters reduce motion", old: "Accessibility", landing: true },

    // ---- Updates & recovery (the page is the landing)
    { id: "updates", file: "SetUpdates.qml", cat: "updates", label: "Updates & recovery", icon: "update", words: "update upgrade version go back rollback decky plugins writable", old: "Updates", landing: true },
    { id: "backup", file: "SetAbout.qml", cat: "updates", label: "Backup and help", icon: "drive", desc: "Back up or restore Hearth, logs, send logs to Dragongamer9", words: "backup restore logs help send", old: "About › Backup", props: { part: "backup" } }
];

// Cards that open a page whose one real home is in another category (settings that show up in two places on purpose).
// 1.14, the new Settings: the controller look's tabs (SettingsFrameB.qml, mockups settings-B1/B2), LB / RB between
// them. Each tab holds whole categories, so every page is in one tab: Quick is its own page.
var TABS = [
    { id: "quick", label: "Quick", cats: [] },
    { id: "system", label: "System", cats: ["system", "home", "apps", "updates", "a11y"] },
    { id: "network", label: "Network", cats: ["net"] },
    { id: "devices", label: "Devices", cats: ["bt", "link"] },
    { id: "gaming", label: "Gaming", cats: ["gaming"] },
    { id: "look", label: "Look", cats: ["pers"] },
    { id: "accounts", label: "Accounts", cats: ["accounts", "privacy"] },
    { id: "server", label: "Server", cats: ["hs"] }
];
function tabOf(pageId) {
    if (pageId === "quick") return "quick";
    var p = page(pageId);
    for (var i = 0; i < TABS.length; i++) if (p && TABS[i].cats.indexOf(p.cat) >= 0) return TABS[i].id;
    return "quick";
}
// A tab's pages for its list on the left: each category's own page first (when it isn't just the category's list of
// cards), then its pages. Subpages open from their page, as in the other look.
function tabPages(tabId, live) {
    var t = null;
    for (var i = 0; i < TABS.length; i++) if (TABS[i].id === tabId) t = TABS[i];
    if (!t) return [];
    var out = [];
    for (var j = 0; j < t.cats.length; j++) {
        var c = category(t.cats[j]);
        var land = c ? page(c.landing) : null;
        if (land && land.file !== "SetCategory.qml" && out.indexOf(land) < 0) out.push(land);
        var subs = pagesIn(t.cats[j], live);
        for (var k = 0; k < subs.length; k++) if (out.indexOf(subs[k]) < 0) out.push(subs[k]);
    }
    return out;
}

var LINKS = {
    net: [],
    hs: [{ page: "hs-tailscale", desc: "Lives in Network & internet now" }],
    gaming: [{ page: "storage", label: "Game storage", desc: "Games by size, moving games between drives, low space warning" }],
    privacy: [{ page: "clipboard", label: "Clipboard history", desc: "What you copy is kept for Super+V. Turn it off or clear it" },
              { page: "signin", label: "Lock after sleep", desc: "Ask for your password when this PC wakes" },
              { page: "hs-safety", label: "Home server safety", desc: "Who can reach what, open ports, ransomware guard" }],
    accounts: [{ page: "home", label: "Device name", desc: "This PC's name on your network: rename it on Home" }],
    // 1.14: backing up your files is Home server › Backups & sync (to a drive, another Hearth device or the cloud).
    updates: [{ page: "hs-backups", label: "Back up your files", desc: "To a drive, another Hearth device or the cloud" }]
};

// The heading over a landing page's other pages ("More in <category>" when there's none here).
var MORE = { net: "Reach your devices from anywhere" };

// Ids that aren't pages: old forms that open a page.
var ALIASES = { "appearance:bar": "bar-dock", "personalization": "appearance", "hs": "hs-overview", "a11y": "accessibility", "hearth-link": "link" };

function page(id) { for (var i = 0; i < PAGES.length; i++) if (PAGES[i].id === id) return PAGES[i]; return null; }
function category(id) { for (var i = 0; i < CATEGORIES.length; i++) if (CATEGORIES[i].id === id) return CATEGORIES[i]; return null; }
function landingOf(catId) { var c = category(catId); return c ? c.landing : "home"; }
// The pages a category lists (not its landing, not hidden ones, not live-only ones off the live USB).
function pagesIn(catId, live) {
    return PAGES.filter(function (p) { return p.cat === catId && !p.landing && !p.hidden && !p.parent && (!p.live || live); });
}
function resolve(id) {
    if (!id) return "";
    if (page(id)) return id;
    if (ALIASES[id]) return ALIASES[id];
    var c = category(id);
    return c ? c.landing : "";
}
// Breadcrumbs: [{id, label}] from the category to the page.
function crumbs(id) {
    var p = page(id);
    if (!p) return [];
    var c = category(p.cat), out = [];
    if (p.landing) return [{ id: id, label: c ? c.label : p.label }];
    out.push({ id: c ? c.landing : "home", label: c ? c.label : "" });
    if (p.parent) { var q = page(p.parent); if (q) out.push({ id: q.id, label: q.label }); }
    out.push({ id: id, label: p.label });
    return out;
}
// Back from a page: its parent, else its category's landing ("" on a landing).
function up(id) {
    var p = page(id);
    if (!p || p.landing) return "";
    return p.parent || landingOf(p.cat);
}
function pathOf(id) { return crumbs(id).map(function (c) { return c.label; }).join(" › "); }
