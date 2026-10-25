#include "pad.h"
#include "host.h"

#include <QStringList>
#include <cmath>

#ifdef HAVE_SDL2
#include <SDL.h>
#endif

Pad::Pad(Host *host, QObject *parent) : QObject(parent), m_host(host) {
#ifdef HAVE_SDL2
    // Read controllers whatever window has the keyboard (gamescope gives Hearth's window the input; the page decides
    // whether to act on it: App.padsAllowed). No video: Qt owns the window.
    SDL_SetHint(SDL_HINT_JOYSTICK_ALLOW_BACKGROUND_EVENTS, "1");
    SDL_SetHint(SDL_HINT_NO_SIGNAL_HANDLERS, "1");
    if (SDL_Init(SDL_INIT_GAMECONTROLLER | SDL_INIT_HAPTIC) != 0 && SDL_Init(SDL_INIT_GAMECONTROLLER) != 0) {
        m_host->log(QString("controllers: SDL couldn't start (%1); the keyboard and Hearth's own pad events still work").arg(SDL_GetError()));
        return;
    }
    m_ok = true;
    SDL_GameControllerEventState(SDL_ENABLE);
    connect(&m_timer, &QTimer::timeout, this, &Pad::poll);
    m_timer.start(8);
    m_host->log(QString("controllers: reading them with SDL %1.%2.%3").arg(SDL_MAJOR_VERSION).arg(SDL_MINOR_VERSION).arg(SDL_PATCHLEVEL));
#else
    m_host->log("controllers: built without SDL2; using the keyboard and Hearth's own pad events");
#endif
}

Pad::~Pad() {
#ifdef HAVE_SDL2
    for (void *p : m_pads) SDL_GameControllerClose(static_cast<SDL_GameController *>(p));
    if (m_ok) SDL_Quit();
#endif
}

void Pad::rumble(double strength, int ms) {
#ifdef HAVE_SDL2
    const Uint16 lo = Uint16(qBound(0.0, strength, 1.0) * 0xffff);
    for (void *p : m_pads) SDL_GameControllerRumble(static_cast<SDL_GameController *>(p), Uint16(lo * 0.4), lo, Uint32(qMax(1, ms)));
#else
    Q_UNUSED(strength); Q_UNUSED(ms);
#endif
}

void Pad::refreshNames() {
#ifdef HAVE_SDL2
    QStringList n;
    for (void *p : m_pads) n << QString::fromUtf8(SDL_GameControllerName(static_cast<SDL_GameController *>(p)));
    m_names = n.join(", ");
    m_count = m_pads.size();
    emit countChanged();
#endif
}

void Pad::poll() {
#ifdef HAVE_SDL2
    static const char *names[SDL_CONTROLLER_BUTTON_MAX] = {
        "a", "b", "x", "y", "view", "guide", "menu", "l3", "r3", "lb", "rb", "up", "down", "left", "right",
#if SDL_CONTROLLER_BUTTON_MAX > 15
        "misc", "paddle1", "paddle2", "paddle3", "paddle4", "touchpad",
#endif
    };
    SDL_Event e;
    bool moved = false;
    while (SDL_PollEvent(&e)) {
        switch (e.type) {
        case SDL_CONTROLLERDEVICEADDED: {
            SDL_GameController *c = SDL_GameControllerOpen(e.cdevice.which);
            if (c) {
                const int id = SDL_JoystickInstanceID(SDL_GameControllerGetJoystick(c));
                if (!m_pads.contains(id)) { m_pads.insert(id, c); m_host->log(QString("controllers: %1 connected").arg(QString::fromUtf8(SDL_GameControllerName(c)))); refreshNames(); }
                else SDL_GameControllerClose(c);
            }
            break;
        }
        case SDL_CONTROLLERDEVICEREMOVED: {
            auto it = m_pads.find(e.cdevice.which);
            if (it != m_pads.end()) {
                m_host->log(QString("controllers: %1 disconnected").arg(QString::fromUtf8(SDL_GameControllerName(static_cast<SDL_GameController *>(*it)))));
                SDL_GameControllerClose(static_cast<SDL_GameController *>(*it));
                m_pads.erase(it);
                refreshNames();
            }
            break;
        }
        case SDL_CONTROLLERBUTTONDOWN:
        case SDL_CONTROLLERBUTTONUP:
            if (e.cbutton.button < SDL_CONTROLLER_BUTTON_MAX && names[e.cbutton.button])
                emit button(QString::fromLatin1(names[e.cbutton.button]), e.type == SDL_CONTROLLERBUTTONDOWN);
            break;
        case SDL_CONTROLLERAXISMOTION: {
            const int a = e.caxis.axis;
            if (a < 0 || a >= 6) break;
            const double v = e.caxis.value / 32767.0;
            m_ax[a] = v;
            if (a == SDL_CONTROLLER_AXIS_TRIGGERLEFT || a == SDL_CONTROLLER_AXIS_TRIGGERRIGHT) {
                const int t = a == SDL_CONTROLLER_AXIS_TRIGGERLEFT ? 0 : 1;
                // Triggers are buttons here, with a little room so a resting finger doesn't flicker them.
                if (!m_trig[t] && v > 0.55) { m_trig[t] = true; emit button(t ? "rt" : "lt", true); }
                else if (m_trig[t] && v < 0.35) { m_trig[t] = false; emit button(t ? "rt" : "lt", false); }
            } else moved = true;
            break;
        }
        default: break;
        }
    }
    if (moved) {
        const double lx = m_ax[SDL_CONTROLLER_AXIS_LEFTX], ly = m_ax[SDL_CONTROLLER_AXIS_LEFTY];
        emit axes(lx, ly, m_ax[SDL_CONTROLLER_AXIS_RIGHTX], m_ax[SDL_CONTROLLER_AXIS_RIGHTY]);
        // The left stick as a D-pad: the stronger direction past 0.6, let go under 0.35.
        QString dir = m_stickDir;
        const double mag = std::max(std::fabs(lx), std::fabs(ly));
        if (mag < 0.35) dir.clear();
        else if (mag > 0.6) dir = std::fabs(lx) > std::fabs(ly) ? (lx > 0 ? "right" : "left") : (ly > 0 ? "down" : "up");
        if (dir != m_stickDir) {
            if (!m_stickDir.isEmpty()) emit stick(m_stickDir, false);
            m_stickDir = dir;
            if (!dir.isEmpty()) emit stick(dir, true);
        }
    }
#endif
}
