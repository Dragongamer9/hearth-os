// Hearth GNOME's look (docs/GNOME.md). stylesheet.css is GNOME Shell in Hearth's colours (Mantel's Theme.qml: warm
// firelit darks, the ember accent #ff8a3d) and font; GNOME loads it on top of its own theme while this is on.
// The first time Hearth GNOME starts, Hearth's defaults go into GNOME's settings, only where you haven't set your own:
// the Fireside wallpaper (desktop and lock screen), dark style with the orange accent (GTK 3 / GTK 4 / libadwaita:
// Hearth's own colours come from ~/.config/gtk-4.0/gtk.css, theme/apply.sh), Breeze icons and cursor with Hearth's
// icon set (as in Hearth Classic), the date next to the clock, minimize / maximize / close on windows, no lock on
// idle (as Hearth Classic), GNOME's own on-screen keyboard off (Hearth's is on Guide + X), and Mantel's pinned apps.
// Anything you change afterwards in GNOME's Settings stays.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';
import Shell from 'gi://Shell';
import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';

const WALLPAPER = '/usr/share/backgrounds/hearth/hearth-fireside.jpg';
// Mantel's default pins (hearth-desk DEFAULT_PINS); only the installed ones.
const PINS = ['beacon.desktop', 'xodus-nexus.desktop', 'io.github.kolunmi.Bazaar.desktop', 'org.gnome.Nautilus.desktop'];

function settingsFor(id) {
    const src = Gio.SettingsSchemaSource.get_default();
    const schema = src ? src.lookup(id, true) : null;
    return schema ? {settings: new Gio.Settings({settings_schema: schema}), schema} : null;
}

export default class HearthTheme extends Extension {
    enable() {
        try {
            this._firstRun();
        } catch (e) {
            logError(e, 'Hearth look: defaults');
        }
    }

    disable() {
        // The stylesheet goes with the extension; settings written once stay (they're ordinary GNOME settings).
    }

    _firstRun() {
        const mark = GLib.build_filenamev([GLib.get_user_config_dir(), 'nimbus', 'gnome-set-up']);
        if (GLib.file_test(mark, GLib.FileTest.EXISTS))
            return;
        const have = p => GLib.file_test(p, GLib.FileTest.EXISTS);
        const set = (id, key, value) => {
            const s = settingsFor(id);
            if (!s || !s.schema.has_key(key))
                return;
            // Only where you haven't chosen something yourself.
            if (s.settings.get_user_value(key) !== null)
                return;
            try {
                s.settings.set_value(key, value);
            } catch (e) {
                console.warn(`Hearth look: ${id} ${key}: ${e.message}`);
            }
        };
        const str = v => new GLib.Variant('s', v);
        if (have(WALLPAPER)) {
            const uri = `file://${WALLPAPER}`;
            set('org.gnome.desktop.background', 'picture-uri', str(uri));
            set('org.gnome.desktop.background', 'picture-uri-dark', str(uri));
            set('org.gnome.desktop.background', 'picture-options', str('zoom'));
            set('org.gnome.desktop.screensaver', 'picture-uri', str(uri));
        }
        set('org.gnome.desktop.background', 'primary-color', str('#0f0b09'));
        set('org.gnome.desktop.interface', 'color-scheme', str('prefer-dark'));
        set('org.gnome.desktop.interface', 'accent-color', str('orange'));
        set('org.gnome.desktop.interface', 'clock-show-date', new GLib.Variant('b', true));
        if (have('/usr/share/icons/breeze-dark'))
            set('org.gnome.desktop.interface', 'icon-theme', str('breeze-dark'));
        if (have('/usr/share/icons/breeze_cursors'))
            set('org.gnome.desktop.interface', 'cursor-theme', str('breeze_cursors'));
        set('org.gnome.desktop.wm.preferences', 'button-layout', str('appmenu:minimize,maximize,close'));
        set('org.gnome.mutter', 'center-new-windows', new GLib.Variant('b', true));
        set('org.gnome.desktop.screensaver', 'lock-enabled', new GLib.Variant('b', false));
        set('org.gnome.desktop.a11y.applications', 'screen-keyboard-enabled', new GLib.Variant('b', false));
        // GNOME Console in its dark look, close to Hearth Terminal's.
        set('org.gnome.Console', 'theme', str('night'));
        const apps = Shell.AppSystem.get_default();
        const pins = PINS.filter(id => apps.lookup_app(id));
        if (pins.length)
            set('org.gnome.shell', 'favorite-apps', new GLib.Variant('as', pins));
        GLib.mkdir_with_parents(GLib.path_get_dirname(mark), 0o755);
        GLib.file_set_contents(mark, 'Hearth GNOME defaults applied\n');
    }
}
