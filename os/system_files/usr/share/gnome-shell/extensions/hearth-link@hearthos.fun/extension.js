// Hearth GNOME: Hearth Link in Quick Settings (docs/GNOME.md), the short version of Mantel's Settings › Hearth Link
// (SetLink.qml) and Hearth Plasma's org.hearth.link: this device, the devices paired with it, a pairing code, sync now
// and Hearth Link's settings. The toggle opens / closes pairing (its code shows as the subtitle and on the bar while
// it's open). Everything goes through Mantel's helper, desktop/shell/hearth-link-settings (Hearth Link's own socket).
import GObject from 'gi://GObject';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

function code(st) {
    const c = st && st.pairing && st.pairing.code ? String(st.pairing.code) : '';
    return c ? `${c.slice(0, 3)} ${c.slice(3)}` : '';
}

const LinkToggle = GObject.registerClass(
class LinkToggle extends QuickMenuToggle {
    _init(indicator) {
        super._init({title: 'Hearth Link', gicon: Hearth.icon('hearth-share', 'network-workgroup-symbolic'), toggleMode: false});
        this._indicator = indicator;
        this.st = null;
        this.message = '';
        this.menu.setHeader(Hearth.icon('hearth-share', 'network-workgroup-symbolic'), 'Hearth Link', '');
        this._list = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._list);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._sync = this.menu.addAction('Sync now', () => this.act(['sync'], 'Syncing with your devices'));
        this.menu.addAction('Hearth Link settings', () => {
            Main.panel.statusArea.quickSettings.menu.close();
            Hearth.settings('link');
        });
        this.connect('clicked', () => this.act([this.st && this.st.pairing ? 'pair-close' : 'pair-open']));
        this.menu.connect('open-state-changed', (m, open) => {
            this._fast?.();
            this._fast = open ? Hearth.every(3000, () => this.refresh()) : null;
            if (open)
                this.refresh();
        });
    }

    async refresh() {
        const st = await Hearth.link('status');
        this.st = st;
        this._draw();
    }

    async act(args, done) {
        const r = await Hearth.link(...args);
        this.message = r && r.error ? r.error : (done || '');
        await this.refresh();
    }

    _draw() {
        const st = this.st;
        const peers = st && Array.isArray(st.peers) ? st.peers : [];
        const c = code(st);
        this.checked = !!(st && st.pairing);
        this.subtitle = !st ? 'Not running' : st.error ? 'Not running' : c ? `Code ${c}`
            : peers.length === 1 ? '1 paired device' : `${peers.length} paired devices`;
        this.menu.setHeader(Hearth.icon('hearth-share', 'network-workgroup-symbolic'), 'Hearth Link',
            st && st.me ? `This device: ${st.me.name}` : '');
        this._indicator.showCode(c);
        this._list.removeAll();
        const line = (text, cls) => {
            const item = new PopupMenu.PopupMenuItem(text, {reactive: false, can_focus: false});
            if (cls)
                item.label.add_style_class_name(cls);
            this._list.addMenuItem(item);
        };
        if (!st || st.error)
            line(st && st.error ? st.error : "Hearth Link isn't running.", 'hearth-dim');
        if (c) {
            line(`Code: ${c}`, 'hearth-code');
            if (st.me)
                line(`On the other device: Settings › Hearth Link, pick ${st.me.name} and type this code.`, 'hearth-dim');
        }
        for (const p of peers)
            line(`${p.name}${p.server ? ' · Hearth Storage' : ''} — ${p.online ? 'Online' : 'Off'}`);
        if (st && st.me && !peers.length)
            line('No devices paired yet. Pair one to share games, saves and files between your Hearth devices.', 'hearth-dim');
        if (this.message)
            line(this.message, 'hearth-dim');
        this._sync.setSensitive(peers.length > 0);
    }
});

const Indicator = GObject.registerClass(
class Indicator extends SystemIndicator {
    _init() {
        super._init();
        // On the bar only while a pairing code is showing (Hearth Plasma brings its tray icon forward the same way).
        this._icon = this._addIndicator();
        this._icon.gicon = Hearth.icon('hearth-share', 'network-workgroup-symbolic');
        this._icon.visible = false;
        this.toggle = new LinkToggle(this);
        this.quickSettingsItems.push(this.toggle);
    }

    showCode(c) {
        this._icon.visible = !!c;
    }
});

export default class HearthLink extends Extension {
    enable() {
        this._indicator = new Indicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        this._indicator.toggle.refresh();
        this._stop = Hearth.every(60000, () => this._indicator?.toggle.refresh());
    }

    disable() {
        this._stop?.();
        this._stop = null;
        this._indicator?.toggle._fast?.();
        this._indicator?.quickSettingsItems.forEach(i => i.destroy());
        this._indicator?.destroy();
        this._indicator = null;
    }

    /** For checks and screenshots (gnome-test.yml). */
    openMenu() {
        Main.panel.statusArea.quickSettings.menu.open();
        this._indicator?.toggle.menu.open();
    }

    debugState() {
        return JSON.stringify({st: this._indicator ? this._indicator.toggle.st : null});
    }
}
