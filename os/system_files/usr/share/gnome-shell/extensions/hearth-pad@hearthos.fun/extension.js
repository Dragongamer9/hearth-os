// Hearth GNOME with a controller (docs/GNOME.md), as Mantel's Pad.qml does for Hearth Classic: starts
// `hearth-pad --gnome` (desktop/shell/hearth-pad; it reads every controller and acts as a mouse and keyboard through
// /dev/uinput), draws what it asks for (the Hearth menu on Guide, Hearth's on-screen keyboard on Guide + X / Y), and
// tells it what's on screen:
//   keys {on}   GNOME Shell itself has the keyboard (the overview, the app grid, a menu, Quick Settings, the calendar
//               and notifications, the bar after View + Menu): the D-pad moves the Shell's focus, A is Enter, B Esc;
//   pause {on}  a fullscreen window (a game) is in front;
//   app {kind}  GNOME Files, Console or Text Editor is in front: the buttons become those apps' keys.
// What hearth-pad sends back: launcher (Menu: the Hearth launcher), overview (View), close (View + B), barfocus
// (View + Menu), radial, osk, oskpad / oskaxes (the keyboard's buttons and sticks), toast, mouse.
// GNOME's own on-screen keyboard is switched off while this runs (Hearth's is the keyboard), and given back after.
// Quick Settings gets Keyboard and Controller mouse. Without python3-evdev or /dev/uinput, hearth-pad says why and
// stops; GNOME then works as usual with mouse, keyboard and touch (the keyboard still opens from Quick Settings).
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import GObject from 'gi://GObject';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import {QuickToggle, SystemIndicator} from 'resource:///org/gnome/shell/ui/quickSettings.js';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';
import {HearthRadial} from './radial.js';
import {HearthKeyboard} from './keyboard.js';

// The apps with keys of their own on the controller (hearth-pad gnome_button), by window class / app id.
const APP_KINDS = [[/nautilus/i, 'files'], [/console|kgx/i, 'terminal'], [/texteditor/i, 'text']];

/** hearth-pad as a child process: one JSON object per line each way. Restarted after 3 s unless something is missing. */
class PadProcess {
    constructor(onLine) {
        this.onLine = onLine;
        this.ready = false;
        this.problem = '';
        this.running = false;
        this._queue = [];
        this._writing = false;
        this._stopped = false;
    }

    start() {
        if (this._stopped || this.running)
            return;
        if (!Hearth.exists(Hearth.PAD)) {
            this.problem = 'hearth-pad is missing';
            return;
        }
        try {
            const l = new Gio.SubprocessLauncher({flags: Gio.SubprocessFlags.STDIN_PIPE | Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE});
            this._proc = l.spawnv(['python3', Hearth.PAD, '--gnome']);
        } catch (e) {
            this.problem = `couldn't start hearth-pad: ${e.message}`;
            return;
        }
        this.running = true;
        this._stdin = this._proc.get_stdin_pipe();
        this._stdout = new Gio.DataInputStream({base_stream: this._proc.get_stdout_pipe(), close_base_stream: true});
        this._read();
        this._proc.wait_async(null, (p, res) => {
            try {
                p.wait_finish(res);
            } catch (e) {
                // ended
            }
            const code = p.get_if_exited() ? p.get_exit_status() : -1;
            this.running = false;
            this.ready = false;
            this._proc = null;
            this._stdin = null;
            this._queue = [];
            this._writing = false;
            // 2: python3-evdev missing, 3: no /dev/uinput (see hearth-pad): don't keep retrying.
            if (!this._stopped && code !== 2 && code !== 3)
                this._restart = Hearth.later(3000, () => this.start());
        });
    }

    _read() {
        this._stdout.read_line_async(GLib.PRIORITY_DEFAULT, null, (s, res) => {
            let line;
            try {
                [line] = s.read_line_finish_utf8(res);
            } catch (e) {
                return;
            }
            if (line === null)
                return;
            try {
                this.onLine(line);
            } catch (e) {
                logError(e, 'Hearth controller');
            }
            this._read();
        });
    }

    send(obj) {
        if (!this._stdin)
            return;
        this._queue.push(new TextEncoder().encode(`${JSON.stringify(obj)}\n`));
        this._flush();
    }

    _flush() {
        if (this._writing || !this._stdin || !this._queue.length)
            return;
        this._writing = true;
        const bytes = new GLib.Bytes(this._queue.shift());
        this._stdin.write_bytes_async(bytes, GLib.PRIORITY_DEFAULT, null, (s, res) => {
            try {
                s.write_bytes_finish(res);
            } catch (e) {
                // hearth-pad went away; wait_async restarts it
            }
            this._writing = false;
            this._flush();
        });
    }

    stop() {
        this._stopped = true;
        this._restart?.();
        try {
            this._proc?.force_exit();
        } catch (e) {
            // already gone
        }
    }
}

const PadToggles = GObject.registerClass(
class PadToggles extends SystemIndicator {
    _init(ext) {
        super._init();
        this.keyboard = new QuickToggle({title: 'Keyboard', iconName: 'input-keyboard-symbolic', toggleMode: false});
        this.keyboard.connect('clicked', () => {
            Main.panel.statusArea.quickSettings.menu.close();
            ext.keyboard?.toggle();
        });
        this.mouse = new QuickToggle({title: 'Controller mouse', iconName: 'input-mouse-symbolic', toggleMode: false});
        this.mouse.connect('clicked', () => ext.setMouse(!ext.mouseOn));
        this.quickSettingsItems.push(this.keyboard, this.mouse);
    }
});

export default class HearthPad extends Extension {
    enable() {
        this.mouseOn = true;
        this.lastAction = '';
        this._sent = {};
        this.pad = new PadProcess(line => this._handle(line));
        this.keyboard = new HearthKeyboard(this.pad);
        this.radial = new HearthRadial(action => this.run(action), () => this.pad.send({cmd: 'radial', open: false}));
        this._quietGnomeKeyboard();
        this._toggles = new PadToggles(this);
        Main.panel.statusArea.quickSettings.addExternalIndicator(this._toggles);
        this._syncMouse();
        this.pad.start();
        this._syncStop = Hearth.every(250, () => this._sync());
    }

    disable() {
        this._syncStop?.();
        this._syncStop = null;
        this.pad?.stop();
        this.pad = null;
        this.keyboard?.destroy();
        this.keyboard = null;
        this.radial?.destroy();
        this.radial = null;
        this._toggles?.quickSettingsItems.forEach(i => i.destroy());
        this._toggles?.destroy();
        this._toggles = null;
        this._toast?.destroy();
        this._toast = null;
        this._giveBackGnomeKeyboard();
    }

    // ---- GNOME's own on-screen keyboard off while Hearth's is the keyboard (it also comes up by itself on touch).
    _quietGnomeKeyboard() {
        try {
            const km = Main.keyboard;
            if (km && typeof km._syncEnabled === 'function') {
                this._origSync = km._syncEnabled;
                km._syncEnabled = function () {
                    if (this._keyboard && typeof this._destroyKeyboard === 'function')
                        this._destroyKeyboard();
                };
                km._syncEnabled();
            }
        } catch (e) {
            logError(e, "Hearth controller: GNOME's keyboard");
        }
    }

    _giveBackGnomeKeyboard() {
        try {
            if (this._origSync) {
                Main.keyboard._syncEnabled = this._origSync;
                Main.keyboard._syncEnabled();
            }
        } catch (e) {
            // GNOME's own state
        }
        this._origSync = null;
    }

    // ---- what's on screen → hearth-pad
    _shellHasKeyboard() {
        if (this.keyboard?.open)
            return false;
        const f = global.stage.get_key_focus();
        return (f !== null && f !== global.stage) || (Main.modalCount || 0) > 0;
    }

    _appKind(w) {
        if (!w)
            return '';
        const id = `${w.get_wm_class() || ''} ${w.get_gtk_application_id?.() || ''}`;
        const hit = APP_KINDS.find(([re]) => re.test(id));
        return hit ? hit[1] : '';
    }

    _sync() {
        if (!this.pad?.ready)
            return;
        const w = global.display.get_focus_window();
        const state = {
            keys: this._shellHasKeyboard(),
            pause: !!(w && w.is_fullscreen()),
            app: this._appKind(w),
        };
        if (state.keys !== this._sent.keys)
            this.pad.send({cmd: 'keys', on: state.keys});
        if (state.pause !== this._sent.pause)
            this.pad.send({cmd: 'pause', on: state.pause});
        if (state.app !== this._sent.app)
            this.pad.send({cmd: 'app', kind: state.app});
        this._sent = state;
    }

    _handle(line) {
        let m;
        try {
            m = JSON.parse(line);
        } catch (e) {
            return;
        }
        switch (m.t) {
        case 'ready':
            this.pad.ready = true;
            this.pad.problem = '';
            this._sent = {};
            this.pad.send({cmd: 'mouse', on: this.mouseOn});
            if (this.keyboard?.open)
                this.pad.send({cmd: 'osk', open: true});
            this._sync();
            break;
        case 'error':
            this.pad.problem = m.text || '';
            break;
        case 'radial':
            this.radial.update(m);
            break;
        case 'osk':
            if (m.open)
                this.keyboard.show();
            else
                this.keyboard.close();
            break;
        case 'oskpad':
            this.keyboard.padButton(m.b, !!m.down);
            break;
        case 'oskaxes':
            this.keyboard.padAxes(m.lx || 0, m.ly || 0, m.rx || 0, m.ry || 0);
            break;
        case 'launcher':
            this.run('apps');
            break;
        case 'overview':
            this.run('overview');
            break;
        case 'close':
            this.run('close');
            break;
        case 'barfocus':
            this.run('bar');
            break;
        case 'toast':
            this.toast(m.text || '');
            break;
        case 'mouse':
            this.mouseOn = m.on !== false;
            this._syncMouse();
            break;
        }
    }

    /** What the Hearth menu's items and the controller's shortcuts do. */
    run(action) {
        this.lastAction = action;
        switch (action) {
        case 'apps': {
            const l = Main.extensionManager.lookup('hearth-launcher@hearthos.fun');
            if (l && l.stateObj && typeof l.stateObj.toggle === 'function')
                l.stateObj.toggle();
            else
                Main.overview.showApps();
            break;
        }
        case 'keyboard':
            this.keyboard?.toggle();
            break;
        case 'quick':
            Main.panel.statusArea.quickSettings?.menu.open();
            break;
        case 'notifications':
            Main.panel.statusArea.dateMenu?.menu.open();
            break;
        case 'close': {
            const w = global.display.get_focus_window();
            if (w && w.can_close())
                w.delete(global.get_current_time());
            break;
        }
        case 'game':
            Hearth.gameMode();
            break;
        case 'overview':
            Main.overview.toggle();
            break;
        case 'mouse':
            this.setMouse(!this.mouseOn);
            break;
        case 'bar':
            // The D-pad walks the bar from here (GNOME's focus groups); B / Esc gives the focus back to the window.
            Main.panel.navigate_focus(null, St.DirectionType.TAB_FORWARD, false);
            break;
        }
    }

    setMouse(on) {
        this.mouseOn = on;
        this.pad?.send({cmd: 'mouse', on});
        this._syncMouse();
        this.toast(`Controller mouse ${on ? 'on' : 'off'}`);
    }

    _syncMouse() {
        if (this._toggles)
            this._toggles.mouse.checked = this.mouseOn;
        this.radial?.setLabel('mouse', this.mouseOn ? 'Controller mouse: on' : 'Controller mouse: off');
    }

    toast(text) {
        if (!text)
            return;
        const m = Main.layoutManager.primaryMonitor;
        if (!m)
            return;
        if (!this._toast) {
            this._toast = new St.Label({style_class: 'hearth-toast'});
            Main.layoutManager.addTopChrome(this._toast);
        }
        this._toast.text = text;
        this._toast.show();
        this._toast.set_position(Math.round(m.x + (m.width - this._toast.width) / 2), m.y + Math.round(m.height * 0.12));
        this._toastStop?.();
        this._toastStop = Hearth.later(1800, () => this._toast?.hide());
    }

    // ---- for checks (gnome-test.yml) and the Hearth menu from elsewhere
    openKeyboard() {
        this.keyboard?.show();
    }

    openRadial() {
        this.radial.show();
        this.pad?.send({cmd: 'radial', open: true});
    }

    debugState() {
        const w = global.display.get_focus_window();
        return JSON.stringify({
            running: !!this.pad?.running, ready: !!this.pad?.ready, problem: this.pad ? this.pad.problem : '',
            sent: this._sent, mouse: this.mouseOn, lastAction: this.lastAction,
            radial: this.radial ? {open: this.radial.open, sel: this.radial.sel} : null,
            osk: this.keyboard ? this.keyboard.state() : null,
            focus: w ? (w.get_wm_class() || '') : '', title: w ? (w.get_title() || '') : '',
            shellKeyboard: this._shellHasKeyboard(), gnomeKeyboardOff: !!this._origSync,
        });
    }
}
