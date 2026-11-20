/* Test tooling for files-drag-out.sh (CI only; after firehold-fakeinput.c on feature/firehold-plasma-1.16): the pointer
 * moved, pressed and let go through KWin's own fake input (org_kde_kwin_fake_input), which works on its virtual
 * backend. KWin offers it only to programs whose desktop file asks for it (X-KDE-Wayland-Interfaces).
 *   files-fakeinput drag X1 Y1 X2 Y2 STEPS HOLD_MS   press at X1,Y1, move to X2,Y2 in STEPS steps, hold, let go
 *   files-fakeinput click X Y
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

static void at(struct wl_display *dpy, double x, double y)
{
    org_kde_kwin_fake_input_pointer_motion_absolute(fi, wl_fixed_from_double(x), wl_fixed_from_double(y));
    wl_display_roundtrip(dpy);
}
static void button(struct wl_display *dpy, int down)
{
    org_kde_kwin_fake_input_button(fi, 0x110, down ? WL_POINTER_BUTTON_STATE_PRESSED : WL_POINTER_BUTTON_STATE_RELEASED);
    wl_display_roundtrip(dpy);
}

int main(int argc, char **argv)
{
    if (argc < 4) { fprintf(stderr, "usage: %s drag X1 Y1 X2 Y2 STEPS HOLD_MS | click X Y\n", argv[0]); return 2; }
    struct wl_display *dpy = wl_display_connect(NULL);
    if (!dpy) { fprintf(stderr, "no Wayland display\n"); return 1; }
    struct wl_registry *reg = wl_display_get_registry(dpy);
    wl_registry_add_listener(reg, &listener, NULL);
    wl_display_roundtrip(dpy);
    if (!fi) { fprintf(stderr, "KWin doesn't offer org_kde_kwin_fake_input to this program\n"); return 3; }
    org_kde_kwin_fake_input_authenticate(fi, "Hearth CI", "dragging a file out of Hearth Files");
    wl_display_roundtrip(dpy);
    if (strcmp(argv[1], "click") == 0) {
        at(dpy, atof(argv[2]), atof(argv[3]));
        usleep(200000);
        button(dpy, 1);
        usleep(80000);
        button(dpy, 0);
    } else if (strcmp(argv[1], "drag") == 0 && argc >= 8) {
        double x1 = atof(argv[2]), y1 = atof(argv[3]), x2 = atof(argv[4]), y2 = atof(argv[5]);
        int steps = atoi(argv[6]), hold = atoi(argv[7]);
        at(dpy, x1, y1);
        usleep(300000);
        button(dpy, 1);
        usleep(150000);
        for (int i = 1; i <= steps; i++) {
            at(dpy, x1 + (x2 - x1) * i / steps, y1 + (y2 - y1) * i / steps);
            usleep(40000);
        }
        /* a little wiggle over the target, as a hand does (targets update on motion) */
        for (int i = 0; i < 6; i++) { at(dpy, x2 + (i % 2 ? 3 : -3), y2); usleep(60000); }
        usleep(hold * 1000);
        button(dpy, 0);
        usleep(300000);
    } else {
        fprintf(stderr, "unknown: %s\n", argv[1]);
        return 2;
    }
    wl_display_disconnect(dpy);
    return 0;
}
