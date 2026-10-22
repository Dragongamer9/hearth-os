#!/usr/bin/env python3
"""Builds site/docs/*.html (Hearth OS documentation) from the content below: one header, section list and
footer for every page. The Website workflow runs it before publishing; the release notes come from docs/ROADMAP.md
and the version from Nimbus.Host.csproj, so they follow each release. Usage: tools/build-docs.py [site/docs]"""
import html, os, re, sys

OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(os.path.dirname(__file__), '..', 'site', 'docs')
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..')
R2 = 'https://pub-f59beb6fa21c421ca4435ee7f42b3de4.r2.dev/'
VERSION = re.search(r'<Version>([^<]+)</Version>', open(os.path.join(ROOT, 'hosts/windows/Nimbus.Host/Nimbus.Host.csproj')).read()).group(1)
ROADMAP = open(os.path.join(ROOT, 'docs/ROADMAP.md')).read()

PAGES = [
    ('index', 'Overview', 'Start here'),
    ('install', 'Installing', 'Get Hearth OS onto a PC or handheld'),
    ('updates', 'Updates and editions', 'Updates, going back, switching edition'),
    ('using', 'Using Hearth', 'Game Mode, the desktop, buttons'),
    ('troubleshooting', 'Troubleshooting', 'When something goes wrong'),
    ('requests', 'Requests', 'Ideas, changes and new editions'),
    ('releases', 'Release notes', 'What changed in each version, and what\'s next'),
]


EDITIONS = [('anatase', 'Anatase', 'Fedora'), ('ubuntu', 'Ubuntu', 'Ubuntu'), ('cachyos', 'CachyOS', 'Arch'), ('peppermint', 'Peppermint', 'Debian')]
# Sections (by heading id) that only apply to some editions; everything else is for all of them.
SECTION_EDS = {
    'podman': 'ubuntu cachyos peppermint',
    'brightness': 'ubuntu peppermint',
    'boot-text': 'anatase',
    'from-other': 'anatase',
}


def only(eds, text, tag='div'):
    """A part of a page that only applies to some editions (hidden by the edition picker otherwise)."""
    return f'<{tag} class="ed" data-ed="{eds}">{text}</{tag}>'


def wrap_sections(body):
    """Each h2 and what follows it becomes a <section>; the edition-specific ones get data-ed."""
    parts = re.split(r'(?=<h2 id=")', body)
    out = [parts[0]]
    for part in parts[1:]:
        sid = re.match(r'<h2 id="([^"]+)"', part).group(1)
        eds = SECTION_EDS.get(sid)
        out.append(f'<section class="ed" data-ed="{eds}">{part}</section>' if eds else f'<section>{part}</section>')
    return ''.join(out)


def picker():
    opts = '<option value="all">All editions</option>' + ''.join(f'<option value="{e}">{n} ({b})</option>' for e, n, b in EDITIONS)
    return (f'<label class="picker"><span>Edition</span><select aria-label="Show the docs for this edition">{opts}</select></label>'
            '<span class="edshow" hidden></span>')


def cmd(name, text):
    """A command with Copy and "Did this work?" (docs.js adds the buttons)."""
    return f'<div class="cmd" data-cmd="{name}"><pre><code>{html.escape(text)}</code></pre></div>'


def h2(id_, text):
    return f'<h2 id="{id_}">{text}</h2>'


def page(slug, title, lead, body):
    sections = re.findall(r'<h2 id="([^"]+)">(.*?)</h2>', body)
    side = ['<div class="grp"><h4>Docs</h4>']
    for s, t, _ in PAGES:
        href = 'index.html' if s == 'index' else f'{s}.html'
        side.append(f'<a href="{href}"{" class=\"cur\" aria-current=\"page\"" if s == slug else ""}>{t}</a>')
        if s == slug and sections:
            side.append('<div class="sub">' + ''.join(f'<a href="#{i}"' + (f' data-ed="{SECTION_EDS[i]}"' if i in SECTION_EDS else '') + f'>{re.sub("<[^>]+>", "", t2)}</a>' for i, t2 in sections) + '</div>')
    side.append('</div>')
    i = [p[0] for p in PAGES].index(slug)
    prev = PAGES[i - 1] if i > 0 else None
    nxt = PAGES[i + 1] if i + 1 < len(PAGES) else None
    href = lambda s: 'index.html' if s == 'index' else f'{s}.html'
    pager = '<div class="pager">' + (f'<a href="{href(prev[0])}">← {prev[1]}</a>' if prev else '<span></span>') + \
            (f'<a href="{href(nxt[0])}">{nxt[1]} →</a>' if nxt else '<span></span>') + '</div>'
    full_title = 'Hearth OS docs' if slug == 'index' else f'{title} · Hearth OS docs'
    return f'''<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{full_title}</title>
<meta name="description" content="{html.escape(lead)}">
<meta name="theme-color" content="#120c09">
<link rel="icon" href="../img/logo.svg" type="image/svg+xml">
<link rel="preconnect" href="https://fonts.googleapis.com">
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800;900&display=swap" rel="stylesheet">
<link rel="stylesheet" href="docs.css">
</head>
<body>
<a class="skip" href="#main">Skip to the page</a>
<nav class="top" aria-label="Main">
  <div class="wrap">
    <a class="brand" href="../"><img src="../img/logo.svg" alt="">Hearth OS <span>Docs</span></a>
    <div class="links">
      <a href="../">Home</a>
      <a class="keep" href="troubleshooting.html">Help</a>
      <a href="requests.html">Requests</a>
      <a href="releases.html">Release notes</a>
      <a href="https://github.com/Dragongamer9/hearth-os">GitHub</a>
      <a class="cta" href="../#download">Download</a>
    </div>
  </div>
</nav>
<div class="wrap layout">
  <aside class="side" aria-label="Sections">{''.join(side)}</aside>
  <main id="main">
    <div class="crumb"><span>Docs · for Hearth OS {VERSION}</span>{picker()}</div>
    <h1>{title}</h1>
    <p class="lead">{lead}</p>
{wrap_sections(body)}
{pager}
  </main>
</div>
<footer>
  <div class="wrap">
    <a class="brand" href="../"><img src="../img/logo.svg" alt="" width="22" height="22">Hearth OS</a>
    <a href="releases.html">Release notes</a>
    <a href="requests.html">Requests</a>
    <a href="troubleshooting.html#ask">Report a problem</a>
  </div>
</footer>
<script src="docs.js" defer></script>
</body>
</html>
'''


EDITION_NOTES = (
    only('anatase', '''<p><b>Anatase</b> is Fedora underneath, as an image (bootc). It's the main edition: every change is tested on it first.</p>
<ul>APPS<li>Steam: from Flathub, installed the first time the device is online.</li>
<li>Terminal tools that change the system use <code>rpm-ostree</code> (Fedora's image tool).</li></ul>''')
    + only('ubuntu', '''<p><b>Ubuntu</b> is Ubuntu 26.04 underneath, made into an image (bootc). <span class="ver">Beta Experimental</span></p>
<ul>APPS<li><code>apt</code> can't change the system itself: it's a read-only image that updates in one piece.</li><li>Steam: built in, from Ubuntu's packages.</li></ul>''')
    + only('cachyos', '''<p><b>CachyOS</b> is Arch-based CachyOS underneath, made into an image (bootc), with CachyOS's handheld kernel (Steam Deck, ROG Ally, Legion Go and MSI Claw patches). <span class="ver">Beta Experimental</span></p>
<ul>APPS<li><code>pacman</code> can't change the system itself: it's a read-only image that updates in one piece.</li><li>Steam: built in, from CachyOS's packages.</li></ul>''')
    + only('peppermint', '''<p><b>Peppermint</b> is Debian 13 "trixie" underneath (Peppermint OS's base), made into an image (bootc). <span class="ver">Beta Experimental</span></p>
<ul>APPS<li><code>apt</code> can't change the system itself: it's a read-only image that updates in one piece.</li><li>Steam: built in, from Debian's packages.</li></ul>''')
).replace('APPS', '<li>Apps: Flatpak, from Flathub, in the <b>Apps</b> store (Desktop Mode).</li>')

# ------------------------------------------------------------------ Overview
INDEX = f'''
<div class="cards">
  <a href="install.html"><b>Installing</b><span>Downloads, making the USB stick, installing next to Windows or on its own.</span></a>
  <a href="updates.html"><b>Updates and editions</b><span>How updates work, going back to the last version, switching edition.</span></a>
  <a href="using.html"><b>Using Hearth</b><span>Game Mode, the desktop (Mantel), the keyboard, the power button, sending logs.</span></a>
  <a href="troubleshooting.html"><b>Troubleshooting</b><span>Fixes for known problems, how to get unstuck, and how to tell us.</span></a>
  <a href="requests.html"><b>Requests</b><span>Ask for a feature, a change, device support or a new edition.</span></a>
</div>
{h2('what', 'What Hearth OS is')}
<p>Hearth OS is a controller-first gaming system for handhelds and PCs. It starts in <b>Game Mode</b>, a console-style
screen with Steam, Epic, GOG, Xbox and your other games in one library, and has a full desktop called <b>Mantel</b> one
button away (<b>Desktop Mode</b>).</p>
<p>It's <b>image-based</b>: the whole system is one tested image that updates in one piece, in the background.
An update can't half-install, and the version you had before stays on the drive so you can go back to it.</p>
<div class="note"><p>Hearth OS is in <b>beta</b>. It works for everyday play, but expect rough edges. When something breaks,
<a href="troubleshooting.html#logs">send a log</a> and it gets looked at.</p></div>

{h2('editions', 'Editions')}
<p>Hearth looks and works the same on every edition. What changes is the Linux underneath it.</p>
<div class="tbl"><table>
<tr><th>Edition</th><th>Built on</th><th>Status</th><th>Good for</th></tr>
<tr><td><b>Anatase</b></td><td>Fedora (Anatase)</td><td>Recommended</td><td>Everyone. The most tested edition, with the best hardware support.</td></tr>
<tr><td><b>Ubuntu</b></td><td>Ubuntu 26.04</td><td>Beta Experimental</td><td>People who know Ubuntu, or need Ubuntu packages.</td></tr>
<tr><td><b>CachyOS</b></td><td>CachyOS (Arch)</td><td>Beta Experimental</td><td>Newer processors (x86-64-v3: AVX2). Tuned kernel, handheld patches.</td></tr>
<tr><td><b>Peppermint</b></td><td>Debian 13</td><td>Beta Experimental</td><td>People who prefer Debian's slower, steady pace.</td></tr>
</table></div>
<p>You can <a href="updates.html#switch">switch edition</a> later without a USB stick. Your games and saves come along.</p>

{h2('yours', 'About your edition')}
<p class="ed" data-ed="all">Pick your edition above to see what's particular about it.</p>
{EDITION_NOTES}

{h2('needs', 'What you need')}
<ul>
<li>A 64-bit PC or handheld (x86-64) with AMD, Intel or Nvidia graphics (Nvidia GTX 16 / RTX 20 or newer).</li>
<li>For installing: a USB stick of 16 GB or more, and a drive of 64 GB or more.</li>
<li class="ed" data-ed="cachyos">A processor from about 2015 or newer (x86-64-v3: AVX2, BMI2, FMA). Hearth checks before switching to CachyOS.</li>
</ul>
<p>Tested on the ASUS ROG Ally and Xbox Ally, and desktop PCs with AMD and Nvidia graphics.</p>
'''

# ------------------------------------------------------------------ Installing
INSTALL = f'''
{h2('download', 'Downloads')}
<p>Pick one. The <b>offline</b> USBs carry the whole system and install without internet. The small installer
downloads the system while it installs, and offers every edition.</p>
<div class="tbl"><table>
<tr><th>Download</th><th>Size</th><th>Internet while installing</th></tr>
<tr class="ed" data-ed="anatase"><td><a href="{R2}hearth-os-live.iso">Anatase, offline USB</a> (recommended)</td><td>about 9 GB</td><td>Not needed. Try it before installing.</td></tr>
<tr><td><a href="{R2}hearth-os-installer.iso">Small installer</a> (every edition)</td><td>1.6 GB</td><td>Needed</td></tr>
<tr class="ed" data-ed="ubuntu"><td><a href="{R2}hearth-os-ubuntu.iso">Ubuntu, offline</a></td><td>about 4.4 GB</td><td>Not needed</td></tr>
<tr class="ed" data-ed="cachyos"><td><a href="{R2}hearth-os-cachyos.iso">CachyOS, offline</a></td><td>about 5.2 GB</td><td>Not needed</td></tr>
<tr class="ed" data-ed="peppermint"><td><a href="{R2}hearth-os-peppermint.iso">Peppermint, offline</a></td><td>about 4.3 GB</td><td>Not needed</td></tr>
</table></div>
<p>To check a download wasn't damaged, compare its checksum (a <code>.sha256</code> file sits next to each ISO, for example
<a href="{R2}hearth-os-live.iso.sha256">hearth-os-live.iso.sha256</a>). On Linux:</p>
{cmd('sha256-check', 'sha256sum hearth-os-live.iso')}
<p>The result should match the text in the <code>.sha256</code> file.</p>

{h2('usb', 'Making the USB stick')}
<ol class="steps">
<li><b>Get a USB writer.</b> <a href="https://flathub.org/apps/org.fedoraproject.MediaWriter">Fedora Media Writer</a> or
<a href="https://etcher.balena.io/">balenaEtcher</a>, on Windows, macOS or Linux.</li>
<li><b>Write the ISO to the stick.</b> Choose "custom image" or "flash from file", pick the ISO, pick the stick. Everything on the stick is erased.</li>
</ol>

{h2('boot', 'Starting from the USB')}
<ol class="steps">
<li><b>Plug in the stick and turn the device on</b> while pressing its boot-menu key: often <b>F12</b>, <b>F11</b> or <b>F8</b> on PCs.
On a <b>ROG Ally</b>: hold <b>Volume Down</b> while pressing power.</li>
<li><b>Pick the USB stick</b> in the list. If it isn't there, Secure Boot or "Fast Boot" may be hiding it: turn those off in the firmware settings (often <b>Del</b> or <b>F2</b> at power-on).</li>
<li><b>Try it out.</b> The offline Anatase USB starts Hearth straight away with a welcome card. Nothing on the drive changes until you choose to install.</li>
</ol>

{h2('installing', 'Installing')}
<ol class="steps">
<li><b>Choose Install Hearth</b> (on the welcome card, or the installer that opens on its own).</li>
<li><b>Pick where it goes:</b> a whole drive (everything on it is erased), or <b>next to Windows</b> if there's space.</li>
<li><b>Make your account</b>: name, password, time zone, Wi-Fi.</li>
<li><b>Wait, then restart</b> when it says it's done, and take the USB stick out.</li>
</ol>
<div class="note"><p>Every ISO is tested before it's published: it's installed on a virtual PC and the installed system has to start.
If an install fails on your hardware, <a href="troubleshooting.html#logs">send the installer log</a>: the installer saves one.</p></div>

{h2('first', 'The first start')}
<ul>
<li>Hearth starts in Game Mode. Sign in to Steam and your other stores from <b>Settings › Game stores</b>.</li>
<li class="ed" data-ed="anatase">Steam installs itself (from Flathub) the first time the device is online, so after an offline install it appears a few minutes after you connect.</li>
<li class="ed" data-ed="ubuntu cachyos peppermint">Steam is already installed (from the edition's own packages).</li>
<li>An installed system starts at the version the ISO was made with, and <a href="updates.html">updates itself</a> to the newest one once it's online.</li>
</ul>

{h2('from-other', 'Coming from Bazzite or Fedora Atomic')}
<p>Bazzite, Bluefin, Aurora, Silverblue and Kinoite can move to Hearth OS without a USB stick; your account and files stay.
From the Bazzite-based Hearth OS, use <b>Settings › Updates › Move to a new Hearth OS</b>, or in Desktop Mode:</p>
{cmd('move-from-bazzite', 'pkexec /usr/libexec/nimbus/hearth-admin move-to-new-os')}
<div class="note warn"><p>Beta Experimental: back up anything important first.</p></div>
'''

# ------------------------------------------------------------------ Updates and editions
UPDATES = f'''
{h2('how', 'How updates work')}
<ul>
<li>Hearth checks for updates when it starts and every 6 hours (<b>Settings › Updates › Check automatically</b>).</li>
<li>An update downloads in the background while you play. It's used <b>the next time you restart</b>.</li>
<li>The whole system is replaced in one piece, so an update can't half-install. The version before it stays on the drive.</li>
<li>To update now: <b>Settings › Updates › Check for updates</b>, then restart when it says it's ready.</li>
</ul>
<p>From a terminal in Desktop Mode, the same update:</p>
{cmd('update-now', 'pkexec /usr/libexec/nimbus/hearth-admin update')}

{h2('rollback', 'Going back to the last version')}
<p>If a new version has a problem, go back to the one before it.</p>
<h3>From the boot menu</h3>
<p>Restart, and in the boot menu pick the second <b>Hearth OS</b> entry (the older version). On a handheld, the boot menu
can be moved with the D-pad or volume buttons.</p>
<h3>From a terminal</h3>
<p>This makes the version before the default, from the next restart on:</p>
{cmd('rollback', 'sudo bootc rollback')}
<p>Then restart. The next update moves you forward again.</p>

{h2('switch', 'Switching edition')}
<p>Move between Anatase, Ubuntu, CachyOS and Peppermint without a USB stick: <b>Settings › Updates › Switch edition</b>.</p>
<ul>
<li><b>Kept:</b> your account, your games, saves and store libraries (Steam, Epic, GOG...).</li>
<li><b>Set aside:</b> the rest of your home folder goes into a folder called <code>.old-hearth-</code> plus the date, for 30 days,
so you can copy back anything you need. After 30 days it's deleted.</li>
<li>The new edition downloads in the background (about 3 GB), then starts at the next restart. Before anything starts you see exactly what's kept.</li>
</ul>
<div class="note warn ed" data-ed="ubuntu cachyos peppermint"><p><b>Installed from 0.3.0 or earlier:</b> switching edition (and updating) fails with
"podman executable not found in PATH". See <a href="troubleshooting.html#podman">the fix</a>.</p></div>
<h3>Switching from a terminal</h3>
<p>The same switch as the Settings button. Change <code>ubuntu</code> to <code>anatase</code>, <code>cachyos</code> or <code>peppermint</code>:</p>
{cmd('switch-edition', 'pkexec /usr/libexec/nimbus/hearth-admin move-to-new-os --edition ubuntu')}
<p>When it says <b>Done. Restart to start the new Hearth OS.</b>, restart.</p>

{h2('versions', 'Versions')}
<p>What changed in each version, and what's coming next: <a href="releases.html">release notes</a>.</p>
'''

# ------------------------------------------------------------------ Using Hearth
USING = f'''
{h2('modes', 'Game Mode and Desktop Mode')}
<p><b>Game Mode</b> is the console-style screen Hearth starts in. <b>Desktop Mode</b> (Mantel) is a full desktop with windows,
a file manager and a terminal.</p>
<ul>
<li>To the desktop: <b>Power › Switch to desktop</b> in Game Mode.</li>
<li>Back to Game Mode: the <b>Game Mode</b> button at the bottom right of the desktop.</li>
<li><b>Settings › Sign-in › Start in</b> picks which one you see after signing in.</li>
</ul>

{h2('controller', 'Controller')}
<div class="tbl"><table>
<tr><th>Button</th><th>In Game Mode</th></tr>
<tr><td>A / B</td><td>Select / back</td></tr>
<tr><td>Y</td><td>Search</td></tr>
<tr><td>Menu (☰)</td><td>Main menu</td></tr>
<tr><td>View (⧉)</td><td>Quick Access: volume, brightness, power, Wi-Fi</td></tr>
<tr><td>Xbox / Guide</td><td>Over a game: the overlay (back to Hearth, close the game, settings)</td></tr>
<tr><td>View + Menu together</td><td>Controller mouse on or off (for launchers and installers a controller can't drive)</td></tr>
</table></div>
<p>The controller mouse: right stick moves the pointer, A or RT clicks, X or LT right-clicks, left stick scrolls, B is Esc.</p>

{h2('keyboard', 'The on-screen keyboard')}
<p>Laid out like SteamOS's, at the bottom of the screen. With a controller:</p>
<div class="tbl"><table>
<tr><th>Button</th><th>Does</th></tr>
<tr><td>A</td><td>Type the selected key</td></tr>
<tr><td>Y</td><td>Delete</td></tr>
<tr><td>X</td><td>Space</td></tr>
<tr><td>LB</td><td>Shift (twice for caps lock)</td></tr>
<tr><td>RB</td><td>Symbols (&amp;123)</td></tr>
<tr><td>Menu</td><td>Done</td></tr>
<tr><td>B</td><td>Close</td></tr>
</table></div>
<p>The ← and → keys move the text cursor. A USB or Bluetooth keyboard works too.</p>

{h2('power', 'The power button')}
<p><b>Settings › Power › Power button</b> sets what a quick <b>Press</b> and a <b>Hold</b> do (sleep, the power menu, turn off).
Holding the button for several seconds always turns the device off.</p>

{h2('display', 'Display and performance')}
<ul>
<li><b>Settings › Display › Drawing engine</b>: on Nvidia graphics Hearth uses Chromium to draw Game Mode, which is much smoother there. It's chosen automatically.</li>
<li><b>Settings › Power</b>: power mode and the power limit (TDP) on handhelds.</li>
</ul>

{h2('profiles', 'Profiles and stores')}
<ul>
<li>Each Hearth profile can sign in to its own Steam account. With more than one profile, each sees its own Steam games.</li>
<li>Stores are signed in to from <b>Settings › Game stores</b>. Beacon (the Battle for Middle-earth launcher) is built in.</li>
</ul>

{h2('apps', "Hearth's own apps")}
<p>On Hearth OS and on Windows (Ember), from Desktop Mode's app menu or the Start menu:</p>
<ul>
<li><b>Hearth Browser</b>: real Firefox with Hearth's look. Google sign-in, streaming video and uBlock Origin work. In Game Mode it opens full screen with bigger buttons and an on-screen keyboard (it opens by itself when you pick the address bar; or the Keyboard button, or Ctrl+Alt+K).</li>
<li><b>Terminal</b>: tabs, split view (Ctrl+Shift+D), find (Ctrl+Shift+F), themes, and a quick-keys bar and on-screen keyboard for controllers. Your shell on Linux; PowerShell, Command Prompt or WSL on Windows.</li>
<li><b>Disks</b>: your drives and their health, formatting USB drives and SD cards, partitions, and <b>Use for games</b> (a drive or SD card as a Steam library in one step). It won't touch the drive the system runs from.</li>
<li><b>hearthfetch</b>: type it in the Terminal for a summary of your system.</li>
</ul>
<p>Desktop Mode's app drawer has a <b>Games</b> tab with your installed games; on Windows it also has a <b>Windows</b> tab with the built-in programs.</p>

{h2('logs', 'Sending logs')}
<p>When something goes wrong, <b>Settings › System › Send logs to the developer</b>. You get a short code (like
<code>ABCD-EFGH-JK</code>): share it when you report the problem. Logs are kept 14 days, personal details are taken out
before they're sent, and only someone with the code can read them.</p>
'''

# ------------------------------------------------------------------ Troubleshooting
TROUBLE = f'''
<div class="note"><p>Each fix says which version it's for. Most known problems are fixed by updating:
<b>Settings › Updates › Check for updates</b>. Under every command, tell us whether it worked: that's how we know which fixes need work.</p></div>

{h2('logs', 'First: send a log')}
<p><b>Settings › System › Send logs to the developer</b>, then share the code. The log shows what Hearth saw at the moment
it went wrong, so send it right after the problem happens.</p>

{h2('terminal', 'Getting to a terminal')}
<ul>
<li><b>Desktop Mode:</b> open <b>Terminal</b> from the app menu.</li>
<li><b>When the screen is stuck:</b> with a USB keyboard, press <b>Ctrl + Alt + F3</b> and sign in with your user name and password.
<b>Ctrl + Alt + F1</b> (or F2) goes back.</li>
</ul>

{h2('podman', 'Switching edition or updating fails: "podman executable not found"')}
<p><span class="ver">Peppermint, Ubuntu, CachyOS · 0.3.0 and earlier</span> <span class="ver fixed">Fixed in 0.3.1</span></p>
<p>Those images were missing a tool (podman) that switching and updating need, and updating to the fix needs it too.
This runs the switch with podman unpacked into a temporary folder (nothing is installed; it's gone after the restart).
Change <code>ubuntu</code> to the edition you want:</p>
{cmd('fix-podman-switch', 'curl -sL raw.githubusercontent.com/Dragongamer9/hearth-os/main/tools/switch-edition.sh | sh -s ubuntu')}
<p>When it says <b>Done. Restart to start the new Hearth OS.</b>, restart. A "could not find netavark" line along the way is only a warning.</p>

{h2('stuck', 'Stuck on a black screen, or Hearth doesn\'t respond')}
<ol class="steps">
<li><b>Wait half a minute.</b> Starting a game or switching modes can take a moment.</li>
<li><b>Hold the power button</b> for about 10 seconds to turn the device off, then turn it on again.</li>
<li><b>Still stuck after starting?</b> Pick the older <b>Hearth OS</b> entry in the boot menu (<a href="updates.html#rollback">going back</a>),
then send a log from there.</li>
</ol>

{h2('power-button', 'The power button turns the device off instead of sleeping')}
<p><span class="ver">0.2.10 and earlier</span> <span class="ver fixed">Fixed in 0.3.0</span> Update. After updating, <b>Settings › Power › Power button</b> is followed.</p>

{h2('brightness', 'Brightness doesn\'t change ("get write permission")')}
<p><span class="ver">Peppermint, Ubuntu · 0.3.0 and earlier</span> <span class="ver fixed">Fixed in 0.3.1</span>
Update and restart. If you can't update yet, this gives your account permission (then restart):</p>
{cmd('fix-brightness', 'sudo usermod -aG video $USER')}

{h2('boot-text', 'Lines of text when turning on and off')}
<p><span class="ver">Anatase · 0.2.10 and earlier</span> <span class="ver fixed">Fixed in 0.3.0</span>
Update, then restart once. Or, right away (then restart):</p>
{cmd('fix-boot-text', 'sudo rpm-ostree kargs --append-if-missing=quiet --append-if-missing=splash --append-if-missing=loglevel=3')}

{h2('sounds', 'Game Mode sounds fade out')}
<p><span class="ver fixed">Fixed in 0.3.1</span> Update. Until then, restarting Hearth brings the sounds back for a while.</p>

{h2('steam-mode', 'Stuck in Steam\'s Game Mode')}
<p><span class="ver">0.2.8 and 0.2.9</span> <span class="ver fixed">Removed in 0.2.10</span>
Steam's own Game Mode was an option in 0.2.8 and 0.2.9. If a device is stuck in it: with a USB keyboard press
<b>Ctrl + Alt + F3</b>, sign in, and run (then restart):</p>
{cmd('fix-steam-mode', 'echo hearth > ~/.config/nimbus/game-mode && sudo reboot')}

{h2('nvidia', 'Game Mode feels slow on Nvidia')}
<p>Check <b>Settings › Display › Drawing engine</b> is <b>Chromium</b> (the default on Nvidia since 0.2.6), and update.</p>

{h2('steam-games', 'Steam games are missing from the library')}
<p>Make sure Steam is signed in. With more than one Hearth profile, each profile shows the games of the Steam account
signed in on it: sign in to Steam on that profile.</p>

{h2('install-failed', 'Installing fails')}
<ul>
<li>Try the <b>offline USB</b> for your edition instead of the small installer (no download to go wrong).</li>
<li>If the install says it failed but the computer later starts Hearth OS anyway, it may have finished: check <b>Settings › Updates</b> and update.</li>
<li>From the live USB, <b>Settings › System › Send logs to the developer</b> includes the installer's log.</li>
</ul>

{h2('ask', 'Still stuck? Tell us')}
<p>Describe what happened. It goes straight to the developer, like a log, and you get a code to quote if you follow up.
Don't include passwords.</p>
<form id="ask" class="ask" novalidate>
  <label><span>What went wrong? <span class="req">(needed)</span></span>
    <textarea name="what" rows="5" maxlength="4000" required placeholder="What you did, what you expected, and what happened instead. Error messages word for word help a lot."></textarea></label>
  <div class="row2">
    <label>Edition
      <select name="edition"><option value="">Not sure</option><option>Anatase</option><option>Ubuntu</option><option>CachyOS</option><option>Peppermint</option><option>Ember (Windows)</option></select></label>
    <label>Hearth version <input name="version" maxlength="20" placeholder="e.g. 0.3.1 (Settings › Updates)"></label>
  </div>
  <div class="row2">
    <label>Device <input name="device" maxlength="80" placeholder="e.g. ROG Ally, desktop with RTX 5070"></label>
    <label>Log code, if you sent one <input name="code" maxlength="20" placeholder="ABCD-EFGH-JK"></label>
  </div>
  <label>What you already tried <textarea name="tried" rows="2" maxlength="2000"></textarea></label>
  <label><span>How to reach you <span class="opt">(optional: a GitHub or Discord name)</span></span> <input name="contact" maxlength="100"></label>
  <div class="send"><button type="submit">Send to the developer</button><span class="msg" role="status" aria-live="polite"></span></div>
</form>
'''


# ------------------------------------------------------------------ Requests
REQUESTS = f'''
{h2('what', 'What you can ask for')}
<ul>
<li><b>A new feature</b>, or a change to one that's there: something in Game Mode, the desktop, Settings, the stores.</li>
<li><b>A new edition</b>: Hearth OS on another Linux (for example another family than Fedora, Debian/Ubuntu or Arch).</li>
<li><b>Device support</b>: a handheld, controller or PC part that doesn't work well yet.</li>
<li><b>Anything else</b>: games, launchers, the website, these docs.</li>
</ul>
<p>Something broken? That's <a href="troubleshooting.html#ask">a problem report</a> instead, so it gets fixed first.</p>

{h2('how', 'What happens to a request')}
<ol class="steps">
<li><b>It reaches the developer</b>, straight away, with a code you can quote later.</li>
<li><b>Requests are read and grouped</b>: the same idea from several people counts for more.</li>
<li><b>What's planned goes under <a href="releases.html#next">Coming next</a></b>, and the <a href="releases.html">release notes</a> say when it arrives.</li>
</ol>

{h2('send', 'Send a request')}
<form id="req" class="ask" novalidate>
  <div class="row2">
    <label>What kind of request?
      <select name="type"><option>New feature</option><option>Change something</option><option>New edition</option><option>Device support</option><option>Other</option></select></label>
    <label>For which edition?
      <select name="edition"><option value="">Every edition</option><option>Anatase</option><option>Ubuntu</option><option>CachyOS</option><option>Peppermint</option><option>Ember (Windows)</option><option>A new edition</option></select></label>
  </div>
  <label><span>In a few words <span class="req">(needed)</span></span>
    <input name="title" maxlength="120" required placeholder="e.g. Hearth OS on openSUSE, or a battery saver for handhelds"></label>
  <label><span>Tell us more <span class="req">(needed)</span></span>
    <textarea name="details" rows="5" maxlength="4000" required placeholder="What you'd like, and why: what would it let you do? For a new edition: which Linux, and what you'd use it for."></textarea></label>
  <div class="row2">
    <label><span>Your device <span class="opt">(optional)</span></span> <input name="device" maxlength="80" placeholder="e.g. Legion Go, desktop with RX 7800"></label>
    <label><span>How to reach you <span class="opt">(optional)</span></span> <input name="contact" maxlength="100" placeholder="a GitHub or Discord name"></label>
  </div>
  <div class="send"><button type="submit">Send the request</button><span class="msg" role="status" aria-live="polite"></span></div>
</form>
'''

REQ_JS = '''
// Requests page: sent to the counter's /v1/logs as kind "request" (filed under requests/ with the label "request").
(function(){
  var f = document.getElementById('req'); if(!f) return;
  var API = 'https://hearth-counter.game305-cl.workers.dev';
  f.addEventListener('submit', function(e){
    e.preventDefault();
    var msg = f.querySelector('.msg'), btn = f.querySelector('button'), v = function(n){ return (f.elements[n].value || '').trim(); };
    msg.className = 'msg';
    if(v('title').length < 4){ msg.className = 'msg err'; msg.textContent = 'Give it a short title.'; f.elements.title.focus(); return; }
    if(v('details').length < 10){ msg.className = 'msg err'; msg.textContent = 'Say a bit more about what you\\'d like.'; f.elements.details.focus(); return; }
    var body = 'Hearth logs: a request from hearthos.fun/docs\\n\\n'
      + 'Type: ' + v('type') + '\\n'
      + 'Title: ' + v('title') + '\\n'
      + 'Edition: ' + (v('edition') || 'every edition') + '\\n'
      + 'Device: ' + (v('device') || '?') + '\\n'
      + 'Contact: ' + (v('contact') || 'none') + '\\n\\n'
      + '===== The request =====\\n' + v('details') + '\\n';
    btn.disabled = true; msg.textContent = 'Sending…';
    fetch(API + '/v1/logs', {method: 'POST', body: body, headers: {'content-type': 'text/plain; charset=utf-8',
        'x-hearth-kind': 'request', 'x-hearth-version': 'website', 'x-hearth-note': encodeURIComponent(v('type') + ': ' + v('title'))}})
      .then(function(r){ return r.json().then(function(j){ return {ok: r.ok, j: j}; }); })
      .then(function(x){
        if(x.ok && x.j.code){ msg.innerHTML = 'Thanks! Sent. Your code: <b></b>'; msg.querySelector('b').textContent = x.j.code; f.reset(); }
        else { msg.className = 'msg err'; msg.textContent = (x.j && x.j.error === 'busy, try again later') ? 'Lots of messages right now: try again in an hour.' : 'That didn\\'t send. Try again in a moment.'; }
      })
      .catch(function(){ msg.className = 'msg err'; msg.textContent = 'That didn\\'t send (no connection?). Try again in a moment.'; })
      .then(function(){ btn.disabled = false; });
  });
})();
'''


# ------------------------------------------------------------------ Release notes (from docs/ROADMAP.md)
def md_inline(t):
    t = html.escape(t)
    t = re.sub(r'`([^`]+)`', r'<code>\1</code>', t)
    t = re.sub(r'\*\*([^*]+)\*\*', r'<b>\1</b>', t)
    return t


def roadmap_sections():
    """[(heading, [item, ...])] in file order; items are bullet lines (continuation lines joined)."""
    out, cur = [], None
    for line in ROADMAP.splitlines():
        if line.startswith('## '):
            cur = (line[3:].strip(), []); out.append(cur)
        elif cur is not None and re.match(r'^\s*- ', line):
            cur[1].append(re.sub(r'^\s*- ', '', line).strip())
        elif cur is not None and cur[1] and line.startswith('      ') and line.strip():
            cur[1][-1] += ' ' + line.strip()
    return out


def releases_body():
    secs = roadmap_sections()
    vers = []
    for head, items in secs:
        m = re.match(r'^(\d+)\.(\d+)\.(\d+)(\s*\(released\))?$', head)
        if not m:
            continue
        done = [re.sub(r'^\[x\]\s*', '', i) for i in items if not i.startswith('[ ]')]
        if not done:
            continue
        vers.append(((int(m[1]), int(m[2]), int(m[3])), head.split()[0], done))
    vers.sort(reverse=True)
    nxt = [i for h, items in secs if h in ('Later', 'Hearth Browser, next') for i in items]
    nxt = [re.sub(r'^\[[ x]\]\s*', '', i) for i in nxt]
    body = [h2('latest', f'Hearth OS {vers[0][1]}' + ' <span class="ver">Latest</span>')]
    body.append('<ul>' + ''.join(f'<li>{md_inline(i)}</li>' for i in vers[0][2]) + '</ul>')
    body.append('<p>Installed systems update themselves; see <a href="updates.html">updates</a>.</p>')
    if nxt:
        body.append(h2('next', 'Coming next'))
        body.append('<p>Planned, in no fixed order. Something missing? <a href="requests.html">Ask for it</a>.</p>')
        body.append('<ul>' + ''.join(f'<li>{md_inline(i)}</li>' for i in nxt) + '</ul>')
    body.append(h2('earlier', 'Earlier versions'))
    for _, v, items in vers[1:]:
        body.append(f'<details class="q" id="v{v}"><summary>Hearth OS {v} <span class="cnt">{len(items)} change{"s" if len(items) != 1 else ""}</span></summary>'
                    '<div class="a"><ul>' + ''.join(f'<li>{md_inline(i)}</li>' for i in items) + '</ul></div></details>')
    return '\n'.join(body)


ASK_CSS = '''
/* "Still stuck? Tell us" */
.ask{display:grid;gap:12px;background:var(--panel);border:1px solid var(--line2);border-radius:var(--r);padding:18px;margin:14px 0 8px}
.ask label{display:grid;gap:5px;font-size:13.5px;font-weight:600;color:var(--text)}
.ask .req,.ask .opt{font-weight:400;color:var(--muted)}
.ask input,.ask select,.ask textarea{font:inherit;font-size:14.5px;font-weight:400;color:var(--text);background:#0f0a07;border:1px solid var(--line2);border-radius:9px;padding:9px 11px;width:100%}
.ask textarea{resize:vertical;line-height:1.5}
.ask input:focus,.ask select:focus,.ask textarea:focus{outline:2px solid rgba(255,138,61,.6);outline-offset:0;border-color:var(--ember)}
.ask .row2{display:grid;grid-template-columns:1fr 1fr;gap:12px}
.ask .send{display:flex;align-items:center;gap:12px;flex-wrap:wrap}
.ask button{font:inherit;font-weight:700;font-size:15px;color:var(--ink);background:var(--ember);border:0;border-radius:999px;padding:10px 18px;cursor:pointer}
.ask button:hover{background:#ff9d5a}
.ask button:disabled{opacity:.6;cursor:default}
.ask .msg{font-size:14px;color:var(--soft)}
.ask .msg b{color:var(--flame);font-size:15px;letter-spacing:.04em}
.ask .msg.err{color:#ff9b8f}
@media (max-width:600px){ .ask .row2{grid-template-columns:1fr} }
'''

ASK_JS = '''
// "Still stuck? Tell us" (troubleshooting): sent to the Hearth OS counter's /v1/logs as kind "docs", so it lands with
// the device logs (a file and an issue in the private logs repository). Answers a code to quote.
(function(){
  var f = document.getElementById('ask'); if(!f) return;
  var API = 'https://hearth-counter.game305-cl.workers.dev';
  f.addEventListener('submit', function(e){
    e.preventDefault();
    var msg = f.querySelector('.msg'), btn = f.querySelector('button'), v = function(n){ return (f.elements[n].value || '').trim(); };
    msg.className = 'msg';
    if(v('what').length < 10){ msg.className = 'msg err'; msg.textContent = 'Say a bit more about what went wrong.'; f.elements.what.focus(); return; }
    var body = 'Hearth logs: a problem sent from hearthos.fun/docs\\n\\n'
      + 'Page: ' + location.pathname + '\\n'
      + 'Edition: ' + (v('edition') || 'not sure') + '\\n'
      + 'Version: ' + (v('version') || '?') + '\\n'
      + 'Device: ' + (v('device') || '?') + '\\n'
      + 'Log code: ' + (v('code') || 'none') + '\\n'
      + 'Contact: ' + (v('contact') || 'none') + '\\n'
      + 'Browser: ' + navigator.userAgent + '\\n\\n'
      + '===== What went wrong =====\\n' + v('what') + '\\n\\n'
      + '===== What they tried =====\\n' + (v('tried') || '(nothing said)') + '\\n';
    var note = (v('edition') ? v('edition') + ': ' : '') + v('what').replace(/\\s+/g, ' ').slice(0, 300);
    btn.disabled = true; msg.textContent = 'Sending…';
    fetch(API + '/v1/logs', {method: 'POST', body: body, headers: {'content-type': 'text/plain; charset=utf-8',
        'x-hearth-kind': 'docs', 'x-hearth-version': (v('version') || 'website').replace(/[^\\w.+-]/g, '').slice(0, 20) || 'website',
        'x-hearth-note': encodeURIComponent(note)}})
      .then(function(r){ return r.json().then(function(j){ return {ok: r.ok, j: j}; }); })
      .then(function(x){
        if(x.ok && x.j.code){ msg.innerHTML = 'Sent. Your code: <b></b>. Quote it if you follow up.'; msg.querySelector('b').textContent = x.j.code; f.reset(); }
        else { msg.className = 'msg err'; msg.textContent = (x.j && x.j.error === 'busy, try again later') ? 'Lots of reports right now: try again in an hour.' : 'That didn\\'t send. Try again in a moment.'; }
      })
      .catch(function(){ msg.className = 'msg err'; msg.textContent = 'That didn\\'t send (no connection?). Try again in a moment.'; })
      .then(function(){ btn.disabled = false; });
  });
})();
'''

os.makedirs(OUT, exist_ok=True)
for slug, body in [('index', INDEX), ('install', INSTALL), ('updates', UPDATES), ('using', USING), ('troubleshooting', TROUBLE), ('requests', REQUESTS), ('releases', releases_body())]:
    t = next(p for p in PAGES if p[0] == slug)
    lead = {'index': 'Everything about installing, updating and using Hearth OS, and what to do when something goes wrong.',
            'install': 'Download Hearth OS, write it to a USB stick, and install it on its own drive or next to Windows.',
            'updates': 'Hearth OS updates itself in one piece. Here is how that works, how to go back, and how to switch edition.',
            'using': 'Game Mode, the desktop, the controller and keyboard, the power button, and sending logs.',
            'troubleshooting': 'Fixes for known problems, how to get unstuck, and how to tell us when nothing here helps.',
            'requests': 'Ideas for Hearth OS, changes you\'d like, and editions on other Linux: ask here.',
            'releases': 'What changed in each version of Hearth OS, newest first, and what\'s planned next.'}[slug]
    with open(os.path.join(OUT, f'{slug}.html'), 'w') as f:
        f.write(page(slug, t[1] if slug != 'index' else 'Hearth OS documentation', lead, body))
css = open(os.path.join(OUT, 'docs.css')).read()
if 'Still stuck? Tell us' not in css:
    open(os.path.join(OUT, 'docs.css'), 'a').write(ASK_CSS)
js = open(os.path.join(OUT, 'docs.js')).read()
if 'Requests page:' not in js:
    open(os.path.join(OUT, 'docs.js'), 'a').write(REQ_JS)
    js = open(os.path.join(OUT, 'docs.js')).read()
if 'Still stuck? Tell us' not in js:
    open(os.path.join(OUT, 'docs.js'), 'a').write(ASK_JS)
print('built', sorted(os.listdir(OUT)))
