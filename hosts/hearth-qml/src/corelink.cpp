#include "corelink.h"
#include "host.h"

#include <QCoreApplication>
#include <QProcessEnvironment>
#include <QRegularExpression>

CoreLink::CoreLink(Host *host, QObject *parent) : QObject(parent), m_host(host) {
    m_program = qEnvironmentVariable("HEARTH_CORE_BIN");
    if (m_program.isEmpty()) m_program = "/usr/lib/nimbus/nimbus";
    // The core's own log lines (stderr) go where this program's go: the session log.
    m_proc.setProcessChannelMode(QProcess::ForwardedErrorChannel);
    m_proc.setReadChannel(QProcess::StandardOutput);
    connect(&m_proc, &QProcess::readyReadStandardOutput, this, &CoreLink::readOut);
    connect(&m_proc, &QProcess::stateChanged, this, [this] { emit runningChanged(); });
    connect(&m_proc, &QProcess::errorOccurred, this, [this](QProcess::ProcessError e) {
        if (e == QProcess::FailedToStart) {
            m_host->log("core: couldn't start " + m_program + ": " + m_proc.errorString());
            if (!m_leaving) { m_leaving = true; emit exited(1); QCoreApplication::exit(1); }
        }
    });
    connect(&m_proc, &QProcess::finished, this, [this](int code, QProcess::ExitStatus st) {
        readOut();
        const int c = st == QProcess::CrashExit ? 1 : code;
        m_host->log(QString("core exited (%1%2)").arg(code).arg(st == QProcess::CrashExit ? ", crashed" : ""));
        // Hearth's core gone: leave with its code (the session's watchdog takes over), rather than an empty window.
        if (!m_leaving) { m_leaving = true; emit exited(c); QCoreApplication::exit(c); }
    });
    connect(host, &Host::leaving, this, [this](int) { stop(); });
    connect(qApp, &QCoreApplication::aboutToQuit, this, [this] { stop(); });
}

CoreLink::~CoreLink() { stop(); }

QStringList CoreLink::coreArgs(const QStringList &argv) {
    static const QRegularExpression foreign(
        "^--(ozone-platform|enable-features|disable-features|no-sandbox|disable-gpu|use-gl|use-angle|enable-wayland-ime|in-process-gpu|"
        "self-test|shots|platform|qwindowgeometry|qwindowtitle|style|reverse|plugin)(=|$)");
    QStringList out{"--core-stdio"};
    for (int i = 1; i < argv.size(); ++i) {
        const QString &a = argv[i];
        if (a == "--shots") { ++i; continue; }
        if (a.startsWith("--") && !foreign.match(a).hasMatch() && a != "--core-stdio") out << a;
    }
    // A value after --page (Hearth Settings' page) goes along.
    const int i = argv.indexOf("--page");
    if (i >= 0 && i + 1 < argv.size() && !argv[i + 1].startsWith("--")) out << argv[i + 1];
    return out;
}

void CoreLink::start() {
    if (running()) return;
    const QStringList a = coreArgs(m_host->args());
    QProcessEnvironment env = QProcessEnvironment::systemEnvironment();
    // host.info and Settings › Display › Renderer say Qt draws; the core counts the start as working once the page is up.
    env.insert("HEARTH_RENDERER_RUNNING", "qt");
    env.remove("HEARTH_QT_PARENT");
    m_proc.setProcessEnvironment(env);
    m_host->log("core: starting " + m_program + " " + a.join(' '));
    m_proc.start(m_program, a);
}

void CoreLink::send(const QString &json) {
    if (m_proc.state() != QProcess::Running) return;
    QByteArray b = json.toUtf8();
    b.replace('\n', ' ');
    b.append('\n');
    m_proc.write(b);
}

void CoreLink::readOut() {
    m_buf += m_proc.readAllStandardOutput();
    int i;
    while ((i = m_buf.indexOf('\n')) >= 0) {
        const QByteArray l = m_buf.left(i).trimmed();
        m_buf.remove(0, i + 1);
        // Lines that aren't JSON objects are ignored (a stray print can't break the channel).
        if (l.startsWith('{')) emit line(QString::fromUtf8(l));
    }
}

void CoreLink::stop() {
    if (m_proc.state() == QProcess::NotRunning) return;
    m_leaving = true;
    // The core leaves when its input closes (core_ipc.rs), putting the controller service back to normal.
    m_proc.closeWriteChannel();
    if (!m_proc.waitForFinished(3000)) { m_proc.terminate(); if (!m_proc.waitForFinished(2000)) m_proc.kill(); }
}
