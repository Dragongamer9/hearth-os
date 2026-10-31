// Hearth GNOME: Hearth Storage in Quick Settings (docs/GNOME.md), as Hearth Plasma's org.hearth.storage and Mantel's
// Settings › Hearth Link › Hearth Storage: the toggle uses this device as Hearth Storage (a home server for your paired
// Hearth devices); the menu lists the servers and devices (online, battery, free space, last backup), the shared
// folders, Back up now and the settings. Through Mantel's helper, desktop/shell/hearth-link-settings.
import GObject from 'gi://GObject';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

function bytes(n) {
    const u = ['B', 'KB', 'MB', 'GB', 'TB'];
    let i = 0;
    let v = Number(n) || 0;
    while (v >= 1000 && i < u.length - 1) {
        v /= 1000;
        i++;
    }
    return `${i ? v.toFixed(1) : v} ${u[i]}`;
}

function devLine(d) {
    const b = d.battery;
    return [d.me ? 'This device' : d.online ? 'Online' : 'Off',
        b && b.percent !== undefined && b.percent !== null ? `Battery ${b.percent}%` : '',
        d.free !== undefined && d.free !== null ? `${bytes(d.free)} free` : '',
        d.lastBackup ? `Backed up ${new Date(d.lastBackup).toLocaleDateString()}` : ''].filter(x => x).join(' · ');
}

const StorageToggle = GObject.registerClass(
class StorageToggle extends QuickMenuToggle {
    _init() {
        super._init({title: 'Hearth Storage', gicon: Hearth.icon('hearth-disks', 'drive-harddisk-symbolic'), toggleMode: false});
        this.st = null;
        this.devs = null;
        this.message = '';
        this.menu.setHeader(Hearth.icon('hearth-disks', 'drive-harddisk-symbolic'), 'Hearth Storage',
            'A home server for your paired Hearth devices');
        this._list = new PopupMenu.PopupMenuSection();
        this.menu.addMenuItem(this._list);
        this.menu.addMenuItem(new PopupMenu.PopupSeparatorMenuItem());
        this._backup = this.menu.addAction('Back up now', () => this.act(['backup-now'], 'Backing up to your Hearth Storage'));
        this.menu.addAction('Hearth Storage settings', () => {
            Main.panel.statusArea.quickSettings.menu.close();
            Hearth.settings('link');
        });
        this.connect('clicked', () => {
            const stc = this._stc();
            if (stc)
                this.act(['storage-set', 'server', stc.server ? 'off' : 'on']);
        });
        this.menu.connect('open-state-changed', (m, open) => {
            if (open)
                this.refresh();
        });
    }

    _stc() {
        // As Hearth Plasma's widget: status → storage.config.
        return this.st && !this.st.error && this.st.storage ? (this.st.storage.config || {}) : null;
    }

    async refresh() {
        this.st = await Hearth.link('status');
        if (this._stc())
            this.devs = await Hearth.link('devices');
        this._draw();
    }

    async act(args, done) {
        const r = await Hearth.link(...args);
        this.message = r && r.error ? r.error : (done || '');
        await this.refresh();
    }

    _draw() {
        const stc = this._stc();
        this.checked = !!(stc && stc.server);
        this.subtitle = !stc ? 'Needs Hearth Link' : stc.server ? 'This device' : 'Off';
        this._list.removeAll();
        const line = (text, cls) => {
            const item = new PopupMenu.PopupMenuItem(text, {reactive: false, can_focus: false});
            if (cls)
                item.label.add_style_class_name(cls);
            this._list.addMenuItem(item);
        };
        if (!stc)
            line("Hearth Storage works through Hearth Link, which isn't running.", 'hearth-dim');
        const devs = this.devs && Array.isArray(this.devs.devices) ? this.devs.devices : [];
        if (this.devs && this.devs.serverOffline)
            line(`${this.devs.serverName || 'Your Hearth Storage'} isn't answering (asleep or away).`, 'hearth-dim');
        for (const d of stc ? devs : [])
            line(`${d.name}${d.server ? ' · Hearth Storage' : ''}: ${devLine(d)}`);
        for (const s of stc && stc.server ? (stc.shares || []) : [])
            line(`${s.name} · ${s.path}`, 'hearth-dim');
        if (this.message)
            line(this.message, 'hearth-dim');
        this._backup.visible = !!stc && !stc.server;
    }
});

const Indicator = GObject.registerClass(
class Indicator extends SystemIndicator {
    _init() {
        super._init();
        this.toggle = new StorageToggle();
        this.quickSettingsItems.push(this.toggle);
    }
});

export default class HearthStorage extends Extension {
    enable() {
        this._indicator = new Indicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        this._indicator.toggle.refresh();
        this._stop = Hearth.every(120000, () => this._indicator?.toggle.refresh());
    }

    disable() {
        this._stop?.();
        this._stop = null;
        this._indicator?.quickSettingsItems.forEach(i => i.destroy());
        this._indicator?.destroy();
        this._indicator = null;
    }

    openMenu() {
        Main.panel.statusArea.quickSettings.menu.open();
        this._indicator?.toggle.menu.open();
    }
}
