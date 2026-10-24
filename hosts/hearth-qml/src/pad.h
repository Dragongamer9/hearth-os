// Controllers, read with SDL2's game controller API (any Xbox-, PlayStation- or Switch-style pad, and Handheld
// Daemon's virtual one), as the same button names the web page uses: a b x y lb rb lt rt l3 r3 view menu guide
// up down left right. The left stick also moves the selection (stick()), and both sticks are reported (axes()) for the
// on-screen keyboard's Ember Wheel. Built without SDL2, there's nothing here and the core's pad events and the
// keyboard still work.
#pragma once
#include <QHash>
#include <QObject>
#include <QTimer>

class Host;

class Pad : public QObject {
    Q_OBJECT
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_PROPERTY(bool available READ available CONSTANT)
    Q_PROPERTY(QString names READ names NOTIFY countChanged)
public:
    explicit Pad(Host *host, QObject *parent = nullptr);
    ~Pad() override;
    int count() const { return m_count; }
    bool available() const { return m_ok; }
    QString names() const { return m_names; }
    Q_INVOKABLE void rumble(double strength, int ms);

signals:
    void button(const QString &name, bool down);
    void stick(const QString &dir, bool down);
    void axes(double lx, double ly, double rx, double ry);
    void countChanged();

private:
    void poll();
    void refreshNames();
    Host *m_host;
    QTimer m_timer;
    bool m_ok = false;
    int m_count = 0;
    QString m_names;
    QHash<int, void *> m_pads;     // instance id → SDL_GameController*
    double m_ax[6] = {0, 0, 0, 0, 0, 0};
    bool m_trig[2] = {false, false};
    QString m_stickDir;
};
