// GNOME with Hearth inside (1.13, docs/DESKTOPS.md; mockup docs/mockups/next-update/desktops-3-gnome.png): a Hearth
// section in GNOME's own quick settings (the Hearth square among the status icons in the top bar): "Hearth", a big
// orange "Return to Game Mode", then Controller, On-screen keyboard, Hearth notices, Server at home, Hearth update
// and Switch desktop ›, each with its value on the right. Wi-Fi, Bluetooth, Night Light and the rest stay GNOME's.
// Values and actions through hearth-desktops (tray / act), the same helper as KDE's Hearth tray item.
import GObject from 'gi://GObject';
import St from 'gi://St';
import Clutter from 'gi://Clutter';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {QuickSettingsItem, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const DESKTOPS = `${Hearth.SHARE}/desktop/shell/hearth-desktops`;
const ORANGE = '#f0a840';

async function desktops(...args) {
    if (!Hearth.exists(DESKTOPS))
        return null;
    return Hearth.lastJson(await Hearth.run(['python3', DESKTOPS, ...args]));
}

function closeQuickSettings() {
    Main.panel.statusArea.quickSettings?.menu?.close();
}

function act(...args) {
    closeQuickSettings();
    Hearth.spawn(['python3', DESKTOPS, 'act', ...args]);
}

const ROWS = [
    ['controller', '🎮', 'Controller'],
    ['keyboard', '⌨', 'On-screen keyboard'],
    ['notices', '🔔', 'Hearth notices'],
    ['server', '🖥', 'Server at home'],
    ['update', '⬇', 'Hearth update'],
    ['switch', '⇄', 'Switch desktop'],
];

const HearthSection = GObject.registerClass(
class HearthSection extends QuickSettingsItem {
    _init() {
        super._init({style_class: 'hearth-section', can_focus: false, reactive: false, x_expand: true});
        this.set_style('background-color: rgba(255,255,255,0.06); border-radius: 18px; padding: 14px;');
        const box = new St.BoxLayout({vertical: true, x_expand: true, style: 'spacing: 6px;'});
        this.set_child(box);
        const head = new St.BoxLayout({style: 'spacing: 10px;'});
        head.add_child(new St.Widget({style: `background-color: ${ORANGE}; border-radius: 5px; width: 22px; height: 22px;`,
            y_align: Clutter.ActorAlign.CENTER}));
        head.add_child(new St.Label({text: 'Hearth', style: 'font-weight: bold; font-size: 15pt;', y_align: Clutter.ActorAlign.CENTER}));
        box.add_child(head);
        const big = new St.Button({label: '🎮  Return to Game Mode', can_focus: true, x_expand: true,
            style: `background-color: ${ORANGE}; color: #1a1206; font-weight: bold; font-size: 13pt; border-radius: 999px; padding: 12px; margin: 6px 0;`});
        big.connect('clicked', () => act('game'));
        box.add_child(big);
        this._values = {};
        this._list = new St.BoxLayout({vertical: true, x_expand: true});
        this._switch = new St.BoxLayout({vertical: true, x_expand: true, visible: false});
        box.add_child(this._list);
        box.add_child(this._switch);
        for (const [id, glyph, title] of ROWS) {
            const row = new St.Button({can_focus: true, x_expand: true, style: 'padding: 8px 8px; border-radius: 10px;'});
            const line = new St.BoxLayout({x_expand: true});
            line.add_child(new St.Label({text: `${glyph}  ${title}`, x_expand: true, y_align: Clutter.ActorAlign.CENTER}));
            const value = new St.Label({text: '', style: 'color: rgba(255,255,255,0.65);', y_align: Clutter.ActorAlign.CENTER});
            line.add_child(value);
            row.set_child(line);
            row.connect('notify::hover', () => row.set_style(`padding: 8px 8px; border-radius: 10px;${row.hover ? ' background-color: rgba(255,255,255,0.08);' : ''}`));
            row.connect('clicked', () => (id === 'switch' ? this._showSwitch(true) : act(id)));
            this._values[id] = value;
            this._list.add_child(row);
        }
        this._t = {};
    }

    _showSwitch(on) {
        this._list.visible = !on;
        this._switch.visible = on;
        this._switch.destroy_all_children();
        if (!on)
            return;
        this._switch.add_child(new St.Label({text: 'Switch desktop', style: 'font-weight: bold; padding: 4px 8px;'}));
        for (const d of this._t.desktops || []) {
            const here = this._t.now === d.id;
            const b = new St.Button({label: d.name + (here ? '  (using now)' : ''), reactive: !here, can_focus: !here, x_align: Clutter.ActorAlign.START,
                style: 'padding: 8px; border-radius: 10px;'});
            b.connect('clicked', () => act('switch', d.id));
            this._switch.add_child(b);
        }
        const more = new St.Button({label: 'Get KDE Plasma, or remove one… (Settings)', x_align: Clutter.ActorAlign.START, style: 'padding: 8px; border-radius: 10px;'});
        more.connect('clicked', () => act('settings'));
        this._switch.add_child(more);
        const back = new St.Button({label: '‹ Back', x_align: Clutter.ActorAlign.START, style: 'padding: 8px; border-radius: 10px;'});
        back.connect('clicked', () => this._showSwitch(false));
        this._switch.add_child(back);
    }

    show(t) {
        this._t = t || {};
        const v = {controller: t.controller || '', keyboard: t.keyboard || 'shows when you type',
            notices: (t.noticesNew ? `${t.noticesNew} new · ` : '') + "in GNOME's notifications",
            server: t.server || '', update: t.update || '', switch: `${t.switch || 'Mantel · KDE · GNOME'} ›`};
        for (const [id, label] of Object.entries(this._values))
            label.text = v[id] || '';
    }
});

const Indicator = GObject.registerClass(
class Indicator extends SystemIndicator {
    _init() {
        super._init();
        // The Hearth square among the status icons.
        this.add_child(new St.Widget({style: `background-color: ${ORANGE}; border-radius: 3px; width: 14px; height: 14px;`,
            y_align: Clutter.ActorAlign.CENTER}));
        this.section = new HearthSection();
        this.quickSettingsItems.push(this.section);
    }

    async refresh() {
        const t = await desktops('tray');
        if (t && !t.error)
            this.section.show(t);
    }
});

export default class HearthQuick extends Extension {
    enable() {
        this._indicator = new Indicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator, 2);
        const menu = Main.panel.statusArea.quickSettings.menu;
        this._openId = menu.connect('open-state-changed', (m, open) => {
            if (open)
                this._indicator?.refresh();
            else
                this._indicator?.section._showSwitch(false);
        });
        this._indicator.refresh();
    }

    disable() {
        if (this._openId)
            Main.panel.statusArea.quickSettings.menu.disconnect(this._openId);
        this._openId = 0;
        this._indicator?.quickSettingsItems.forEach(i => i.destroy());
        this._indicator?.destroy();
        this._indicator = null;
    }
}
