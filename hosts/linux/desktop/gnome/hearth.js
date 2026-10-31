// Hearth GNOME: what every Hearth GNOME Shell extension (os/system_files/usr/share/gnome-shell/extensions/*@hearthos.fun)
// shares: running Hearth's helpers (the same ones Mantel and Hearth Plasma's widgets use: desktop/gnome/hearth-gnome,
// which is hearth-plasma with GNOME's app list, desktop/shell/hearth-link-settings, nimbus-mode, hearth-settings),
// reading their one-line JSON answers, and icons by name or path. Installed at /usr/share/nimbus/desktop/gnome/hearth.js;
// the extensions import it by that path. Nothing here throws: a missing helper answers null. docs/GNOME.md.
import Gio from 'gi://Gio';
import GLib from 'gi://GLib';

export const SHARE = GLib.getenv('NIMBUS_SHARE') || '/usr/share/nimbus';
export const HELPER = `${SHARE}/desktop/gnome/hearth-gnome`;
export const LINK = `${SHARE}/desktop/shell/hearth-link-settings`;
export const PAD = `${SHARE}/desktop/shell/hearth-pad`;
export const OSK_DIR = `${SHARE}/desktop/shell/osk`;

export function exists(path) {
    return !!path && GLib.file_test(path, GLib.FileTest.EXISTS);
}

/** Starts argv in the background (nothing waits for it). */
export function spawn(argv) {
    try {
        const p = Gio.Subprocess.new(argv, Gio.SubprocessFlags.STDOUT_SILENCE | Gio.SubprocessFlags.STDERR_SILENCE);
        p.wait_async(null, () => {});
        return true;
    } catch (e) {
        console.warn(`Hearth: couldn't start ${argv[0]}: ${e.message}`);
        return false;
    }
}

/** A shell command line in the background (sh -c). */
export function shell(cmd) {
    return spawn(['sh', '-c', cmd]);
}

/** Runs argv and resolves with its output ('' when it couldn't run). */
export function run(argv) {
    return new Promise(resolve => {
        let p;
        try {
            p = Gio.Subprocess.new(argv, Gio.SubprocessFlags.STDOUT_PIPE | Gio.SubprocessFlags.STDERR_SILENCE);
        } catch (e) {
            resolve('');
            return;
        }
        p.communicate_utf8_async(null, null, (proc, res) => {
            try {
                const [, out] = proc.communicate_utf8_finish(res);
                resolve(out || '');
            } catch (e) {
                resolve('');
            }
        });
    });
}

/** The last line of a helper's output as JSON: {error} for "error: ...", null when there's nothing to read. */
export function lastJson(out) {
    const lines = String(out || '').trim().split('\n');
    const l = lines[lines.length - 1] || '';
    if (l.startsWith('error: '))
        return {error: l.slice(7)};
    try {
        return JSON.parse(l);
    } catch (e) {
        return null;
    }
}

/** hearth-gnome (Hearth's apps, games, Quick Access values). */
export async function helper(...args) {
    if (!exists(HELPER))
        return null;
    return lastJson(await run(['python3', HELPER, ...args]));
}

/** hearth-link-settings (Hearth Link and Hearth Storage), as Mantel's Settings › Hearth Link uses it. */
export async function link(...args) {
    if (!exists(LINK))
        return null;
    return lastJson(await run(['python3', LINK, ...args]));
}

/** Back to Game Mode (nimbus-mode ends GNOME properly and starts Game Mode in the same session). */
export function gameMode() {
    return spawn(['nimbus-mode', 'game']);
}

/** Hearth Settings (optionally at a page). */
export function settings(page) {
    return spawn(page ? ['hearth-settings', page] : ['hearth-settings']);
}

/** An icon by theme name, or by file path (.desktop files and Hearth's games can give either). */
export function icon(name, fallback = 'application-x-executable') {
    try {
        if (name && name.startsWith('/') && exists(name))
            return Gio.FileIcon.new(Gio.File.new_for_path(name));
        if (name && !name.includes('/'))
            return Gio.ThemedIcon.new_with_default_fallbacks(name.replace(/\.(png|svg|xpm)$/, ''));
    } catch (e) {
        // fall through
    }
    return Gio.ThemedIcon.new(fallback);
}

/** Reads a text file ('' when it isn't there). */
export function readText(path) {
    try {
        const [ok, bytes] = GLib.file_get_contents(path);
        return ok ? new TextDecoder().decode(bytes) : '';
    } catch (e) {
        return '';
    }
}

/** Writes a text file (its folder made first); false on failure. */
export function writeText(path, text) {
    try {
        GLib.mkdir_with_parents(GLib.path_get_dirname(path), 0o755);
        return GLib.file_set_contents(path, text);
    } catch (e) {
        return false;
    }
}

/** A repeating GLib timer; returns a function that stops it. */
export function every(ms, fn) {
    const id = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
        try {
            fn();
        } catch (e) {
            logError(e, 'Hearth timer');
        }
        return GLib.SOURCE_CONTINUE;
    });
    return () => GLib.source_remove(id);
}

/** Once, after ms; returns a function that cancels it. */
export function later(ms, fn) {
    let id = GLib.timeout_add(GLib.PRIORITY_DEFAULT, ms, () => {
        id = 0;
        try {
            fn();
        } catch (e) {
            logError(e, 'Hearth timer');
        }
        return GLib.SOURCE_REMOVE;
    });
    return () => {
        if (id)
            GLib.source_remove(id);
        id = 0;
    };
}
