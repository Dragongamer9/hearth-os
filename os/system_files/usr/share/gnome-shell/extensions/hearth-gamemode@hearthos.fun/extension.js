// Hearth GNOME: Mantel's Game Mode button (Bar.qml "Back to Game Mode", Hearth Plasma's org.hearth.gamemode): the
// "Game Mode" pill on the accent colour near the end of the bar, and Super+G. Both run nimbus-mode game, which ends
// GNOME through its own sign-out and starts Game Mode in the same session. docs/GNOME.md.
import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import Meta from 'gi://Meta';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const Pill = GObject.registerClass(
class Pill extends PanelMenu.Button {
    _init() {
        super._init(0.0, 'Game Mode', true);
        this.add_style_class_name('hearth-gamemode');
        this.accessible_name = 'Game Mode';
        this.add_child(new St.Label({text: 'Game Mode', style_class: 'hearth-gamemode-pill', y_align: Clutter.ActorAlign.CENTER}));
    }

    vfunc_event(event) {
        const t = event.type();
        if (t === Clutter.EventType.BUTTON_RELEASE || t === Clutter.EventType.TOUCH_END ||
            (t === Clutter.EventType.KEY_PRESS && [Clutter.KEY_Return, Clutter.KEY_KP_Enter, Clutter.KEY_space].includes(event.get_key_symbol()))) {
            Hearth.gameMode();
            return Clutter.EVENT_STOP;
        }
        return Clutter.EVENT_PROPAGATE;
    }
});

export default class HearthGameMode extends Extension {
    enable() {
        this._pill = new Pill();
        // Before the show-desktop strip (hearth-bar@hearthos.fun keeps that last), after the clock.
        Main.panel.addToStatusArea('hearth-gamemode', this._pill, Main.panel._rightBox.get_n_children(), 'right');
        // Super+G anywhere on the desktop, in the overview and over menus.
        this._action = global.display.grab_accelerator('<Super>g', Meta.KeyBindingFlags.NONE);
        if (this._action && this._action !== Meta.KeyBindingAction.NONE) {
            this._binding = Meta.external_binding_name_for_action(this._action);
            Main.wm.allowKeybinding(this._binding, Shell.ActionMode.NORMAL | Shell.ActionMode.OVERVIEW | Shell.ActionMode.POPUP);
            this._accelId = global.display.connect('accelerator-activated', (display, action) => {
                if (action === this._action)
                    Hearth.gameMode();
            });
        } else {
            console.warn('Hearth Game Mode: Super+G is taken');
        }
    }

    disable() {
        if (this._accelId)
            global.display.disconnect(this._accelId);
        this._accelId = 0;
        if (this._action && this._action !== Meta.KeyBindingAction.NONE) {
            global.display.ungrab_accelerator(this._action);
            if (this._binding)
                Main.wm.allowKeybinding(this._binding, Shell.ActionMode.NONE);
        }
        this._action = 0;
        this._binding = null;
        this._pill?.destroy();
        this._pill = null;
    }

    debugState() {
        return JSON.stringify({pill: !!this._pill, superG: !!this._action && this._action !== Meta.KeyBindingAction.NONE});
    }
}
