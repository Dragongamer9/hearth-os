// gnome-test.yml only (never in the images). A headless GNOME Shell reads no input devices, so the test reads
// hearth-pad's uinput keyboard and pointer itself and replays what they send here, through Clutter's virtual devices
// (the same events libinput would deliver on a device). Also turns on GNOME Shell's "unsafe mode" so the test may use
// org.gnome.Shell.Eval and org.gnome.Shell.Screenshot.
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

export default class HearthCi extends Extension {
    enable() {
        try {
            global.context.unsafe_mode = true;
        } catch (e) {
            logError(e, 'hearth-ci: unsafe mode');
        }
        const seat = Clutter.get_default_backend().get_default_seat();
        const kbd = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
        const ptr = seat.create_virtual_device(Clutter.InputDeviceType.POINTER_DEVICE);
        const now = () => GLib.get_monotonic_time();
        globalThis.hearthCi = {
            key: (code, down) => kbd.notify_key(now(), code, down ? Clutter.KeyState.PRESSED : Clutter.KeyState.RELEASED),
            move: (dx, dy) => ptr.notify_relative_motion(now(), dx, dy),
            button: (code, down) => ptr.notify_button(now(), code, down ? Clutter.ButtonState.PRESSED : Clutter.ButtonState.RELEASED),
            wheel: n => ptr.notify_discrete_scroll(now(), n > 0 ? Clutter.ScrollDirection.UP : Clutter.ScrollDirection.DOWN, Clutter.ScrollSource.WHEEL),
        };
    }

    disable() {
        delete globalThis.hearthCi;
    }
}
