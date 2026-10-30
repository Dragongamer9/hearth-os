// SPDX-License-Identifier: GPL-2.0-or-later
#include "hearthgamepad.h"

#include <QAbstractSpinBox>
#include <QAction>
#include <QApplication>
#include <QDir>
#include <QDockWidget>
#include <QElapsedTimer>
#include <QFile>
#include <QHash>
#include <QKeyEvent>
#include <QLineEdit>
#include <QPlainTextEdit>
#include <QPointer>
#include <QSocketNotifier>
#include <QTextEdit>
#include <QTimer>
#include <QWidget>

#include <cerrno>
#include <cstring>
#include <fcntl.h>
#include <linux/input.h>
#include <sys/ioctl.h>
#include <unistd.h>

namespace
{

enum Button { A, B, X, Y, LB, RB, LT, RT, View, Menu, Up, Down, Left, Right, None };

constexpr int kRepeatFirstMs = 380;
constexpr int kRepeatMs = 110;
constexpr int kDuplicateMs = 60; // the same button from two devices (a handheld's pad and its virtual one)

bool testBit(const unsigned long *bits, int bit)
{
    return bits[bit / (8 * sizeof(long))] & (1UL << (bit % (8 * sizeof(long))));
}

Button fromKey(int code)
{
    switch (code) {
    case BTN_SOUTH:
        return A;
    case BTN_EAST:
        return B;
    case BTN_NORTH: // xpad's X
        return X;
    case BTN_WEST: // xpad's Y
        return Y;
    case BTN_TL:
        return LB;
    case BTN_TR:
        return RB;
    case BTN_TL2:
        return LT;
    case BTN_TR2:
        return RT;
    case BTN_SELECT:
        return View;
    case BTN_START:
        return Menu;
    case BTN_DPAD_UP:
        return Up;
    case BTN_DPAD_DOWN:
        return Down;
    case BTN_DPAD_LEFT:
        return Left;
    case BTN_DPAD_RIGHT:
        return Right;
    default:
        return None;
    }
}

struct Pad {
    int fd = -1;
    QString path;
    QSocketNotifier *notifier = nullptr;
    int hatX = 0;
    int hatY = 0;
    // Analog triggers: axis code → (button, threshold).
    QHash<int, QPair<Button, int>> triggers;
    QHash<int, bool> triggerDown;
};

class Gamepad : public QObject
{
public:
    explicit Gamepad(QObject *parent)
        : QObject(parent)
    {
        m_scan.setInterval(3000);
        connect(&m_scan, &QTimer::timeout, this, [this] {
            scan();
            m_padHelper = hearthPadRunning();
        });
        m_repeat.setSingleShot(true);
        connect(&m_repeat, &QTimer::timeout, this, [this] {
            if (m_held != None && m_active) {
                act(m_held);
                m_repeat.start(kRepeatMs);
            }
        });
        connect(qApp, &QGuiApplication::applicationStateChanged, this, [this](Qt::ApplicationState s) {
            m_active = s == Qt::ApplicationActive;
            if (m_active) {
                scan();
                m_padHelper = hearthPadRunning();
            } else {
                m_held = None;
                m_repeat.stop();
            }
        });
        m_active = QGuiApplication::applicationState() == Qt::ApplicationActive;
        m_padHelper = hearthPadRunning();
        scan();
        m_scan.start();
    }

    ~Gamepad() override
    {
        for (Pad *p : std::as_const(m_pads)) {
            ::close(p->fd);
            delete p;
        }
    }

private:
    QList<Pad *> m_pads;
    QTimer m_scan;
    QTimer m_repeat;
    Button m_held = None;
    bool m_active = false;
    bool m_padHelper = false;
    QHash<int, qint64> m_lastPress;
    QElapsedTimer m_clock;
    QPointer<QWidget> m_beforePlaces;

    // Hearth Classic's controller helper (not Game Mode's controller mouse, hearth-pad --game).
    static bool hearthPadRunning()
    {
        const QStringList pids = QDir(QStringLiteral("/proc")).entryList(QDir::Dirs | QDir::NoDotAndDotDot);
        for (const QString &pid : pids) {
            if (pid.isEmpty() || !pid.at(0).isDigit()) {
                continue;
            }
            QFile f(QStringLiteral("/proc/%1/cmdline").arg(pid));
            if (!f.open(QIODevice::ReadOnly)) {
                continue;
            }
            const QByteArray cmd = f.read(4096);
            if (cmd.contains("hearth-pad") && !cmd.contains("--game")) {
                return true;
            }
        }
        return false;
    }

    void scan()
    {
        const QStringList names = QDir(QStringLiteral("/dev/input")).entryList({QStringLiteral("event*")}, QDir::System);
        for (const QString &n : names) {
            const QString path = QStringLiteral("/dev/input/") + n;
            bool open = false;
            for (const Pad *p : std::as_const(m_pads)) {
                open = open || p->path == path;
            }
            if (!open) {
                tryOpen(path);
            }
        }
    }

    void tryOpen(const QString &path)
    {
        const int fd = ::open(QFile::encodeName(path).constData(), O_RDONLY | O_NONBLOCK | O_CLOEXEC);
        if (fd < 0) {
            return;
        }
        unsigned long evBits[(EV_MAX + 1) / (8 * sizeof(long)) + 1] = {};
        unsigned long keyBits[(KEY_MAX + 1) / (8 * sizeof(long)) + 1] = {};
        unsigned long absBits[(ABS_MAX + 1) / (8 * sizeof(long)) + 1] = {};
        char name[256] = {};
        ioctl(fd, EVIOCGNAME(sizeof(name) - 1), name);
        // A controller (has A and sticks), and not one of Hearth's own virtual devices.
        if (ioctl(fd, EVIOCGBIT(0, sizeof(evBits)), evBits) < 0 || !testBit(evBits, EV_KEY) || !testBit(evBits, EV_ABS)
            || ioctl(fd, EVIOCGBIT(EV_KEY, sizeof(keyBits)), keyBits) < 0 || !testBit(keyBits, BTN_SOUTH)
            || std::strncmp(name, "Hearth ", 7) == 0) {
            ::close(fd);
            return;
        }
        ioctl(fd, EVIOCGBIT(EV_ABS, sizeof(absBits)), absBits);
        auto *p = new Pad;
        p->fd = fd;
        p->path = path;
        auto addTrigger = [&](int code, Button b) {
            if (!testBit(absBits, code)) {
                return;
            }
            input_absinfo info = {};
            if (ioctl(fd, EVIOCGABS(code), &info) == 0 && info.maximum > info.minimum) {
                p->triggers.insert(code, qMakePair(b, info.minimum + (info.maximum - info.minimum) / 2));
            }
        };
        // Z/RZ are the triggers when the right stick is RX/RY (xpad, most pads); otherwise they're the right stick.
        if (testBit(absBits, ABS_RX) || testBit(absBits, ABS_RY)) {
            addTrigger(ABS_Z, LT);
            addTrigger(ABS_RZ, RT);
        }
        addTrigger(ABS_BRAKE, LT);
        addTrigger(ABS_GAS, RT);
        p->notifier = new QSocketNotifier(fd, QSocketNotifier::Read, this);
        connect(p->notifier, &QSocketNotifier::activated, this, [this, p] {
            readPad(p);
        });
        m_pads << p;
    }

    void dropPad(Pad *p)
    {
        m_pads.removeAll(p);
        p->notifier->setEnabled(false);
        p->notifier->deleteLater();
        ::close(p->fd);
        delete p;
    }

    void readPad(Pad *p)
    {
        input_event ev;
        for (;;) {
            const ssize_t n = ::read(p->fd, &ev, sizeof(ev));
            if (n != ssize_t(sizeof(ev))) {
                if (n < 0 && errno != EAGAIN && errno != EINTR) {
                    dropPad(p); // unplugged
                }
                return;
            }
            if (ev.type == EV_KEY && ev.value != 2) {
                button(fromKey(ev.code), ev.value == 1);
            } else if (ev.type == EV_ABS && ev.code == ABS_HAT0X) {
                hat(p->hatX, ev.value, Left, Right);
            } else if (ev.type == EV_ABS && ev.code == ABS_HAT0Y) {
                hat(p->hatY, ev.value, Up, Down);
            } else if (ev.type == EV_ABS && p->triggers.contains(ev.code)) {
                const auto t = p->triggers.value(ev.code);
                const bool down = ev.value > t.second;
                if (down != p->triggerDown.value(ev.code)) {
                    p->triggerDown.insert(ev.code, down);
                    button(t.first, down);
                }
            }
        }
    }

    void hat(int &state, int value, Button minus, Button plus)
    {
        const int v = value < 0 ? -1 : value > 0 ? 1 : 0;
        if (v == state) {
            return;
        }
        if (state != 0) {
            button(state < 0 ? minus : plus, false);
        }
        state = v;
        if (v != 0) {
            button(v < 0 ? minus : plus, true);
        }
    }

    void button(Button b, bool down)
    {
        if (b == None) {
            return;
        }
        const bool arrow = b == Up || b == Down || b == Left || b == Right;
        if (!down) {
            if (b == m_held) {
                m_held = None;
                m_repeat.stop();
            }
            return;
        }
        if (!m_active) {
            return;
        }
        if (!m_clock.isValid()) {
            m_clock.start();
        }
        const qint64 now = m_clock.elapsed();
        if (m_lastPress.contains(b) && now - m_lastPress.value(b) < kDuplicateMs) {
            return;
        }
        m_lastPress.insert(b, now);
        // With Hearth Classic's helper running, it already does these (see hearthgamepad.h).
        if (m_padHelper && b != LB && b != RB && b != LT && b != RT && b != View) {
            return;
        }
        act(b);
        if (arrow) {
            m_held = b;
            m_repeat.start(kRepeatFirstMs);
        }
    }

    static QWidget *window()
    {
        QWidget *w = QApplication::activeWindow();
        return w ? w->window() : nullptr;
    }

    // One of Dolphin's actions, by its name in Dolphin's action collection.
    static bool trigger(const char *name)
    {
        QWidget *w = window();
        if (!w) {
            return false;
        }
        QAction *a = w->findChild<QAction *>(QString::fromLatin1(name));
        if (!a || !a->isEnabled()) {
            return false;
        }
        a->trigger();
        return true;
    }

    static void key(int k)
    {
        QWidget *t = QApplication::activePopupWidget();
        if (!t) {
            t = QApplication::focusWidget();
        }
        if (!t) {
            t = window();
        }
        if (!t) {
            return;
        }
        QKeyEvent press(QEvent::KeyPress, k, Qt::NoModifier);
        QCoreApplication::sendEvent(t, &press);
        QKeyEvent release(QEvent::KeyRelease, k, Qt::NoModifier);
        QCoreApplication::sendEvent(t, &release);
    }

    // A menu, a dialog or a text field (renaming, the location bar, the filter) has the controller's attention.
    static bool somethingOpen()
    {
        if (QApplication::activePopupWidget() || QApplication::activeModalWidget()) {
            return true;
        }
        QWidget *f = QApplication::focusWidget();
        return f && (qobject_cast<QLineEdit *>(f) || qobject_cast<QTextEdit *>(f) || qobject_cast<QPlainTextEdit *>(f) || qobject_cast<QAbstractSpinBox *>(f));
    }

    void places()
    {
        QWidget *w = window();
        if (!w) {
            return;
        }
        auto *dock = w->findChild<QDockWidget *>(QStringLiteral("placesDock"));
        if (!dock || !dock->widget()) {
            return;
        }
        QWidget *f = QApplication::focusWidget();
        if (f && dock->isAncestorOf(f)) {
            // Back to the files.
            if (m_beforePlaces) {
                m_beforePlaces->setFocus(Qt::OtherFocusReason);
            } else {
                dock->widget()->focusNextChild();
            }
            return;
        }
        m_beforePlaces = f;
        if (!dock->isVisible()) {
            dock->show();
        }
        QWidget *target = dock->widget()->focusProxy() ? dock->widget()->focusProxy() : dock->widget();
        target->setFocus(Qt::OtherFocusReason);
    }

    void act(Button b)
    {
        switch (b) {
        case Up:
            key(Qt::Key_Up);
            break;
        case Down:
            key(Qt::Key_Down);
            break;
        case Left:
            key(Qt::Key_Left);
            break;
        case Right:
            key(Qt::Key_Right);
            break;
        case A:
            key(Qt::Key_Return);
            break;
        case B:
            if (somethingOpen()) {
                key(Qt::Key_Escape);
            } else if (!trigger("go_back")) {
                trigger("go_up");
            }
            break;
        case X:
            key(Qt::Key_Menu);
            break;
        case Y:
            trigger("show_filter_bar");
            break;
        case Menu:
            trigger("hamburger_menu");
            break;
        case LB:
            if (!somethingOpen()) {
                trigger("go_back");
            }
            break;
        case RB:
            if (!somethingOpen()) {
                trigger("go_forward");
            }
            break;
        case LT:
            trigger("activate_prev_tab");
            break;
        case RT:
            trigger("activate_next_tab");
            break;
        case View:
            places();
            break;
        case None:
            break;
        }
    }
};

} // namespace

void HearthGamepad::install()
{
    if (qEnvironmentVariableIsSet("HEARTH_FILES_NO_CONTROLLER")) {
        return;
    }
    new Gamepad(qApp);
}
