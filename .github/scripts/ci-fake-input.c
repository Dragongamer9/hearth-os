/* CI only (hearth-apps-rust.yml's kwin job, plasma-hearth.yml's extras): keys and the pointer for a KWin on a virtual
 * screen, which reads no keyboards of its own, through KWin's fake input (org_kde_kwin_fake_input; KWin gives it to
 * this program by its path, /usr/local/bin/ci-fake-input, through ci-fake-input.desktop's X-KDE-Wayland-Interfaces).
 * The keys reach KWin as typed keys do: its global shortcuts (Print Screen, Super+Shift+S) and Alt+Tab held down.
 *   ci-fake-input CMD...   "k CODE 1|0" a key down / up (evdev codes: 56 Alt, 15 Tab, 99 Print, 125 Super,
 *                          42 Shift, 31 S, 28 Enter, 1 Esc, 106 Right, 108 Down), "a X Y" the pointer to a place
 *                          (KWin's coordinates), "b CODE 1|0" a button (272 left), "s MS" wait.
 * Every key still down at the end is let go. Exit 2: KWin didn't give this program fake input. */
#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <wayland-client.h>
#include "fake-input-client-protocol.h"

static struct org_kde_kwin_fake_input *fake;

static void reg_global(void *d, struct wl_registry *r, uint32_t name, const char *iface, uint32_t v) {
    (void)d;
    if (strcmp(iface, org_kde_kwin_fake_input_interface.name) == 0 && !fake)
        fake = wl_registry_bind(r, name, &org_kde_kwin_fake_input_interface, v < 5 ? v : 5);
}
static void reg_remove(void *d, struct wl_registry *r, uint32_t name) { (void)d; (void)r; (void)name; }
static const struct wl_registry_listener reg = { reg_global, reg_remove };

static void wait_ms(long ms) {
    struct timespec t = { ms / 1000, (ms % 1000) * 1000000L };
    nanosleep(&t, NULL);
}

int main(int argc, char **argv) {
    struct wl_display *dpy = wl_display_connect(NULL);
    if (!dpy) { fprintf(stderr, "ci-fake-input: no Wayland display\n"); return 1; }
    struct wl_registry *r = wl_display_get_registry(dpy);
    wl_registry_add_listener(r, &reg, NULL);
    wl_display_roundtrip(dpy);
    if (!fake) { fprintf(stderr, "ci-fake-input: KWin didn't give org_kde_kwin_fake_input to this program\n"); return 2; }
    org_kde_kwin_fake_input_authenticate(fake, "Hearth CI", "Pressing keys for the checks");
    wl_display_roundtrip(dpy);
    int down[0x300] = {0};
    for (int i = 1; i < argc; i++) {
        int a = 0, b = 0;
        double x = 0, y = 0;
        const char *c = argv[i];
        if (c[0] == 'k' && sscanf(c + 1, "%d %d", &a, &b) == 2 && a > 0 && a < 0x300) {
            org_kde_kwin_fake_input_keyboard_key(fake, (uint32_t)a, b ? WL_KEYBOARD_KEY_STATE_PRESSED : WL_KEYBOARD_KEY_STATE_RELEASED);
            down[a] = b;
        } else if (c[0] == 'a' && sscanf(c + 1, "%lf %lf", &x, &y) == 2) {
            org_kde_kwin_fake_input_pointer_motion_absolute(fake, wl_fixed_from_double(x), wl_fixed_from_double(y));
        } else if (c[0] == 'b' && sscanf(c + 1, "%d %d", &a, &b) == 2 && a > 0) {
            org_kde_kwin_fake_input_button(fake, (uint32_t)a, b ? WL_POINTER_BUTTON_STATE_PRESSED : WL_POINTER_BUTTON_STATE_RELEASED);
        } else if (c[0] == 's' && sscanf(c + 1, "%d", &a) == 1) {
            wl_display_roundtrip(dpy);
            wait_ms(a);
            continue;
        } else {
            fprintf(stderr, "ci-fake-input: what is \"%s\"?\n", c);
            continue;
        }
        wl_display_roundtrip(dpy);
        wait_ms(25);
    }
    for (int k = 0; k < 0x300; k++)
        if (down[k]) org_kde_kwin_fake_input_keyboard_key(fake, (uint32_t)k, WL_KEYBOARD_KEY_STATE_RELEASED);
    wl_display_roundtrip(dpy);
    wl_display_disconnect(dpy);
    return 0;
}
