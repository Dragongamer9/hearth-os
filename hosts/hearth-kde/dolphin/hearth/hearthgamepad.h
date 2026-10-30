// SPDX-License-Identifier: GPL-2.0-or-later
//
// Hearth Files with a game controller. Reads the controllers in /dev/input (the session's user may: logind gives the
// seat's user access to joysticks, as Hearth's hearth-pad uses them) only while a Hearth Files window is the active
// one, and turns buttons into Dolphin's own actions:
//
//   D-pad      move between files (arrow keys; held: repeats)    A      open
//   B          back (Esc while a menu, dialog or rename is open)  X      the file's menu (Menu key)
//   Y          filter the folder (type to narrow it down)         Menu   Dolphin's menu
//   LB / RB    back / forward                                     LT/RT  previous / next tab
//   View       Places (Hearth Storage, drives, folders) and back to the files
//
// In Hearth Classic, Hearth's controller helper (desktop/shell/hearth-pad) is running and already types arrows,
// Enter, Esc and the Menu key for Hearth Files (Pad.qml's keysContext), and Y opens the on-screen keyboard: then only
// LB/RB, LT/RT and View are done here, so nothing happens twice.
#pragma once

namespace HearthGamepad
{
void install();
}
