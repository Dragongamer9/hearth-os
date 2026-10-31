// Hearth Files: KDE Dolphin with Hearth OS's changes (docs/KDE-APPS.md). Dolphin's main.cpp calls this once, just
// before it registers its KAboutData (patches/0001-hearth-files-setup.patch).
//
// SPDX-License-Identifier: GPL-2.0-or-later
#pragma once

class KAboutData;

// Hearth's name and credits (KDE's stay), the Hearth look where the desktop doesn't already give it, and the
// controller (hearthgamepad.h).
void hearthFilesSetup(KAboutData &about);
