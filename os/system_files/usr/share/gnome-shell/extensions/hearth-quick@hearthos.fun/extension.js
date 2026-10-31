// Hearth GNOME: Hearth's additions to GNOME's Quick Settings (docs/GNOME.md), what Mantel's Quick Access and Hearth
// Plasma's org.hearth.quicksettings have that GNOME's own panel doesn't: Game Mode, Hearth Settings, and the power mode
// through hearth-power (power-profiles-daemon, SteamOS Manager or Handheld Daemon: handhelds' TDP modes too). Volume,
// brightness, Wi-Fi, Bluetooth, the screenshot button and power stay GNOME's own. Values come from hearth-gnome quick
// (the same helper as Hearth Plasma's widget).
import GObject from 'gi://GObject';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {QuickToggle, QuickMenuToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const PROFILES = [['power-saver', 'Saver', 'power-profile-power-saver-symbolic'],
    ['balanced', 'Balanced', 'power-profile-balanced-symbolic'],
    ['performance', 'Performance', 'power-profile-performance-symbolic']];

function closeQuickSettings() {
    Main.panel.statusArea.quickSettings?.menu?.close();
}

const ActionToggle = GObject.registerClass(
class ActionToggle extends QuickToggle {
    _init(title, iconName, run) {
        super._init({title, gicon: Hearth.icon(iconName, 'system-run-symbolic'), toggleMode: false});
        this.add_style_class_name('hearth-quick-action');
        this.connect('clicked', () => {
            closeQuickSettings();
            run();
        });
    }
});

const PowerMode = GObject.registerClass(
class PowerMode extends QuickMenuToggle {
    _init() {
        super._init({title: 'Power mode', iconName: 'power-profile-balanced-symbolic', toggleMode: false});
        this.menu.setHeader('power-profile-balanced-symbolic', 'Power mode', 'Through Hearth (handhelds too)');
        this._items = {};
        for (const [id, label, icon] of PROFILES) {
            const item = new PopupMenu.PopupImageMenuItem(label, icon);
            item.connect('activate', () => this.setProfile(id));
            this._items[id] = item;
            this.menu.addMenuItem(item);
        }
        this.visible = false;
        this.connect('clicked', () => {
            // Click: the next mode, as GNOME's own toggle does.
            const ids = PROFILES.map(p => p[0]);
            this.setProfile(ids[(ids.indexOf(this.profile) + 1) % ids.length]);
        });
        this.profile = '';
    }

    showProfile(profile) {
        this.profile = profile || '';
        this.visible = !!profile;
        const p = PROFILES.find(x => x[0] === profile);
        if (p) {
            this.subtitle = p[1];
            this.iconName = p[2];
            this.checked = profile !== 'balanced';
        }
        for (const [id, item] of Object.entries(this._items))
            item.setOrnament(id === profile ? PopupMenu.Ornament.CHECK : PopupMenu.Ornament.NONE);
    }

    async setProfile(id) {
        const r = await Hearth.helper('quick-set', 'profile', id);
        if (r && !r.error)
            this.showProfile(id);
    }
});

const Indicator = GObject.registerClass(
class Indicator extends SystemIndicator {
    _init() {
        super._init();
        this.power = new PowerMode();
        this.quickSettingsItems.push(this.power);
        this.quickSettingsItems.push(new ActionToggle('Game Mode', 'input-gaming-symbolic', () => Hearth.gameMode()));
        this.quickSettingsItems.push(new ActionToggle('Hearth Settings', 'hearth-settings', () => Hearth.settings()));
    }

    async refresh() {
        const q = await Hearth.helper('quick');
        this.power.showProfile(q && !q.error ? q.profile : '');
    }
});

export default class HearthQuick extends Extension {
    enable() {
        this._indicator = new Indicator();
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._indicator);
        const menu = Main.panel.statusArea.quickSettings.menu;
        this._openId = menu.connect('open-state-changed', (m, open) => {
            if (open)
                this._indicator?.refresh();
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
