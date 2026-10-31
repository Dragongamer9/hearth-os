// SPDX-License-Identifier: GPL-2.0-or-later
#include "hearthfiles.h"
#include "hearthgamepad.h"

#include <KAboutData>

#include <QApplication>
#include <QColor>
#include <QFile>
#include <QHash>
#include <QPalette>
#include <QStandardPaths>
#include <QTextStream>

namespace
{

using Scheme = QHash<QString, QHash<QString, QColor>>;

// A KDE color scheme file (Hearth.colors: [Colors:View] BackgroundNormal=38,29,24 ...).
Scheme readScheme(const QString &path)
{
    Scheme s;
    QFile f(path);
    if (!f.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return s;
    }
    QTextStream in(&f);
    QString group;
    while (!in.atEnd()) {
        const QString line = in.readLine().trimmed();
        if (line.startsWith(QLatin1Char('[')) && line.endsWith(QLatin1Char(']'))) {
            group = line.mid(1, line.size() - 2);
            continue;
        }
        const int eq = line.indexOf(QLatin1Char('='));
        if (eq <= 0 || !group.startsWith(QLatin1String("Colors:"))) {
            continue;
        }
        const QStringList rgb = line.mid(eq + 1).split(QLatin1Char(','));
        if (rgb.size() < 3) {
            continue;
        }
        s[group][line.left(eq).trimmed()] = QColor(rgb[0].trimmed().toInt(), rgb[1].trimmed().toInt(), rgb[2].trimmed().toInt());
    }
    return s;
}

// The palette KDE's own platform theme would make from the scheme (the roles Dolphin's widgets use).
void applyHearthPalette()
{
    // Plasma (Hearth Plasma sets the Hearth scheme for the whole desktop) and Hearth Classic (qt6ct with Hearth's
    // palette, desktop/theme/apply.sh) already give these colors, and keep your own choice if you change it there.
    const QByteArray desktop = qgetenv("XDG_CURRENT_DESKTOP");
    const QByteArray theme = qgetenv("QT_QPA_PLATFORMTHEME");
    if (desktop.contains("KDE") || theme.startsWith("qt6ct") || theme == "kde" || qEnvironmentVariableIsSet("HEARTH_FILES_SYSTEM_COLORS")) {
        return;
    }
    const QString path = QStandardPaths::locate(QStandardPaths::GenericDataLocation, QStringLiteral("color-schemes/Hearth.colors"));
    if (path.isEmpty()) {
        return;
    }
    const Scheme s = readScheme(path);
    auto c = [&s](const char *group, const char *key) {
        return s.value(QLatin1String("Colors:") + QLatin1String(group)).value(QLatin1String(key));
    };
    QPalette p = QApplication::palette();
    auto set = [&p](QPalette::ColorRole role, const QColor &col) {
        if (col.isValid()) {
            p.setColor(QPalette::Active, role, col);
            p.setColor(QPalette::Inactive, role, col);
            p.setColor(QPalette::Disabled, role, col);
        }
    };
    set(QPalette::Window, c("Window", "BackgroundNormal"));
    set(QPalette::WindowText, c("Window", "ForegroundNormal"));
    set(QPalette::Base, c("View", "BackgroundNormal"));
    set(QPalette::AlternateBase, c("View", "BackgroundAlternate"));
    set(QPalette::Text, c("View", "ForegroundNormal"));
    set(QPalette::Button, c("Button", "BackgroundNormal"));
    set(QPalette::ButtonText, c("Button", "ForegroundNormal"));
    set(QPalette::Highlight, c("Selection", "BackgroundNormal"));
    set(QPalette::HighlightedText, c("Selection", "ForegroundNormal"));
    set(QPalette::ToolTipBase, c("Tooltip", "BackgroundNormal"));
    set(QPalette::ToolTipText, c("Tooltip", "ForegroundNormal"));
    set(QPalette::Link, c("View", "ForegroundLink"));
    set(QPalette::PlaceholderText, c("View", "ForegroundInactive"));
    const QColor dim = c("View", "ForegroundInactive");
    if (dim.isValid()) {
        p.setColor(QPalette::Disabled, QPalette::Text, dim);
        p.setColor(QPalette::Disabled, QPalette::WindowText, dim);
        p.setColor(QPalette::Disabled, QPalette::ButtonText, dim);
    }
    QApplication::setPalette(p);
}

} // namespace

void hearthFilesSetup(KAboutData &about)
{
    // Hearth's name; Dolphin's authors, licence and KDE's credits are all kept.
    about.setDisplayName(QStringLiteral("Hearth Files"));
    about.setShortDescription(QStringLiteral("Hearth OS's file manager, built from KDE's Dolphin"));
    about.setOtherText(QStringLiteral(
        "Hearth Files is KDE's Dolphin file manager with Hearth OS's changes: Hearth Storage in Places (hearth:/), "
        "controller navigation and the Hearth look. Dolphin is made by the Dolphin developers and the KDE community "
        "and is free software under the GNU General Public License; so is Hearth Files. "
        "Hearth's changes: https://github.com/Dragongamer9/hearth-os (hosts/hearth-kde). "
        "Dolphin: https://apps.kde.org/dolphin"));
    about.addCredit(QStringLiteral("The KDE community"), QStringLiteral("Dolphin, KIO and KDE Frameworks, which Hearth Files is built from"),
                    QString(), QStringLiteral("https://kde.org"));
    applyHearthPalette();
    HearthGamepad::install();
}
