// Hearth GNOME's bar (docs/GNOME.md), inspired by Mantel's floating bottom bar (hosts/linux/desktop/shell/Bar.qml):
//   - GNOME's top bar moves to the bottom of the main screen and floats (stylesheet.css: a rounded bar with a margin
//     all round; windows keep clear of it, like Mantel's exclusive zone);
//   - pinned and open apps sit after the Hearth mark and Activities, Mantel's order: pinned first (GNOME's favourites,
//     Mantel's pins the first time), then the other open apps; windows of one app share a button. Click: open, bring
//     to the front, or minimize when it's in front; several windows: the next one. Middle-click: a new window.
//     Right-click (X on the controller): GNOME's app menu (pin / unpin, new window, quit);
//   - the thin strip at the very end shows the desktop (again: the windows come back).
// Every button takes the keyboard focus, so the controller's View + Menu (hearth-pad) walks the bar with the D-pad.
// Turning it off puts the top bar back where GNOME had it.
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import Shell from 'gi://Shell';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as PopupMenu from 'resource:///org/gnome/shell/ui/popupMenu.js';
import * as AppFavorites from 'resource:///org/gnome/shell/ui/appFavorites.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const ICON = 30;

const TaskButton = GObject.registerClass(
class TaskButton extends St.Button {
    _init(app, bar) {
        super._init({style_class: 'hearth-task', can_focus: true, reactive: true, track_hover: true,
            button_mask: St.ButtonMask.ONE | St.ButtonMask.TWO | St.ButtonMask.THREE, accessible_name: app.get_name()});
        this.app = app;
        this._bar = bar;
        const box = new St.BoxLayout({vertical: true, x_align: Clutter.ActorAlign.CENTER});
        box.add_child(app.create_icon_texture(ICON));
        this._dot = new St.Widget({style_class: 'hearth-task-dot', x_align: Clutter.ActorAlign.CENTER});
        box.add_child(this._dot);
        this.set_child(box);
        this.connect('clicked', (b, button) => this._clicked(button));
        this.connect('popup-menu', () => this._menu());
    }

    sync(focusApp) {
        const running = this.app.get_windows().length > 0;
        this._dot.visible = running;
        if (focusApp === this.app)
            this.add_style_class_name('focused');
        else
            this.remove_style_class_name('focused');
    }

    _clicked(button) {
        if (button === 2)
            return this._newWindow();
        if (button === 3)
            return this._menu();
        const wins = this.app.get_windows().filter(w => !w.skip_taskbar);
        if (!wins.length)
            return this._newWindow();
        const t = global.get_current_time();
        const focus = global.display.get_focus_window();
        const i = wins.indexOf(focus);
        if (wins.length === 1) {
            if (i === 0 && !Main.overview.visible)
                wins[0].minimize();
            else
                Main.activateWindow(wins[0], t);
        } else {
            Main.activateWindow(wins[(i + 1) % wins.length], t);
        }
        if (Main.overview.visible)
            Main.overview.hide();
    }

    _newWindow() {
        try {
            this.app.open_new_window(-1);
        } catch (e) {
            this.app.activate();
        }
        if (Main.overview.visible)
            Main.overview.hide();
    }

    async _menu() {
        try {
            if (!this._appMenu) {
                const {AppMenu} = await import('resource:///org/gnome/shell/ui/appMenu.js');
                this._appMenu = new AppMenu(this, St.Side.BOTTOM);
                this._appMenu.setApp(this.app);
                Main.uiGroup.add_child(this._appMenu.actor);
                this._appMenu.actor.hide();
                this._bar.menuManager.addMenu(this._appMenu);
            }
            this._appMenu.open(true);
        } catch (e) {
            logError(e, 'Hearth bar: app menu');
        }
    }

    _onDestroy() {
        this._appMenu?.destroy();
    }

    vfunc_destroy() {
        this._onDestroy();
        super.vfunc_destroy();
    }
});

const TaskBar = GObject.registerClass(
class TaskBar extends St.BoxLayout {
    _init() {
        super._init({style_class: 'hearth-tasks', reactive: true, y_align: Clutter.ActorAlign.CENTER});
        this.menuManager = new PopupMenu.PopupMenuManager(this);
        this._apps = Shell.AppSystem.get_default();
        this._tracker = Shell.WindowTracker.get_default();
        this._favs = AppFavorites.getAppFavorites();
        this._ids = [
            [this._apps, this._apps.connect('app-state-changed', () => this._queue())],
            [this._favs, this._favs.connect('changed', () => this._queue())],
            [this._tracker, this._tracker.connect('notify::focus-app', () => this._syncFocus())],
            [global.display, global.display.connect('window-created', () => this._queue())],
        ];
        this._rebuild();
    }

    _queue() {
        if (this._pending)
            return;
        this._pending = true;
        this._idle = GLib.idle_add(GLib.PRIORITY_DEFAULT_IDLE, () => {
            this._pending = false;
            this._idle = 0;
            this._rebuild();
            return GLib.SOURCE_REMOVE;
        });
    }

    _list() {
        const out = [];
        const seen = new Set();
        for (const app of this._favs.getFavorites()) {
            if (!seen.has(app.get_id())) {
                seen.add(app.get_id());
                out.push(app);
            }
        }
        for (const app of this._apps.get_running()) {
            if (!seen.has(app.get_id())) {
                seen.add(app.get_id());
                out.push(app);
            }
        }
        return out;
    }

    _rebuild() {
        const apps = this._list();
        const have = this.get_children();
        const same = have.length === apps.length && have.every((b, i) => b.app === apps[i]);
        if (!same) {
            this.destroy_all_children();
            for (const app of apps)
                this.add_child(new TaskButton(app, this));
        }
        this._syncFocus();
    }

    _syncFocus() {
        const f = this._tracker.focus_app;
        for (const b of this.get_children())
            b.sync?.(f);
    }

    vfunc_destroy() {
        for (const [obj, id] of this._ids)
            obj.disconnect(id);
        this._ids = [];
        if (this._idle)
            GLib.source_remove(this._idle);
        this._idle = 0;
        super.vfunc_destroy();
    }
});

// The strip at the very end: show the desktop, then the same windows again.
const ShowDesktop = GObject.registerClass(
class ShowDesktop extends St.Button {
    _init() {
        super._init({style_class: 'hearth-show-desktop', can_focus: true, reactive: true, track_hover: true,
            accessible_name: 'Show the desktop', y_expand: true});
        this._shown = null;
        this.connect('clicked', () => this.toggle());
    }

    toggle() {
        const ws = global.workspace_manager.get_active_workspace();
        const open = ws.list_windows().filter(w => !w.minimized && !w.skip_taskbar &&
            w.get_window_type() === 0 /* Meta.WindowType.NORMAL */);
        if (open.length) {
            this._shown = {windows: open, front: global.display.get_focus_window()};
            open.forEach(w => w.minimize());
            return;
        }
        const was = this._shown;
        this._shown = null;
        if (!was)
            return;
        was.windows.filter(w => w !== was.front && w.get_workspace()).forEach(w => w.unminimize());
        if (was.front && was.front.get_workspace())
            Main.activateWindow(was.front, global.get_current_time());
    }
});

export default class HearthBar extends Extension {
    enable() {
        const lm = Main.layoutManager;
        this._signals = [];
        // The bar along the bottom of the main screen.
        this._place = () => {
            const m = lm.primaryMonitor;
            if (!m)
                return;
            lm.panelBox.set_position(m.x, m.y + m.height - lm.panelBox.height);
        };
        this._signals.push([lm, lm.connect('monitors-changed', () => this._place())]);
        this._signals.push([lm.panelBox, lm.panelBox.connect('notify::height', () => this._place())]);
        Main.panel.add_style_class_name('hearth-bar');
        lm.panelBox.add_style_class_name('hearth-bar-box');
        this._place();

        // Pinned and open apps after the Hearth mark and Activities.
        this._tasks = new TaskBar();
        Main.panel._leftBox.add_child(this._tasks);

        // The show-desktop strip stays the last thing on the bar.
        this._strip = new ShowDesktop();
        const right = Main.panel._rightBox;
        right.add_child(this._strip);
        this._signals.push([right, right.connect('child-added', () => {
            if (this._strip && this._strip.get_parent() === right)
                right.set_child_above_sibling(this._strip, null);
        })]);
    }

    disable() {
        for (const [obj, id] of this._signals || [])
            obj.disconnect(id);
        this._signals = [];
        this._tasks?.destroy();
        this._tasks = null;
        this._strip?.destroy();
        this._strip = null;
        Main.panel.remove_style_class_name('hearth-bar');
        const lm = Main.layoutManager;
        lm.panelBox.remove_style_class_name('hearth-bar-box');
        const m = lm.primaryMonitor;
        if (m)
            lm.panelBox.set_position(m.x, m.y);
    }

    /** For checks (gnome-test.yml): what the bar shows. */
    debugState() {
        return JSON.stringify({
            y: Main.layoutManager.panelBox.y,
            height: Main.layoutManager.panelBox.height,
            monitorBottom: Main.layoutManager.primaryMonitor ? Main.layoutManager.primaryMonitor.y + Main.layoutManager.primaryMonitor.height : 0,
            apps: this._tasks ? this._tasks.get_children().map(b => b.app.get_id()) : [],
        });
    }
}
