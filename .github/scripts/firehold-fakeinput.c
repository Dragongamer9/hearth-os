/* Test tooling for firehold-plasma-shots.sh (CI only): presses and clicks through KWin's own fake input
 * (org_kde_kwin_fake_input), which works on its virtual backend. KWin offers it only to programs whose desktop file
 * asks for it (X-KDE-Wayland-Interfaces), so the CI script installs one for this program.
 *   firehold-fakeinput click X Y        the left button at X,Y (the whole desktop's coordinates)
 *   firehold-fakeinput keys CODE...     these Linux key codes held down in turn, then let go (56 33 = Alt+F)
 * Built in CI with wayland-scanner and plasma-wayland-protocols' fake-input.xml. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <wayland-client.h>
#include "fake-input-client-protocol.h"

static struct org_kde_kwin_fake_input *fi;

static void global(void *d, struct wl_registry *r, uint32_t name, const char *iface, uint32_t ver)
{
    (void)d;
    if (strcmp(iface, org_kde_kwin_fake_input_interface.name) == 0)
        fi = wl_registry_bind(r, name, &org_kde_kwin_fake_input_interface, ver < 4 ? ver : 4);
}
static void global_remove(void *d, struct wl_registry *r, uint32_t name) { (void)d; (void)r; (void)name; }
static const struct wl_registry_listener listener = { global, global_remove };

int main(int argc, char **argv)
{
    if (argc < 3) { fprintf(stderr, "usage: %s click X Y | keys CODE...\n", argv[0]); return 2; }
    struct wl_display *dpy = wl_display_connect(NULL);
    if (!dpy) { fprintf(stderr, "no Wayland display\n"); return 1; }
    struct wl_registry *reg = wl_display_get_registry(dpy);
    wl_registry_add_listener(reg, &listener, NULL);
    wl_display_roundtrip(dpy);
    if (!fi) { fprintf(stderr, "KWin doesn't offer org_kde_kwin_fake_input to this program\n"); return 3; }
    org_kde_kwin_fake_input_authenticate(fi, "Hearth CI", "pictures of the bar's menus");
    wl_display_roundtrip(dpy);
    if (strcmp(argv[1], "click") == 0 && argc >= 4) {
        org_kde_kwin_fake_input_pointer_motion_absolute(fi, wl_fixed_from_double(atof(argv[2])), wl_fixed_from_double(atof(argv[3])));
        wl_display_roundtrip(dpy);
        usleep(200000);
        org_kde_kwin_fake_input_button(fi, 0x110, WL_POINTER_BUTTON_STATE_PRESSED);
        wl_display_roundtrip(dpy);
        usleep(80000);
        org_kde_kwin_fake_input_button(fi, 0x110, WL_POINTER_BUTTON_STATE_RELEASED);
        wl_display_roundtrip(dpy);
    } else if (strcmp(argv[1], "keys") == 0) {
        for (int i = 2; i < argc; i++) { org_kde_kwin_fake_input_keyboard_key(fi, (uint32_t)atoi(argv[i]), 1); wl_display_roundtrip(dpy); usleep(50000); }
        for (int i = argc - 1; i >= 2; i--) { org_kde_kwin_fake_input_keyboard_key(fi, (uint32_t)atoi(argv[i]), 0); wl_display_roundtrip(dpy); usleep(50000); }
    } else {
        fprintf(stderr, "unknown: %s\n", argv[1]);
        return 2;
    }
    wl_display_disconnect(dpy);
    return 0;
}
