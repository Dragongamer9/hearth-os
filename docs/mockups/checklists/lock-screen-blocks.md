# The lock screen made of blocks (approved Nov 24, 2026)

The owner, Nov 23: "finish the moduler stuff as well. Might as well make the lock screen moduler as well"; Nov 24:
"Lock screen mockups were good add that to 1.18". Mantel's lock screen built from the same blocks as Build anything
(hearth-layout), with the same editor and Describe it. Sources: hearth-workspace
`hearth/mockups/2026-11-23-lock-screen-blocks`.

Mockups: lock-screen-blocks/lock-1-made-of-blocks.png, lock-screen-blocks/lock-2-in-the-1995-look.png, lock-screen-blocks/lock-3-the-editor.png
Release: 1.18

## 1. Made of blocks
- [x] Big clock with the date under it — hosts/linux/desktop/shell/LockBlocks.qml: "Qt.formatDate(lb.now, \"dddd, d MMMM\")"
- [x] Weather block: place, temperature, tonight's line — hosts/linux/desktop/shell/LockBlocks.qml: "lb.weatherWord(wx.now.days[0].code) + \" tonight\""
- [x] Media block (Now playing): title, artist, position of length, previous, pause and next — hosts/linux/desktop/shell/LockBlocks.qml: "lb.mmss(md.pos) + \" of \" + lb.mmss(md.pl.length)"
- [x] While you were away block: names only, no messages shown (your choice), "N more" — hosts/linux/desktop/shell/LockBlocks.qml: "\"names only, no messages shown (your choice)\""
- [x] Your downloads block: name, percent and a bar — hosts/linux/desktop/shell/LockBlocks.qml: "Ink { text: \"Your downloads\""
- [x] Battery block: this device's percent and time left, and the controller's battery — hosts/linux/desktop/shell/LockBlocks.qml: "bt.pads.map(p => (p.model || \"Controller\")"
- [x] Password block: your picture and name, password box with its arrow, "Fingerprint, or your password" — hosts/linux/desktop/shell/LockPassword.qml: "property string hint: \"Fingerprint, or your password\""
- [x] The password block is always there and always on top, and can't be removed (its "always on top · can't be removed" tag) — hosts/linux/desktop/shell/LockPassword.qml: "text: \"always on top · can't be removed\""
- [x] The line along the bottom saying what the blocks are — hosts/linux/desktop/shell/LockBlocks.qml: "\"Lock screen made of blocks, like your desktop\""
- [x] Nothing on the lock screen can open an app, a file or a setting; only the password block unlocks — hosts/linux/desktop/shell/hearth-lockscreen: "nothing on the lock screen opens anything, so what it did when pressed was left out"

## 2. In the 1995 look
- [x] "Hearth is locked" bar with the time — hosts/linux/desktop/shell/lock-screens/retro.json: "\"value\": \"Hearth is locked\""
- [x] The look's desktop icons (My Hearth, Bin), shown only — hosts/linux/desktop/shell/LockBlocks.qml: "// icons: the look's desktop icons, shown only"
- [x] "Welcome back to Hearth" window: picture, "Type your password to unlock this PC.", the password box, OK and Shut down... — hosts/linux/desktop/shell/LockPassword.qml: "\"Type your password to unlock this PC.\""
- [x] The same safety underneath: the password block drawn in that look — hosts/linux/desktop/shell/Lock.qml: "item.unlock.connect(() => root.tryUnlock());"
- [x] Every ready-made desktop gets a matching lock screen — hosts/linux/desktop/shell/LockBlocks.qml: "\"/lock-screens/\" + lb.readyId + \".json\""

## 3. The editor
- [x] "Your lock screen" side: Blocks · drag onto it (Clock, Date, Weather, Media, Notifications, Downloads, Battery, Calendar, Picture, Words, Friends online, Your AI's own...) — hosts/linux/desktop/shell/ScreenEditor.qml: "{ b: \"friends\", t: \"Friends online\" }"
- [x] Always there: Password and fingerprint, with its line (moved, sized and styled, never removed or covered) — hosts/linux/desktop/shell/ScreenEditor.qml: "It can be moved, sized and styled, never removed or covered. Unlocking is Hearth's own, whatever the look."
- [x] Match your desktop (Same as <your desktop>) — hosts/linux/desktop/shell/ScreenEditor.qml: "text: \"Same as \" + ed.deskName"
- [x] Top: "Lock screen" with the layout's name, Undo, Try it (locks for real), Keep this — hosts/linux/desktop/shell/ScreenEditor.qml: "{ t: \"Try it (locks for real)\", a: \"locktry\" }"
- [x] The lock screen on a grid, the picked block outlined — hosts/linux/desktop/shell/ScreenEditor.qml: "// lock mode: the lock screen as it will look (LockBlocks.qml at full size, scaled down), its password block too"
- [x] Or say it: a box that changes the lock screen from words (Describe it) — hosts/linux/desktop/shell/ScreenEditor.qml: "put the weather under the clock and hide my notifications"
- [x] The picked block's side: Shows (for Weather: Place, Tonight too), Look (Corners, See-through, Font), Privacy (Notifications names only, Media controls) — hosts/linux/desktop/shell/ScreenEditor.qml: "Kv { label: \"Media controls\";"
- [x] The line: nothing on the lock screen can open an app, a file or a setting — hosts/linux/desktop/shell/ScreenEditor.qml: "\"Nothing on the lock screen can open an app, a file or a setting. Blocks only show; the password block unlocks.\""
