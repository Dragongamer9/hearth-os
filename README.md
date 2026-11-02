# Hearth Classic (left) and Hearth Plasma (right)

## desktop

![desktop.png](compare/desktop.png)

## files

![files.png](compare/files.png)

## launcher

![launcher.png](compare/launcher.png)

## quick-access

![quick-access.png](compare/quick-access.png)

## Hearth Plasma with a controller (tools/e2e-plasma-pad.py)

- KWin answers through hearth-pad (org.hearth.Pad): FAILED. first probe: None
- right stick moves the pointer: FAILED. cursor None -> None
- View shows the controls: passed. 
- Guide + X asks for the on-screen keyboard: passed. events: [{'t': 'radial', 'open': True, 'sel': -1}, {'t': 'radial', 'open': False, 'pick': -1}, {'t': 'osk', 'open': True}]
- Menu opens Plasma's launcher: FAILED. in front: None
- D-pad and A start an app from the launcher: FAILED. the test app did not start; pad.log says what was in front
- the Hearth menu opens Hearth Quick Access: FAILED. picked: True, in front: None
- B closes it: FAILED. in front: None
- View + B closes the window: passed. in front before: None
- Hearth Files (Dolphin) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True
- Terminal (Konsole) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True
- Text (Kate) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True
- Reader (Okular) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True
- Calculator (KCalc) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True
- Capture (Spectacle) with the controller: FAILED. in front: None, keys: False, keyboard: True, closed with View + B: True

![pad-1-pointer.png](pad-1-pointer.png)

![pad-2-launcher.png](pad-2-launcher.png)

![pad-3-launcher-dpad.png](pad-3-launcher-dpad.png)

![pad-4-hearth-menu.png](pad-4-hearth-menu.png)

![pad-5-quick-access.png](pad-5-quick-access.png)

![pad-6-quick-access-dpad.png](pad-6-quick-access-dpad.png)

![pad-7-window.png](pad-7-window.png)

## Hearth Plasma

![plasma-1-desktop.png](plasma-1-desktop.png)

![plasma-2-quicksettings.png](plasma-2-quicksettings.png)

![plasma-3-link.png](plasma-3-link.png)

![plasma-4-launcher.png](plasma-4-launcher.png)

![plasma-5-storage.png](plasma-5-storage.png)

![plasma-7-lockscreen.png](plasma-7-lockscreen.png)

