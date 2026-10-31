// Hearth GNOME's Hearth menu (Guide on the controller), Mantel's radial menu (RadialMenu.qml) drawn by GNOME Shell:
// eight items around a glowing centre. hearth-pad picks with either stick, the D-pad or LB / RB, A runs the item,
// B or Guide closes it; the pointer and touch work too.
import Clutter from 'gi://Clutter';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const SIZE = 480;
const ITEM = 104;

export const ITEMS = [
    {action: 'apps', label: 'Apps', icon: 'view-app-grid-symbolic'},
    {action: 'keyboard', label: 'Keyboard', icon: 'input-keyboard-symbolic'},
    {action: 'quick', label: 'Quick Settings', icon: 'open-menu-symbolic'},
    {action: 'close', label: 'Close window', icon: 'window-close-symbolic'},
    {action: 'game', label: 'Game Mode', icon: 'input-gaming-symbolic'},
    {action: 'notifications', label: 'Notifications', icon: 'preferences-system-notifications-symbolic'},
    {action: 'overview', label: 'Overview', icon: 'focus-windows-symbolic'},
    {action: 'mouse', label: 'Controller mouse', icon: 'input-mouse-symbolic'},
];

export class HearthRadial {
    constructor(onPick, onDismiss) {
        this.onPick = onPick;           // (action)
        this.onDismiss = onDismiss;     // closed with the pointer / touch
        this.open = false;
        this.sel = -1;
        this.actor = new St.Widget({style_class: 'hearth-radial', reactive: true, visible: false, width: SIZE, height: SIZE});
        this._centre = new St.Label({style_class: 'hearth-radial-centre', text: 'Hearth'});
        this.actor.add_child(this._centre);
        this._items = ITEMS.map((it, i) => {
            const b = new St.Button({style_class: 'hearth-radial-item', can_focus: false, reactive: true, width: ITEM, height: ITEM});
            const box = new St.BoxLayout({vertical: true, x_align: Clutter.ActorAlign.CENTER, y_align: Clutter.ActorAlign.CENTER});
            box.add_child(new St.Icon({gicon: Hearth.icon(it.icon, 'image-missing'), icon_size: 32, x_align: Clutter.ActorAlign.CENTER}));
            b._label = new St.Label({text: it.label, style_class: 'hearth-radial-label', x_align: Clutter.ActorAlign.CENTER});
            box.add_child(b._label);
            b.set_child(box);
            const a = i * Math.PI / 4;
            b.set_position(Math.round(SIZE / 2 + Math.sin(a) * SIZE * 0.36 - ITEM / 2), Math.round(SIZE / 2 - Math.cos(a) * SIZE * 0.36 - ITEM / 2));
            b.connect('clicked', () => {
                this.hide();
                this.onDismiss();
                this.onPick(it.action);
            });
            b.connect('notify::hover', () => {
                if (b.hover)
                    this.select(i);
            });
            b.track_hover = true;
            this.actor.add_child(b);
            return b;
        });
        this._centre.set_position(SIZE / 2 - 90, SIZE / 2 - 14);
        this._centre.width = 180;
    }

    setLabel(action, label) {
        const i = ITEMS.findIndex(x => x.action === action);
        if (i >= 0)
            this._items[i]._label.text = label;
    }

    show() {
        if (this.open)
            return;
        const m = Main.layoutManager.primaryMonitor;
        if (!m)
            return;
        this.open = true;
        this.select(-1);
        this.actor.set_position(Math.round(m.x + (m.width - SIZE) / 2), Math.round(m.y + (m.height - SIZE) / 2));
        Main.layoutManager.addTopChrome(this.actor);
        this.actor.show();
    }

    hide() {
        if (!this.open)
            return;
        this.open = false;
        this.actor.hide();
        Main.layoutManager.removeChrome(this.actor);
    }

    select(i) {
        this.sel = i;
        this._items.forEach((b, j) => {
            if (j === i)
                b.add_style_pseudo_class('checked');
            else
                b.remove_style_pseudo_class('checked');
        });
        this._centre.text = i >= 0 ? ITEMS[i].label : 'Hearth';
    }

    /** hearth-pad's radial {open, sel, pick}. */
    update(m) {
        if (m.open) {
            this.show();
            this.select(typeof m.sel === 'number' ? m.sel : -1);
            return;
        }
        this.hide();
        if (typeof m.pick === 'number' && m.pick >= 0 && ITEMS[m.pick])
            this.onPick(ITEMS[m.pick].action);
    }

    destroy() {
        this.hide();
        this.actor.destroy();
    }
}
