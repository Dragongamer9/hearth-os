/* hearth-kwin-cast: screen sharing and screenshots from KWin, for Hearth's portal backend (hearth_screencast.py,
 * docs/KWIN-INPUT.md).
 *
 * KWin hands out screen-sharing streams (zkde_screencast_unstable_v1: a PipeWire node per screen, window or area)
 * and screenshots (org.kde.KWin.ScreenShot2 on D-Bus) only to programs whose own .desktop file asks for them
 * (X-KDE-Wayland-Interfaces / X-KDE-DBUS-Restricted-Interfaces; KWin finds the file by this program's path). A
 * Python script can't be told apart from every other Python program, so this small program does those two calls
 * and nothing else: /usr/share/applications/hearth-kwin-cast.desktop names /usr/libexec/hearth/hearth-kwin-cast.
 *
 *   hearth-kwin-cast outputs
 *       one line per screen: "output NAME<TAB>DESCRIPTION<TAB>X<TAB>Y<TAB>WIDTH<TAB>HEIGHT<TAB>SCALE"
 *   hearth-kwin-cast stream output NAME CURSOR | window UUID CURSOR | region X Y W H CURSOR
 *       starts a stream (CURSOR: 1 hidden, 2 in the picture, 4 as metadata) and prints "node N" (the PipeWire node)
 *       once KWin has made it; it lasts until stdin closes or SIGTERM, then "closed". "failed WHY" if it couldn't.
 *   hearth-kwin-cast shot screen NAME | window UUID | workspace | area X Y W H  [CURSOR 0|1]
 *       a screenshot: one line "W H STRIDE FORMAT SCALE" (FORMAT: Qt's QImage format number) and the raw pixels.
 * Exit codes: 1 failed, 2 KWin didn't give this program what it asked for (its .desktop file isn't installed).
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <pthread.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <gio/gio.h>
#include <gio/gunixfdlist.h>
#include <wayland-client.h>
#include "screencast-client-protocol.h"

struct output {
    struct wl_output *wl;
    char name[128];
    char desc[256];
    int x, y, w, h, scale;
    struct output *next;
};

static struct wl_display *display;
static struct zkde_screencast_unstable_v1 *screencast;
static struct output *outputs;
static int done_stream;        /* 1 created, -1 failed, 2 closed */

/* ------------------------------------------------------------------ screens */

static void out_geometry(void *d, struct wl_output *o, int32_t x, int32_t y, int32_t pw, int32_t ph, int32_t sub,
                         const char *make, const char *model, int32_t tr) {
    struct output *op = d;
    (void)o; (void)pw; (void)ph; (void)sub; (void)make; (void)model; (void)tr;
    op->x = x;
    op->y = y;
}
static void out_mode(void *d, struct wl_output *o, uint32_t flags, int32_t w, int32_t h, int32_t refresh) {
    struct output *op = d;
    (void)o; (void)refresh;
    if (flags & WL_OUTPUT_MODE_CURRENT) { op->w = w; op->h = h; }
}
static void out_done(void *d, struct wl_output *o) { (void)d; (void)o; }
static void out_scale(void *d, struct wl_output *o, int32_t f) { struct output *op = d; (void)o; op->scale = f; }
static void out_name(void *d, struct wl_output *o, const char *name) {
    struct output *op = d;
    (void)o;
    snprintf(op->name, sizeof op->name, "%s", name);
}
static void out_desc(void *d, struct wl_output *o, const char *desc) {
    struct output *op = d;
    (void)o;
    snprintf(op->desc, sizeof op->desc, "%s", desc);
    for (char *p = op->desc; *p; p++) if (*p == '\t' || *p == '\n') *p = ' ';
}
static const struct wl_output_listener out_listener = {
    .geometry = out_geometry, .mode = out_mode, .done = out_done, .scale = out_scale, .name = out_name,
    .description = out_desc,
};

static void reg_global(void *d, struct wl_registry *r, uint32_t name, const char *iface, uint32_t version) {
    (void)d;
    if (strcmp(iface, wl_output_interface.name) == 0) {
        if (version < 4) return;      /* KWin 6 has v4: screens are known by name */
        struct output *o = calloc(1, sizeof *o);
        if (!o) return;
        o->scale = 1;
        o->wl = wl_registry_bind(r, name, &wl_output_interface, 4);
        wl_output_add_listener(o->wl, &out_listener, o);
        o->next = outputs;
        outputs = o;
    } else if (strcmp(iface, zkde_screencast_unstable_v1_interface.name) == 0 && !screencast) {
        uint32_t v = version < 4 ? version : 4;
        screencast = wl_registry_bind(r, name, &zkde_screencast_unstable_v1_interface, v);
    }
}
static void reg_remove(void *d, struct wl_registry *r, uint32_t name) { (void)d; (void)r; (void)name; }
static const struct wl_registry_listener reg_listener = { .global = reg_global, .global_remove = reg_remove };

static int connect_wayland(void) {
    display = wl_display_connect(NULL);
    if (!display) { fprintf(stderr, "hearth-kwin-cast: can't reach the compositor\n"); return -1; }
    struct wl_registry *reg = wl_display_get_registry(display);
    wl_registry_add_listener(reg, &reg_listener, NULL);
    wl_display_roundtrip(display);
    wl_display_roundtrip(display);
    return 0;
}

/* ------------------------------------------------------------------ streams */

static void st_closed(void *d, struct zkde_screencast_stream_unstable_v1 *s) {
    (void)d; (void)s;
    printf("closed\n");
    fflush(stdout);
    done_stream = 2;
}
static void st_created(void *d, struct zkde_screencast_stream_unstable_v1 *s, uint32_t node) {
    (void)d; (void)s;
    printf("node %u\n", node);
    fflush(stdout);
    done_stream = 1;
}
static void st_failed(void *d, struct zkde_screencast_stream_unstable_v1 *s, const char *why) {
    (void)d; (void)s;
    printf("failed %s\n", why ? why : "");
    fflush(stdout);
    done_stream = -1;
}
static const struct zkde_screencast_stream_unstable_v1_listener st_listener = {
    .closed = st_closed, .created = st_created, .failed = st_failed,
};

static volatile sig_atomic_t stop;
static void on_signal(int s) { (void)s; stop = 1; }

static int do_stream(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "hearth-kwin-cast: stream output|window|region ...\n"); return 1; }
    if (connect_wayland() < 0) return 1;
    if (!screencast) {
        printf("failed KWin didn't give zkde_screencast_unstable_v1 to this program (hearth-kwin-cast.desktop)\n");
        return 2;
    }
    struct zkde_screencast_stream_unstable_v1 *st = NULL;
    const char *kind = argv[0];
    if (strcmp(kind, "output") == 0 && argc >= 3) {
        uint32_t cursor = (uint32_t)strtoul(argv[2], NULL, 10);
        for (struct output *o = outputs; o; o = o->next)
            if (strcmp(o->name, argv[1]) == 0) { st = zkde_screencast_unstable_v1_stream_output(screencast, o->wl, cursor); break; }
        if (!st) { printf("failed no screen called %s\n", argv[1]); return 1; }
    } else if (strcmp(kind, "window") == 0 && argc >= 3) {
        st = zkde_screencast_unstable_v1_stream_window(screencast, argv[1], (uint32_t)strtoul(argv[2], NULL, 10));
    } else if (strcmp(kind, "region") == 0 && argc >= 6) {
        if (zkde_screencast_unstable_v1_get_version(screencast) < 3) { printf("failed this KWin can't share an area\n"); return 1; }
        st = zkde_screencast_unstable_v1_stream_region(screencast, atoi(argv[1]), atoi(argv[2]),
                                                       (uint32_t)strtoul(argv[3], NULL, 10),
                                                       (uint32_t)strtoul(argv[4], NULL, 10), wl_fixed_from_int(1),
                                                       (uint32_t)strtoul(argv[5], NULL, 10));
    } else {
        fprintf(stderr, "hearth-kwin-cast: stream output NAME CURSOR | window UUID CURSOR | region X Y W H CURSOR\n");
        return 1;
    }
    zkde_screencast_stream_unstable_v1_add_listener(st, &st_listener, NULL);
    signal(SIGTERM, on_signal);
    signal(SIGINT, on_signal);
    signal(SIGPIPE, SIG_IGN);
    while (!stop && done_stream >= 0 && done_stream != 2) {
        struct pollfd p[2] = { { wl_display_get_fd(display), POLLIN, 0 }, { 0, POLLIN, 0 } };
        while (wl_display_prepare_read(display) != 0) wl_display_dispatch_pending(display);
        wl_display_flush(display);
        if (poll(p, 2, -1) < 0) {
            wl_display_cancel_read(display);
            if (errno == EINTR) continue;
            break;
        }
        if (p[0].revents & POLLIN) { if (wl_display_read_events(display) < 0) break; }
        else wl_display_cancel_read(display);
        if (wl_display_dispatch_pending(display) < 0) break;
        if (p[0].revents & (POLLERR | POLLHUP)) break;
        if (p[1].revents & (POLLIN | POLLHUP)) {
            char b[256];
            ssize_t r = read(0, b, sizeof b);
            if (r <= 0) break;        /* the portal backend is done with it */
        }
    }
    if (done_stream != 2) {
        zkde_screencast_stream_unstable_v1_close(st);
        wl_display_roundtrip(display);
        if (done_stream == 1) { printf("closed\n"); fflush(stdout); }
    }
    return done_stream < 0 ? 1 : 0;
}

static int do_outputs(void) {
    if (connect_wayland() < 0) return 1;
    for (struct output *o = outputs; o; o = o->next)
        printf("output %s\t%s\t%d\t%d\t%d\t%d\t%d\n", o->name, o->desc, o->x, o->y, o->w, o->h, o->scale);
    printf("screencast %u\n", screencast ? zkde_screencast_unstable_v1_get_version(screencast) : 0);
    return 0;
}

/* ------------------------------------------------------------------ screenshots (org.kde.KWin.ScreenShot2) */

struct reader { int fd; GByteArray *data; };

static void *read_all(void *arg) {
    struct reader *r = arg;
    guint8 buf[1 << 16];
    for (;;) {
        ssize_t n = read(r->fd, buf, sizeof buf);
        if (n < 0 && errno == EINTR) continue;
        if (n <= 0) break;
        g_byte_array_append(r->data, buf, (guint)n);
    }
    return NULL;
}

static guint32 dict_u(GVariant *d, const char *k) {
    guint32 v = 0;
    if (!g_variant_lookup(d, k, "u", &v)) {
        gint32 i = 0;
        if (g_variant_lookup(d, k, "i", &i)) v = (guint32)i;
    }
    return v;
}

static int do_shot(int argc, char **argv) {
    if (argc < 1) { fprintf(stderr, "hearth-kwin-cast: shot screen NAME | window UUID | workspace | area X Y W H\n"); return 1; }
    GError *err = NULL;
    GDBusConnection *bus = g_bus_get_sync(G_BUS_TYPE_SESSION, NULL, &err);
    if (!bus) { fprintf(stderr, "hearth-kwin-cast: no session bus: %s\n", err->message); return 1; }
    int pfd[2];
    if (pipe2(pfd, O_CLOEXEC) < 0) return 1;
    GVariantBuilder ob;
    g_variant_builder_init(&ob, G_VARIANT_TYPE("a{sv}"));
    const char *kind = argv[0];
    int rest = 1;
    if (strcmp(kind, "screen") == 0 || strcmp(kind, "window") == 0) rest = 2;
    else if (strcmp(kind, "area") == 0) rest = 5;
    int cursor = argc > rest ? atoi(argv[rest]) : 0;
    g_variant_builder_add(&ob, "{sv}", "include-cursor", g_variant_new_boolean(cursor != 0));
    g_variant_builder_add(&ob, "{sv}", "native-resolution", g_variant_new_boolean(TRUE));
    if (strcmp(kind, "window") == 0) {
        g_variant_builder_add(&ob, "{sv}", "include-decoration", g_variant_new_boolean(TRUE));
        g_variant_builder_add(&ob, "{sv}", "include-shadow", g_variant_new_boolean(FALSE));
    }
    GVariant *params;
    const char *method;
    if (strcmp(kind, "screen") == 0 && argc >= 2) { method = "CaptureScreen"; params = g_variant_new("(sa{sv}h)", argv[1], &ob, 0); }
    else if (strcmp(kind, "window") == 0 && argc >= 2) { method = "CaptureWindow"; params = g_variant_new("(sa{sv}h)", argv[1], &ob, 0); }
    else if (strcmp(kind, "workspace") == 0) { method = "CaptureWorkspace"; params = g_variant_new("(a{sv}h)", &ob, 0); }
    else if (strcmp(kind, "area") == 0 && argc >= 5) {
        method = "CaptureArea";
        params = g_variant_new("(iiuua{sv}h)", atoi(argv[1]), atoi(argv[2]), (guint32)strtoul(argv[3], NULL, 10),
                               (guint32)strtoul(argv[4], NULL, 10), &ob, 0);
    } else { fprintf(stderr, "hearth-kwin-cast: shot screen NAME | window UUID | workspace | area X Y W H\n"); return 1; }
    GUnixFDList *fds = g_unix_fd_list_new_from_array(&pfd[1], 1);   /* takes the write end */
    struct reader rd = { pfd[0], g_byte_array_new() };
    pthread_t th;
    pthread_create(&th, NULL, read_all, &rd);
    GVariant *res = g_dbus_connection_call_with_unix_fd_list_sync(bus, "org.kde.KWin", "/org/kde/KWin/ScreenShot2",
        "org.kde.KWin.ScreenShot2", method, params, G_VARIANT_TYPE("(a{sv})"), G_DBUS_CALL_FLAGS_NONE, 15000, fds, NULL,
        NULL, &err);
    g_object_unref(fds);           /* closes our copy of the write end: the reader sees the end of the picture */
    if (!res) {
        fprintf(stderr, "hearth-kwin-cast: %s failed: %s\n", method, err->message);
        int code = (strstr(err->message, "NoAuthorized") || strstr(err->message, "not authorized") ||
                    strstr(err->message, "Unauthorized")) ? 2 : 1;
        close(pfd[0]);
        pthread_cancel(th);
        return code;
    }
    pthread_join(th, NULL);
    close(pfd[0]);
    GVariant *d = g_variant_get_child_value(res, 0);
    guint32 w = dict_u(d, "width"), h = dict_u(d, "height"), stride = dict_u(d, "stride"), fmt = dict_u(d, "format");
    double scale = 1.0;
    g_variant_lookup(d, "scale", "d", &scale);
    if (!w || !h || !stride || rd.data->len < (guint)stride * h) {
        fprintf(stderr, "hearth-kwin-cast: KWin's picture was cut short (%u bytes for %ux%u)\n", rd.data->len, w, h);
        return 1;
    }
    printf("%u %u %u %u %g\n", w, h, stride, fmt, scale);
    fflush(stdout);
    size_t off = 0, n = (size_t)stride * h;
    while (off < n) {
        ssize_t wr = write(1, rd.data->data + off, n - off);
        if (wr < 0) { if (errno == EINTR) continue; return 1; }
        off += (size_t)wr;
    }
    return 0;
}

int main(int argc, char **argv) {
    if (argc >= 2 && strcmp(argv[1], "outputs") == 0) return do_outputs();
    if (argc >= 2 && strcmp(argv[1], "stream") == 0) return do_stream(argc - 2, argv + 2);
    if (argc >= 2 && strcmp(argv[1], "shot") == 0) return do_shot(argc - 2, argv + 2);
    fprintf(stderr, "usage: hearth-kwin-cast outputs | stream ... | shot ...\n");
    return 1;
}
