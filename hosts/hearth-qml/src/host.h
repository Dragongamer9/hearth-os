// Things the QML side needs from the program: its arguments, the log, leaving, starting Hearth again, the self-test.
#pragma once
#include <QObject>
#include <QStringList>
#include <QVariantMap>

class Host : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool selfTest READ selfTest CONSTANT)
    Q_PROPERTY(QString shotsDir READ shotsDir CONSTANT)
    Q_PROPERTY(bool windowed READ windowed CONSTANT)
    Q_PROPERTY(bool inSession READ inSession CONSTANT)
    Q_PROPERTY(QString displayFont READ displayFont CONSTANT)
    Q_PROPERTY(QString version READ version CONSTANT)
    Q_PROPERTY(bool hasSdl READ hasSdl CONSTANT)
    Q_PROPERTY(QString qtVersion READ qtVersion CONSTANT)
public:
    explicit Host(const QStringList &args, QObject *parent = nullptr);

    bool selfTest() const { return m_selfTest; }
    QString shotsDir() const { return m_shotsDir; }
    bool windowed() const { return m_windowed; }
    bool inSession() const;
    QString displayFont() const { return m_displayFont; }
    void setDisplayFont(const QString &f) { m_displayFont = f; }
    QString version() const;
    bool hasSdl() const;
    QString qtVersion() const { return QString::fromLatin1(qVersion()); }
    QStringList args() const { return m_args; }

    /// One line in Hearth's Qt log (stderr, which the session keeps in session.log, and hearth-qml.log).
    Q_INVOKABLE void log(const QString &text);
    Q_INVOKABLE QString env(const QString &name) const;
    /// Leave with this exit code (0: the session goes on; 75: start Hearth again).
    Q_INVOKABLE void quit(int code);
    /// Start Hearth again (outside the session): `nimbus` decides the renderer again.
    Q_INVOKABLE void restartHearth();
    Q_INVOKABLE bool makeDir(const QString &path);
    Q_INVOKABLE QString fileUrl(const QString &path) const;
    Q_INVOKABLE bool fileExists(const QString &path) const;
    /// QML warnings and errors seen so far (the self-test fails on any).
    Q_INVOKABLE QStringList warnings() const;
    Q_INVOKABLE void clearWarnings();
    Q_INVOKABLE qint64 now() const;

    static void installLog();

signals:
    void leaving(int code);

private:
    QStringList m_args;
    bool m_selfTest = false;
    bool m_windowed = false;
    QString m_shotsDir;
    QString m_displayFont;
};
