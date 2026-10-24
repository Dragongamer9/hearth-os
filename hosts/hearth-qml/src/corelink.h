// The channel to the Hearth core (`nimbus --core-stdio`): one JSON object a line, each with "t" (described in
// hosts/hearth-rs/src/winhost.rs). This side only starts the core and moves lines; what they mean is the QML side's
// (App.qml), so all of Hearth's data and logic stays in the core.
#pragma once
#include <QObject>
#include <QProcess>
#include <QStringList>

class Host;

class CoreLink : public QObject {
    Q_OBJECT
    Q_PROPERTY(bool running READ running NOTIFY runningChanged)
    Q_PROPERTY(QString program READ program CONSTANT)
public:
    explicit CoreLink(Host *host, QObject *parent = nullptr);
    ~CoreLink() override;

    bool running() const { return m_proc.state() != QProcess::NotRunning; }
    QString program() const { return m_program; }

    /// Hearth's own arguments for the core (--windowed, --session, --page X...), not Qt's or Chromium's.
    static QStringList coreArgs(const QStringList &argv);

    Q_INVOKABLE void start();
    /// One JSON object (as text) to the core.
    Q_INVOKABLE void send(const QString &json);
    void stop();

signals:
    void line(const QString &json);
    void runningChanged();
    void exited(int code);

private:
    void readOut();
    Host *m_host;
    QProcess m_proc;
    QByteArray m_buf;
    QString m_program;
    bool m_leaving = false;
};
