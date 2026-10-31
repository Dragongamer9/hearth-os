// Hearth GNOME's on-screen keyboard: Hearth's one keyboard (docs/OSK.md) drawn by GNOME Shell. GNOME has no
// layer-shell (the way Hearth Plasma's osk-plasma.qml shows the keyboard over KWin), so it's drawn here as Shell
// chrome along the bottom of the main screen: windows make room for it, and it never takes the keyboard focus, so
// what it types goes to the window or Shell field you're in. The engine is the very same file Mantel runs
// (desktop/shell/osk/hearth-osk.js, a copy of ui/osk), loaded as it is; this file only draws engine.view() (as
// OskBoard.qml does) and hands it the controller (hearth-pad's oskpad / oskaxes) and taps.
// Typing goes through hearth-pad's uinput keyboard (emit / key, as Mantel without wtype and Hearth Plasma); what a
// US uinput keyboard can't type (accents, emoji), or everything when hearth-pad isn't running, goes through GNOME's
// own virtual keyboard (Clutter's, by character). Settings and learned words: ~/.config/nimbus/osk.json and
// ~/.local/share/nimbus/osk-words.json, shared with Game Mode's and Mantel's keyboards.
import Clutter from 'gi://Clutter';
import GLib from 'gi://GLib';
import Gio from 'gi://Gio';
import St from 'gi://St';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';
import * as Hearth from 'file:///usr/share/nimbus/desktop/gnome/hearth.js';

const THEMES = {
    hearth: {bg: '#17171c', bg2: '#1f1f25', key: '#2b2b32', mod: '#222228', text: '#f3efe8', muted: '#a7a3ad', sel: '#f6f2ea', selText: '#141217', accent: '#ff8a3d', ink: '#2a1206'},
    ember: {bg: '#120c0a', bg2: '#1d1310', key: '#2a1a14', mod: '#1f1410', text: '#ffeadb', muted: '#c9a18a', sel: '#ffd9bd', selText: '#2a1206', accent: '#ff7a2f', ink: '#1d0b02'},
    slate: {bg: '#1b2028', bg2: '#222833', key: '#2d3542', mod: '#252c37', text: '#eef2f7', muted: '#9aa6b6', sel: '#f2f6fb', selText: '#111722', accent: '#ff8a3d', ink: '#2a1206'},
    daylight: {bg: '#e9e6e1', bg2: '#dedad3', key: '#ffffff', mod: '#d4d0c8', text: '#1d1b20', muted: '#6b6670', sel: '#1d1b20', selText: '#ffffff', accent: '#ff8a3d', ink: '#2a1206'},
};
const MOD_KEYS = ['bksp', 'enter', 'shift', 'sym', 'abc', 'sym2', 'emoji', 'clip', 'layout', 'left', 'right', 'up', 'down',
    'hide', 'tab', 'esc', 'ctrl', 'alt', 'home', 'end', 'paste', 'prev', 'next', 'page', 'cat'];
const KEYSYMS = {bksp: 'BackSpace', enter: 'Return', tab: 'Tab', esc: 'Escape', left: 'Left', right: 'Right', up: 'Up',
    down: 'Down', home: 'Home', end: 'End'};
const ASCII = /^[\x20-\x7e\n]*$/;

/** Loads Hearth's keyboard engine (plain ES2017 that defines `var HearthOSK`) from Mantel's copy. */
function loadEngine() {
    const src = Hearth.readText(`${Hearth.OSK_DIR}/hearth-osk.js`);
    if (!src)
        return null;
    // eslint-disable-next-line no-new-func
    return new Function(`${src}\nreturn HearthOSK;`)();
}

export class HearthKeyboard {
    constructor(pad) {
        this.pad = pad;                 // .send(obj), .ready
        this.open = false;
        this.typed = '';
        this.vm = null;
        this._sig = '';
        this._keys = [];
        this._words = {};
        try {
            this.Osk = loadEngine();
        } catch (e) {
            logError(e, 'Hearth keyboard: engine');
            this.Osk = null;
        }
        if (!this.Osk)
            return;
        this.Osk.configure({
            systemLayouts: this._systemLayouts(),
            save: d => Hearth.writeText(this._learnedPath(), JSON.stringify(d)),
        });
        this.engine = this.Osk.createEngine({
            host: 'mantel',
            adapter: {
                insert: (text, mods) => this._typeText(text, mods),
                deleteBack: n => this._typeKey('bksp', n, null),
                key: (name, mods) => this._typeKey(name, 1, mods),
            },
            onChange: () => this._changed(),
            onHide: () => this.close(),
            onHaptic: (strength, ms) => this.pad.send({cmd: 'rumble', strength, ms}),
        });
        this.actor = new St.BoxLayout({vertical: true, style_class: 'hosk', reactive: true, visible: false});
        this._strip = new St.BoxLayout({style_class: 'hosk-strip'});
        this._rows = new St.BoxLayout({vertical: true, style_class: 'hosk-rows', y_expand: true});
        this.actor.add_child(this._strip);
        this.actor.add_child(this._rows);
    }

    get available() {
        return !!this.engine;
    }

    destroy() {
        this.close();
        this._tickStop?.();
        this.actor?.destroy();
        this.actor = null;
        this._vkbd = null;
    }

    _learnedPath() {
        return GLib.build_filenamev([GLib.get_user_data_dir(), 'nimbus', 'osk-words.json']);
    }

    _systemLayouts() {
        try {
            const s = new Gio.Settings({schema_id: 'org.gnome.desktop.input-sources'});
            return s.get_value('sources').deepUnpack().filter(x => x[0] === 'xkb').map(x => x[1]);
        } catch (e) {
            return [];
        }
    }

    _loadState() {
        try {
            const st = Hearth.readText(GLib.build_filenamev([GLib.get_user_config_dir(), 'nimbus', 'osk.json']));
            if (st)
                this.Osk.configure({settings: JSON.parse(st)});
        } catch (e) {
            // defaults
        }
        try {
            const d = JSON.parse(Hearth.readText(this._learnedPath()) || '{}');
            this.Osk.importLearned({learned: d.learned || {}, next: d.next || {}, recentEmoji: d.recentEmoji || []});
        } catch (e) {
            // nothing learned yet
        }
        this._needWords();
    }

    _needWords() {
        for (const id of this.Osk.activeLayouts()) {
            const lay = this.Osk.LAYOUTS.find(l => l.id === id);
            if (!lay || this._words[lay.lang])
                continue;
            this._words[lay.lang] = true;
            const text = Hearth.readText(`${Hearth.OSK_DIR}/words/${lay.lang}.js`);
            const m = /HearthOSK\.words\("([a-z]+)",\s*"([^"]*)"\)/.exec(text);
            if (m)
                this.Osk.words(m[1], m[2]);
        }
    }

    toggle() {
        if (this.open)
            this.close();
        else
            this.show();
    }

    show() {
        if (!this.engine || this.open)
            return;
        this._loadState();
        this.engine.reset('text', '');
        this.engine.setHints(true);
        this.open = true;
        this.typed = '';
        this._place();
        Main.layoutManager.addTopChrome(this.actor, {affectsStruts: true, trackFullscreen: false});
        this.actor.show();
        this._monId = Main.layoutManager.connect('monitors-changed', () => this._place());
        this._tickStop = Hearth.every(16, () => this.engine.tick(Date.now()));
        this._changed();
        const s = this.vm && this.vm.settings ? this.vm.settings : {};
        this.pad.send({cmd: 'osk', open: true, split: s.fast === 'split'});
    }

    close() {
        if (!this.open)
            return;
        this.open = false;
        this._tickStop?.();
        this._tickStop = null;
        if (this._monId)
            Main.layoutManager.disconnect(this._monId);
        this._monId = 0;
        this.actor.hide();
        Main.layoutManager.removeChrome(this.actor);
        this.pad.send({cmd: 'osk', open: false});
    }

    padButton(b, down) {
        if (this.open && this.engine) {
            this.engine.setHints(true);
            this.engine.handle(b, down, Date.now());
        }
    }

    padAxes(lx, ly, rx, ry) {
        if (this.open && this.engine)
            this.engine.axes(lx, ly, rx, ry, Date.now());
    }

    _height(m) {
        const s = this.vm && this.vm.settings ? this.vm.settings : {};
        const f = s.size === 'l' ? 0.5 : 0.44;
        return Math.round(Math.min(m.height * f, Math.max(s.size === 's' ? 250 : 290, m.width * (s.size === 's' ? 0.2 : s.size === 'l' ? 0.29 : 0.24))));
    }

    _place() {
        const m = Main.layoutManager.primaryMonitor;
        if (!m || !this.actor)
            return;
        this.vm = this.engine.view();
        const h = this._height(m);
        const top = this.vm.settings && this.vm.settings.position === 'top';
        this.actor.set_size(m.width, h);
        this.actor.set_position(m.x, top ? m.y : m.y + m.height - h);
        this._sig = '';
    }

    // ------------------------------------------------ drawing engine.view()
    _theme() {
        const s = this.vm && this.vm.settings ? this.vm.settings : {};
        return THEMES[s.theme] || THEMES.hearth;
    }

    _label(k) {
        const vm = this.vm;
        if (k.t === 'ch')
            return (vm.shift || vm.caps) && k.v !== 'ß' ? k.v.toUpperCase() : k.v;
        if (k.t === 'shift' && vm.caps)
            return '⇪';
        return k.label || '';
    }

    _changed() {
        if (!this.actor)
            return;
        this.vm = this.engine.view();
        if (!this.open)
            return;
        const vm = this.vm;
        const th = this._theme();
        this.actor.set_style(`background-color: ${th.bg};`);
        const sig = JSON.stringify([vm.panel, vm.layout, vm.shift, vm.caps, vm.rows.map(r => r.map(k => [k.t, k.v, k.w, k.label, k.on, k.off, k.lock]))]);
        if (sig !== this._sig) {
            this._sig = sig;
            this._buildRows();
        }
        this._styleKeys();
        this._buildStrip();
        if (vm.layout)
            this._needWords();
    }

    _buildRows() {
        this._rows.destroy_all_children();
        this._keys = [];
        const vm = this.vm;
        const m = Main.layoutManager.primaryMonitor;
        const width = (this.actor.width || (m ? m.width : 1280)) - 40;
        const gap = 7;
        const rowsH = this.actor.height - 70;
        const keyH = Math.max(30, Math.min(72, Math.floor((rowsH - gap * vm.rows.length) / vm.rows.length)));
        vm.rows.forEach((row, r) => {
            const line = new St.BoxLayout({style_class: 'hosk-row', x_align: Clutter.ActorAlign.CENTER});
            const total = row.reduce((a, k) => a + k.w, 0) || 1;
            const avail = width - gap * (row.length - 1);
            row.forEach((k, c) => {
                const b = new St.Button({style_class: 'hosk-key', can_focus: false, reactive: k.t !== 'none', label: this._label(k)});
                b.set_size(Math.max(20, Math.floor(avail * k.w / total)), keyH);
                b.connect('clicked', () => {
                    this.engine.setHints(false);
                    this.engine.tap(r, c);
                });
                line.add_child(b);
                this._keys.push({b, k, r, c});
            });
            this._rows.add_child(line);
        });
    }

    _styleKeys() {
        const vm = this.vm;
        const th = this._theme();
        for (const {b, k, r, c} of this._keys) {
            const mod = MOD_KEYS.includes(k.t);
            const sel = vm.active && vm.sel.r === r && vm.sel.c === c;
            const aim = vm.split && ((vm.split.l && vm.split.l.r === r && vm.split.l.c === c) || (vm.split.r && vm.split.r.r === r && vm.split.r.c === c));
            const lit = sel || aim;
            const bg = k.t === 'none' || k.emoji ? 'transparent' : lit ? (k.go ? th.accent : th.sel)
                : k.go || k.lock ? th.accent : k.on ? th.mod : mod ? th.mod : th.key;
            const fg = lit && !k.go ? th.selText : (k.go || k.lock) ? th.ink : mod || k.t === 'none' ? th.muted : th.text;
            const ring = lit ? `box-shadow: 0 0 0 3px ${th.accent};` : k.on ? `box-shadow: inset 0 0 0 2px ${th.accent};` : '';
            b.set_style(`background-color: ${bg}; color: ${fg}; ${ring}`);
            b.opacity = k.off ? 90 : 255;
            if (b.label !== this._label(k))
                b.label = this._label(k);
        }
    }

    _buildStrip() {
        this._strip.destroy_all_children();
        const vm = this.vm;
        const th = this._theme();
        const chip = (text, selected, onClick, accent) => {
            const b = new St.Button({style_class: 'hosk-chip', label: text, can_focus: false, x_expand: true});
            b.set_style(`background-color: ${selected ? th.sel : th.bg2}; color: ${selected ? th.selText : accent ? th.accent : th.text};` +
                (selected ? ` box-shadow: 0 0 0 2px ${th.accent};` : ''));
            b.connect('clicked', onClick);
            this._strip.add_child(b);
        };
        if (vm.popup) {
            // accents for the picked key (hold A, View)
            vm.popup.alts.forEach((a, i) => chip(a, vm.popup.i === i, () => this.engine.pickAlt(i)));
        } else if (vm.wheel && vm.wheel.active) {
            // the Ember Wheel: the tilted coal's letters on Y X B A
            vm.wheel.petals.forEach((p, i) => chip(p, vm.wheel.petal === i, () => {}, false));
        } else if (vm.suggestions.length) {
            vm.suggestions.forEach((w, i) => chip(w, vm.active && vm.sel.r === -1 && vm.sel.c === i,
                () => this.engine.tap(-1, i), i === 0));
        } else {
            const s = vm.settings || {};
            const hint = !vm.hints ? '' : s.fast === 'wheel' ? 'Left stick: Ember Wheel · A B X Y type its letters · right stick moves the pointer, RT clicks'
                : s.fast === 'split' ? 'Point with both sticks · LT / RT type'
                    : 'A type · X ⌫ · Y space · LT ⇧ · LB RB move · ☰ Enter · B hide · hold A for accents · RT clicks';
            const l = new St.Label({text: hint, style_class: 'hosk-hint', x_expand: true, y_align: Clutter.ActorAlign.CENTER});
            l.set_style(`color: ${th.muted};`);
            this._strip.add_child(l);
        }
        const badge = new St.Label({text: vm.panel === 'emoji' ? 'Emoji' : vm.panel === 'clip' ? 'Clipboard' : (vm.layoutShort || ''),
            style_class: 'hosk-badge', y_align: Clutter.ActorAlign.CENTER});
        badge.set_style(`background-color: ${th.bg2}; color: ${th.muted};`);
        this._strip.add_child(badge);
    }

    // ------------------------------------------------ typing
    _modList(mods) {
        return mods ? Object.keys(mods).filter(k => mods[k]) : [];
    }

    _typeText(text, mods) {
        this.typed = (this.typed + text).slice(-200);
        if (this.pad.ready && ASCII.test(text)) {
            this.pad.send({cmd: 'emit', text, mods: this._modList(mods)});
            return;
        }
        for (const ch of text)
            this._keyval(ch === '\n' ? Clutter.KEY_Return : Clutter.unicode_to_keysym(ch.codePointAt(0)));
    }

    _typeKey(name, n, mods) {
        if (name === 'bksp')
            this.typed = this.typed.slice(0, Math.max(0, this.typed.length - (n || 1)));
        if (this.pad.ready) {
            this.pad.send({cmd: 'key', key: name, n: n || 1, mods: this._modList(mods)});
            return;
        }
        const sym = KEYSYMS[name] ? Clutter[`KEY_${KEYSYMS[name]}`] : 0;
        for (let i = 0; sym && i < Math.max(1, Math.min(n || 1, 200)); i++)
            this._keyval(sym);
    }

    _keyval(sym) {
        try {
            if (!this._vkbd) {
                const seat = Clutter.get_default_backend().get_default_seat();
                this._vkbd = seat.create_virtual_device(Clutter.InputDeviceType.KEYBOARD_DEVICE);
            }
            const t = GLib.get_monotonic_time();
            this._vkbd.notify_keyval(t, sym, Clutter.KeyState.PRESSED);
            this._vkbd.notify_keyval(t + 1000, sym, Clutter.KeyState.RELEASED);
        } catch (e) {
            logError(e, 'Hearth keyboard: typing');
        }
    }

    state() {
        const vm = this.vm || {};
        return {open: this.open, typed: this.typed, panel: vm.panel || '', layout: vm.layout || '', sel: vm.sel || null,
            available: this.available, uinput: !!this.pad.ready};
    }
}
