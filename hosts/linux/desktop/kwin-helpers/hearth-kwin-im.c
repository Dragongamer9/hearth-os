/* hearth-kwin-im: Hearth's input method for KWin (docs/KWIN-INPUT.md).
 *
 * KWin lets only its input method type text into apps (it has no virtual-keyboard protocol, which wtype needs).
 * KWin starts this program itself (kwinrc [Wayland] InputMethod=/usr/share/applications/hearth-kwin-im.desktop)
 * and gives it zwp_input_method_v1. Whenever a text field has the keyboard (an app speaking text-input v1/v2/v3:
 * Qt, GTK, Firefox, Chromium/Electron with Wayland IME on), KWin hands it a context; text sent here is committed
 * into that field whole, any character (accents, other scripts, emoji), whatever the keyboard layout is.
 *
 * Mantel's on-screen keyboard, clipboard paste and the controller type through hearth-type, which asks this
 * program first and falls back to its virtual keyboard (uinput) when no text field is active.
 *
 * Socket: $XDG_RUNTIME_DIR/hearth/im.sock (folder 0700, socket 0600, only this user: SO_PEERCRED). One line per
 * command, one line back:
 *   S                 "active 1 purpose P hint H" or "active 0 purpose 0 hint 0"
 *   T <text>          commit the text (%-escaped: %25 %0A %0D) into the active field: "ok", or "no" if none
 *   K <keysym> <mods> a key as a keysym (press and release) to the active field: "ok" / "no"
 *   W                 watch: the state now, then a line "active ..." every time it changes (a text field got or
 *                     lost the keyboard), for the on-screen keyboard to open by itself
 * It never reads the text fields (surrounding text is ignored) and keeps nothing.
 */
#define _GNU_SOURCE
#include <errno.h>
#include <fcntl.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/un.h>
#include <time.h>
#include <unistd.h>
#include <wayland-client.h>
#include "input-method-unstable-v1-client-protocol.h"

#define MAX_CLIENTS 16
#define MAX_LINE 65536

struct client {
    int fd;
    int watch;
    size_t len;
    char buf[MAX_LINE];
};

static struct wl_display *display;
static struct zwp_input_method_v1 *im;
static struct zwp_input_method_context_v1 *ctx;
static uint32_t ctx_serial, ctx_purpose, ctx_hint;
static struct client *clients[MAX_CLIENTS];
static char sock_path[sizeof(((struct sockaddr_un *)0)->sun_path)];

static void say(struct client *c, const char *line) {
    size_t n = strlen(line);
    while (n) {
        ssize_t w = send(c->fd, line, n, MSG_NOSIGNAL);
        if (w < 0) {
            if (errno == EINTR) continue;
            return;
        }
        line += w;
        n -= (size_t)w;
    }
}

static void state_line(char *out, size_t n) {
    snprintf(out, n, "active %d purpose %u hint %u\n", ctx ? 1 : 0, ctx ? ctx_purpose : 0, ctx ? ctx_hint : 0);
}

static void tell_watchers(void) {
    char line[96];
    state_line(line, sizeof line);
    for (int i = 0; i < MAX_CLIENTS; i++)
        if (clients[i] && clients[i]->watch) say(clients[i], line);
}

/* ------------------------------------------------------------------ the context KWin gives while a field is active */

static void ctx_surrounding_text(void *d, struct zwp_input_method_context_v1 *c, const char *text, uint32_t cursor,
                                 uint32_t anchor) {
    (void)d; (void)c; (void)text; (void)cursor; (void)anchor;
}
static void ctx_reset(void *d, struct zwp_input_method_context_v1 *c) { (void)d; (void)c; }
static void ctx_content_type(void *d, struct zwp_input_method_context_v1 *c, uint32_t hint, uint32_t purpose) {
    (void)d;
    if (c != ctx) return;
    int changed = hint != ctx_hint || purpose != ctx_purpose;
    ctx_hint = hint;
    ctx_purpose = purpose;
    if (changed) tell_watchers();
}
static void ctx_invoke_action(void *d, struct zwp_input_method_context_v1 *c, uint32_t button, uint32_t index) {
    (void)d; (void)c; (void)button; (void)index;
}
static void ctx_commit_state(void *d, struct zwp_input_method_context_v1 *c, uint32_t serial) {
    (void)d;
    if (c == ctx) ctx_serial = serial;
}
static void ctx_preferred_language(void *d, struct zwp_input_method_context_v1 *c, const char *language) {
    (void)d; (void)c; (void)language;
}
static const struct zwp_input_method_context_v1_listener ctx_listener = {
    .surrounding_text = ctx_surrounding_text,
    .reset = ctx_reset,
    .content_type = ctx_content_type,
    .invoke_action = ctx_invoke_action,
    .commit_state = ctx_commit_state,
    .preferred_language = ctx_preferred_language,
};

static void im_activate(void *d, struct zwp_input_method_v1 *m, struct zwp_input_method_context_v1 *id) {
    (void)d; (void)m;
    if (ctx) zwp_input_method_context_v1_destroy(ctx);
    ctx = id;
    ctx_serial = 0;
    ctx_purpose = 0;
    ctx_hint = 0;
    zwp_input_method_context_v1_add_listener(ctx, &ctx_listener, NULL);
    fprintf(stderr, "hearth-kwin-im: a text field has the keyboard\n");
    tell_watchers();
}
static void im_deactivate(void *d, struct zwp_input_method_v1 *m, struct zwp_input_method_context_v1 *c) {
    (void)d; (void)m;
    if (c == ctx) ctx = NULL;
    zwp_input_method_context_v1_destroy(c);
    tell_watchers();
}
static const struct zwp_input_method_v1_listener im_listener = { .activate = im_activate, .deactivate = im_deactivate };

static void reg_global(void *d, struct wl_registry *r, uint32_t name, const char *iface, uint32_t version) {
    (void)d; (void)version;
    if (strcmp(iface, zwp_input_method_v1_interface.name) == 0 && !im) {
        im = wl_registry_bind(r, name, &zwp_input_method_v1_interface, 1);
        zwp_input_method_v1_add_listener(im, &im_listener, NULL);
    }
}
static void reg_remove(void *d, struct wl_registry *r, uint32_t name) { (void)d; (void)r; (void)name; }
static const struct wl_registry_listener reg_listener = { .global = reg_global, .global_remove = reg_remove };

/* ------------------------------------------------------------------ the socket */

static int unpct(char *s) {
    char *o = s;
    for (char *p = s; *p; p++) {
        if (*p == '%' && p[1] && p[2]) {
            char h[3] = { p[1], p[2], 0 };
            char *end;
            long v = strtol(h, &end, 16);
            if (*end) return -1;
            *o++ = (char)v;
            p += 2;
        } else {
            *o++ = *p;
        }
    }
    *o = 0;
    return (int)(o - s);
}

static uint32_t now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint32_t)(ts.tv_sec * 1000 + ts.tv_nsec / 1000000);
}

static void command(struct client *c, char *line) {
    char out[96];
    if (line[0] == 'S' && (line[1] == 0 || line[1] == ' ')) {
        state_line(out, sizeof out);
        say(c, out);
    } else if (line[0] == 'W' && line[1] == 0) {
        c->watch = 1;
        state_line(out, sizeof out);
        say(c, out);
    } else if (line[0] == 'T' && line[1] == ' ') {
        if (unpct(line + 2) < 0) { say(c, "error bad-escape\n"); return; }
        if (!ctx) { say(c, "no\n"); return; }
        zwp_input_method_context_v1_commit_string(ctx, ctx_serial, line + 2);
        /* Wait until KWin has taken it, so keys typed right after (uinput) come after the text. */
        wl_display_roundtrip(display);
        say(c, ctx ? "ok\n" : "no\n");
    } else if (line[0] == 'K' && line[1] == ' ') {
        unsigned long sym = 0, mods = 0;
        if (sscanf(line + 2, "%lu %lu", &sym, &mods) < 1) { say(c, "error bad-key\n"); return; }
        if (!ctx) { say(c, "no\n"); return; }
        uint32_t t = now_ms();
        zwp_input_method_context_v1_keysym(ctx, ctx_serial, t, (uint32_t)sym, WL_KEYBOARD_KEY_STATE_PRESSED, (uint32_t)mods);
        zwp_input_method_context_v1_keysym(ctx, ctx_serial, t + 1, (uint32_t)sym, WL_KEYBOARD_KEY_STATE_RELEASED, (uint32_t)mods);
        wl_display_roundtrip(display);
        say(c, ctx ? "ok\n" : "no\n");
    } else {
        say(c, "error unknown-command\n");
    }
}

static int same_user(int fd) {
    struct ucred cr;
    socklen_t n = sizeof cr;
    return getsockopt(fd, SOL_SOCKET, SO_PEERCRED, &cr, &n) == 0 && cr.uid == getuid();
}

static int listen_socket(void) {
    const char *rt = getenv("XDG_RUNTIME_DIR");
    if (!rt || !*rt) { fprintf(stderr, "hearth-kwin-im: XDG_RUNTIME_DIR isn't set\n"); return -1; }
    char dir[sizeof sock_path];
    if (snprintf(dir, sizeof dir, "%s/hearth", rt) >= (int)sizeof dir) return -1;
    mkdir(dir, 0700);
    chmod(dir, 0700);
    if (snprintf(sock_path, sizeof sock_path, "%s/im.sock", dir) >= (int)sizeof sock_path) return -1;
    int fd = socket(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC | SOCK_NONBLOCK, 0);
    if (fd < 0) return -1;
    struct sockaddr_un a = { .sun_family = AF_UNIX };
    strcpy(a.sun_path, sock_path);
    unlink(sock_path);
    if (bind(fd, (struct sockaddr *)&a, sizeof a) < 0 || listen(fd, 8) < 0) { close(fd); return -1; }
    chmod(sock_path, 0600);
    return fd;
}

static volatile sig_atomic_t stop;
static void on_signal(int s) { (void)s; stop = 1; }

int main(void) {
    signal(SIGTERM, on_signal);
    signal(SIGINT, on_signal);
    signal(SIGPIPE, SIG_IGN);
    display = wl_display_connect(NULL);
    if (!display) { fprintf(stderr, "hearth-kwin-im: can't reach the compositor\n"); return 1; }
    struct wl_registry *reg = wl_display_get_registry(display);
    wl_registry_add_listener(reg, &reg_listener, NULL);
    wl_display_roundtrip(display);
    if (!im) {
        fprintf(stderr, "hearth-kwin-im: no zwp_input_method_v1 here (KWin gives it only to the input method it "
                        "starts itself: kwinrc [Wayland] InputMethod)\n");
        return 2;
    }
    int lfd = listen_socket();
    if (lfd < 0) { fprintf(stderr, "hearth-kwin-im: can't make %s: %s\n", sock_path, strerror(errno)); return 1; }
    fprintf(stderr, "hearth-kwin-im: ready (%s)\n", sock_path);

    while (!stop) {
        struct pollfd p[2 + MAX_CLIENTS];
        int map[2 + MAX_CLIENTS];
        int n = 0;
        p[n].fd = wl_display_get_fd(display); p[n].events = POLLIN; map[n++] = -1;
        p[n].fd = lfd; p[n].events = POLLIN; map[n++] = -2;
        for (int i = 0; i < MAX_CLIENTS; i++)
            if (clients[i]) { p[n].fd = clients[i]->fd; p[n].events = POLLIN; map[n++] = i; }
        while (wl_display_prepare_read(display) != 0) wl_display_dispatch_pending(display);
        wl_display_flush(display);
        if (poll(p, n, -1) < 0) {
            wl_display_cancel_read(display);
            if (errno == EINTR) continue;
            break;
        }
        if (p[0].revents & POLLIN) {
            if (wl_display_read_events(display) < 0) break;
        } else {
            wl_display_cancel_read(display);
        }
        if (p[0].revents & (POLLERR | POLLHUP)) break;
        if (wl_display_dispatch_pending(display) < 0) break;
        for (int k = 1; k < n; k++) {
            if (!(p[k].revents & (POLLIN | POLLHUP | POLLERR))) continue;
            if (map[k] == -2) {
                int cfd = accept4(lfd, NULL, NULL, SOCK_CLOEXEC);
                if (cfd < 0) continue;
                int slot = -1;
                for (int i = 0; i < MAX_CLIENTS; i++) if (!clients[i]) { slot = i; break; }
                if (slot < 0 || !same_user(cfd)) { close(cfd); continue; }
                clients[slot] = calloc(1, sizeof(struct client));
                if (!clients[slot]) { close(cfd); continue; }
                clients[slot]->fd = cfd;
                continue;
            }
            struct client *c = clients[map[k]];
            ssize_t r = recv(c->fd, c->buf + c->len, sizeof c->buf - 1 - c->len, 0);
            if (r <= 0) {
                if (r < 0 && errno == EINTR) continue;
                close(c->fd);
                free(c);
                clients[map[k]] = NULL;
                continue;
            }
            c->len += (size_t)r;
            c->buf[c->len] = 0;
            char *nl;
            while ((nl = memchr(c->buf, '\n', c->len))) {
                *nl = 0;
                command(c, c->buf);
                size_t used = (size_t)(nl - c->buf) + 1;
                memmove(c->buf, nl + 1, c->len - used);
                c->len -= used;
                c->buf[c->len] = 0;
            }
            if (c->len >= sizeof c->buf - 1) {   /* a line too long: drop the client */
                close(c->fd);
                free(c);
                clients[map[k]] = NULL;
            }
        }
    }
    unlink(sock_path);
    return 0;
}
