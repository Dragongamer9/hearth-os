// Hearth GNOME: Hearth's app launcher, the Hearth mark at the start of the bar (docs/GNOME.md). The same launcher as
// Mantel's (Launcher.qml) and Hearth Plasma's (org.hearth.launcher): search, then Apps (Hearth's own apps and yours
// first, the apps that came with the system in one "System apps" folder) and Games (your installed games from
// Hearth's library, started through Hearth), and Settings, Game Mode, Lock, Sleep, Restart and Power off along the
// bottom. The lists come from desktop/gnome/hearth-gnome (hearth-plasma with GNOME's view of which apps show), so it
// lists exactly what Mantel's launcher does. Arrows (the controller's D-pad) move, Enter (A) opens, Tab (LB / RB)
// switches Apps / Games, Esc (B) closes; the controller's Menu button opens it (hearth-pad@hearthos.fun).
import Clutter from 'gi://Clutter';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PanelMenu from 'resource:///org/gnome/shell/ui/panelMenu.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const COLUMNS = 6;

const Launcher = GObject.registerClass(
class Launcher extends PanelMenu.Button {
    _init() {
        super._init(0.0, 'Hearth apps');
        this.add_style_class_name('hearth-mark');
        this.add_child(new St.Icon({gicon: Hearth.icon('hearth', 'view-app-grid-symbolic'), style_class: 'hearth-mark-icon'}));
        this.apps = [];
        this.games = [];
        this.section = 'apps';
        this.folderOpen = false;
        this.menu.actor.add_style_class_name('hearth-launcher-menu');
        this._build();
        this.menu.connect('open-state-changed', (m, open) => {
            if (!open)
                return;
            this._search.set_text('');
            this.folderOpen = false;
            this._fill();
            this.refresh();
            global.stage.set_key_focus(this._search.clutter_text);
        });
        this.refresh();
    }

    _build() {
        const item = new PopupMenu.PopupBaseMenuItem({reactive: false, can_focus: false, style_class: 'hearth-launcher-item'});
        const col = new St.BoxLayout({vertical: true, style_class: 'hearth-launcher', x_expand: true});
        item.add_child(col);
        this.menu.addMenuItem(item);

        const top = new St.BoxLayout({style_class: 'hearth-launcher-top'});
        this._search = new St.Entry({style_class: 'hearth-launcher-search', hint_text: 'Search apps', can_focus: true, x_expand: true});
        this._search.clutter_text.connect('text-changed', () => this._fill());
        this._search.clutter_text.connect('key-press-event', (a, ev) => this._searchKey(ev));
        top.add_child(this._search);
        this._tabs = {};
        for (const [id, label] of [['apps', 'Apps'], ['games', 'Games']]) {
            const b = new St.Button({label, style_class: 'hearth-launcher-tab', can_focus: true, toggle_mode: false});
            b.connect('clicked', () => this.setSection(id));
            this._tabs[id] = b;
            top.add_child(b);
        }
        col.add_child(top);

        this._grid = new St.Widget({layout_manager: new Clutter.GridLayout({column_homogeneous: true, row_spacing: 6, column_spacing: 6}),
            style_class: 'hearth-launcher-grid', x_expand: true});
        this._scroll = new St.ScrollView({style_class: 'hearth-launcher-scroll', x_expand: true, y_expand: true,
            hscrollbar_policy: St.PolicyType.NEVER, vscrollbar_policy: St.PolicyType.AUTOMATIC});
        if (this._scroll.set_child)
            this._scroll.set_child(this._grid);
        else
            this._scroll.add_actor(this._grid);
        col.add_child(this._scroll);
        this._empty = new St.Label({style_class: 'hearth-launcher-empty', text: '', visible: false});
        col.add_child(this._empty);

        const bottom = new St.BoxLayout({style_class: 'hearth-launcher-bottom'});
        const act = (iconName, label, fn) => {
            const b = new St.Button({style_class: 'hearth-launcher-action', can_focus: true, accessible_name: label});
            const box = new St.BoxLayout({style_class: 'hearth-launcher-action-box'});
            box.add_child(new St.Icon({gicon: Hearth.icon(iconName, 'system-run-symbolic'), icon_size: 18}));
            box.add_child(new St.Label({text: label, y_align: Clutter.ActorAlign.CENTER}));
            b.set_child(box);
            b.connect('clicked', () => {
                this.menu.close();
                fn();
            });
            bottom.add_child(b);
            return b;
        };
        act('hearth-settings', 'Settings', () => Hearth.settings());
        act('input-gaming-symbolic', 'Game Mode', () => Hearth.gameMode());
        bottom.add_child(new St.Widget({x_expand: true}));
        act('system-lock-screen-symbolic', 'Lock', () => Hearth.shell('loginctl lock-session'));
        act('weather-clear-night-symbolic', 'Sleep', () => Hearth.spawn(['systemctl', 'suspend']));
        act('system-reboot-symbolic', 'Restart', () => Hearth.shell('gnome-session-quit --reboot || systemctl reboot'));
        act('system-shutdown-symbolic', 'Power off', () => Hearth.shell('gnome-session-quit --power-off || systemctl poweroff'));
        col.add_child(bottom);
    }

    async refresh() {
        const a = await Hearth.helper('apps');
        if (Array.isArray(a))
            this.apps = a;
        const g = await Hearth.helper('games');
        if (Array.isArray(g))
            this.games = g;
        if (this.menu.isOpen)
            this._fill();
        // Then what Hearth lists now (the cached list shows at once; this can take a few seconds).
        const g2 = await Hearth.helper('games', 'fresh');
        if (Array.isArray(g2) && g2.length) {
            this.games = g2;
            if (this.menu.isOpen && this.section === 'games')
                this._fill();
        }
    }

    setSection(id) {
        this.section = id;
        this.folderOpen = false;
        this._search.hint_text = id === 'games' ? 'Search your games' : 'Search apps';
        this._fill();
    }

    _searchKey(ev) {
        const k = ev.get_key_symbol();
        if (k === Clutter.KEY_Down) {
            const first = this._grid.get_first_child();
            if (first)
                first.grab_key_focus();
            return Clutter.EVENT_STOP;
        }
        if (k === Clutter.KEY_Return || k === Clutter.KEY_KP_Enter) {
            const first = this._shown()[0];
            if (first)
                this._activate(first);
            return Clutter.EVENT_STOP;
        }
        if (k === Clutter.KEY_Tab || k === Clutter.KEY_ISO_Left_Tab) {
            this.setSection(this.section === 'apps' ? 'games' : 'apps');
            return Clutter.EVENT_STOP;
        }
        return Clutter.EVENT_PROPAGATE;
    }

    _matches(a, t) {
        return [a.name, a.generic, a.id].concat(a.keywords || []).some(x => String(x || '').toLowerCase().includes(t));
    }

    _shown() {
        const t = this._search.get_text().trim().toLowerCase();
        if (this.section === 'games') {
            return this.games.filter(g => !t || String(g.title || '').toLowerCase().includes(t))
                .map(g => ({kind: 'game', game: g, name: g.title, icon: g.icon || g.cover || ''}));
        }
        if (t)
            return this.apps.filter(a => this._matches(a, t)).map(a => ({kind: 'app', app: a, name: a.name, icon: a.icon}));
        const sys = this.apps.filter(a => a.system);
        if (this.folderOpen)
            return sys.map(a => ({kind: 'app', app: a, name: a.name, icon: a.icon}));
        const list = this.apps.filter(a => !a.system).map(a => ({kind: 'app', app: a, name: a.name, icon: a.icon}));
        if (sys.length)
            list.push({kind: 'folder', name: 'System apps', icon: 'folder'});
        return list;
    }

    _fill() {
        for (const [id, b] of Object.entries(this._tabs)) {
            if (id === this.section)
                b.add_style_pseudo_class('checked');
            else
                b.remove_style_pseudo_class('checked');
        }
        this._grid.destroy_all_children();
        const items = this._shown();
        const layout = this._grid.layout_manager;
        items.forEach((it, i) => {
            const tile = new St.Button({style_class: 'hearth-tile', can_focus: true, reactive: true, track_hover: true, accessible_name: it.name || ''});
            const box = new St.BoxLayout({vertical: true, style_class: 'hearth-tile-box'});
            const fallback = it.kind === 'game' ? 'input-gaming-symbolic' : it.kind === 'folder' ? 'folder-symbolic' : 'application-x-executable';
            box.add_child(new St.Icon({gicon: Hearth.icon(it.icon, fallback), icon_size: 48, x_align: Clutter.ActorAlign.CENTER}));
            const label = new St.Label({text: it.name || '', style_class: 'hearth-tile-label', x_align: Clutter.ActorAlign.CENTER});
            label.clutter_text.ellipsize = 3; // Pango.EllipsizeMode.END
            box.add_child(label);
            tile.set_child(box);
            tile.connect('clicked', () => this._activate(it));
            tile.connect('key-focus-in', () => this._ensureVisible(tile));
            layout.attach(tile, i % COLUMNS, Math.floor(i / COLUMNS), 1, 1);
        });
        this._empty.visible = items.length === 0;
        this._empty.text = this.section === 'games' ? 'No games yet: add them in Game Mode.' : 'No apps found.';
    }

    _ensureVisible(actor) {
        try {
            const adj = this._scroll.vadjustment || this._scroll.get_vscroll_bar?.().get_adjustment();
            if (!adj)
                return;
            const [, y] = actor.get_transformed_position();
            const [, gy] = this._grid.get_transformed_position();
            const top = y - gy;
            const bottom = top + actor.height;
            if (top < adj.value)
                adj.value = top;
            else if (bottom > adj.value + adj.page_size)
                adj.value = bottom - adj.page_size;
        } catch (e) {
            // best effort
        }
    }

    _activate(it) {
        if (it.kind === 'folder') {
            this.folderOpen = true;
            this._fill();
            const first = this._grid.get_first_child();
            if (first)
                first.grab_key_focus();
            return;
        }
        if (it.kind === 'game')
            Hearth.helper('play', it.game.id);
        else
            Hearth.helper('launch', it.app.id);
        this.menu.close();
    }
});

export default class HearthLauncher extends Extension {
    enable() {
        this._button = new Launcher();
        Main.panel.addToStatusArea('hearth-launcher', this._button, 0, 'left');
    }

    disable() {
        this._button?.destroy();
        this._button = null;
    }

    /** The controller's Menu button and the Hearth menu's Apps (hearth-pad@hearthos.fun). */
    toggle() {
        this._button?.menu.toggle();
    }

    isOpen() {
        return !!this._button?.menu.isOpen;
    }

    debugState() {
        return JSON.stringify({open: this.isOpen(), apps: this._button ? this._button.apps.length : 0,
            games: this._button ? this._button.games.length : 0, section: this._button ? this._button.section : ''});
    }
}
