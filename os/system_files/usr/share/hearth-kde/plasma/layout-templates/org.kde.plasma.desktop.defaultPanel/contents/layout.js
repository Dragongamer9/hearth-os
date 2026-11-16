// KDE Plasma with Hearth inside (1.13, docs/DESKTOPS.md): KDE's own default panel at the bottom, with Hearth's start
// menu (org.hearth.kickoff) in place of Kickoff and the apps pinned the mockup shows. Only for KDE started by Hearth
// (nimbus-session puts /usr/share/hearth-kde first in XDG_DATA_DIRS); used when Plasma makes its first panel, so a
// panel you already have or changed stays as it is. The Hearth item (org.hearth.tray) is in the tray by default.
var panel = new Panel
var panelScreen = panel.screen
panel.location = "bottom"
panel.height = 2 * Math.floor(gridUnit * 2.5 / 2)
var maximumAspectRatio = 21 / 9
if (panel.formFactor === "horizontal") {
    var geo = screenGeometry(panelScreen)
    var maximumWidth = Math.ceil(geo.height * maximumAspectRatio)
    if (geo.width > maximumWidth) {
        panel.alignment = "center"
        panel.minimumLength = maximumWidth
        panel.maximumLength = maximumWidth
    }
}
panel.addWidget("org.hearth.kickoff")
var tasks = panel.addWidget("org.kde.plasma.icontasks")
tasks.currentConfigGroup = ["General"]
tasks.writeConfig("launchers", ["applications:org.kde.dolphin.desktop", "applications:firefox-esr.desktop", "applications:steam.desktop"])
panel.addWidget("org.kde.plasma.marginsseparator")
panel.addWidget("org.kde.plasma.systemtray")
panel.addWidget("org.kde.plasma.digitalclock")
panel.addWidget("org.kde.plasma.showdesktop")
