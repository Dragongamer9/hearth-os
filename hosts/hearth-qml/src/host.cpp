#include "host.h"

#include <QCoreApplication>
#include <QDateTime>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QMutex>
#include <QProcess>
#include <QStandardPaths>
#include <QUrl>
#include <cstdio>

#ifndef HEARTH_QML_VERSION
#define HEARTH_QML_VERSION "dev"
#endif

namespace {
QMutex g_logLock;
QStringList g_warnings;
QString logPath() {
    QString data = qEnvironmentVariable("XDG_DATA_HOME");
    if (data.isEmpty()) data = QDir::homePath() + "/.local/share";
    return data + "/nimbus/hearth-qml.log";
}
void writeLine(const QString &line) {
    QMutexLocker lock(&g_logLock);
    const QByteArray b = line.toUtf8() + '\n';
    fwrite(b.constData(), 1, b.size(), stderr);
    fflush(stderr);
    static bool opened = false;
    static QFile f;
    if (!opened) {
        opened = true;
        const QString p = logPath();
        QDir().mkpath(QFileInfo(p).absolutePath());
        // One run's worth plus the one before: over 2 MB it starts again (the old one is kept as .old).
        if (QFileInfo(p).size() > 2 * 1024 * 1024) { QFile::remove(p + ".old"); QFile::rename(p, p + ".old"); }
        f.setFileName(p);
        if (!f.open(QIODevice::Append | QIODevice::Text)) return;
    }
    if (f.isOpen()) { f.write(b); f.flush(); }
}
void handler(QtMsgType type, const QMessageLogContext &ctx, const QString &msg) {
    const char *lv = type == QtDebugMsg ? "debug" : type == QtInfoMsg ? "info" : type == QtWarningMsg ? "warning" : type == QtCriticalMsg ? "error" : "fatal";
    QString where;
    if (ctx.file && *ctx.file) where = QString(" (%1:%2)").arg(QString::fromUtf8(ctx.file)).arg(ctx.line);
    const QString line = QString("[hearth-qml] %1 %2: %3%4").arg(QTime::currentTime().toString("HH:mm:ss"), lv, msg, where);
    writeLine(line);
    // QML problems (a missing property, a script error, a module that isn't installed): the self-test fails on them.
    const bool qml = (ctx.file && (QByteArray(ctx.file).startsWith("qrc:") || QByteArray(ctx.file).endsWith(".qml"))) ||
                     msg.contains(".qml:") || msg.contains("qrc:/");
    if ((type == QtWarningMsg || type == QtCriticalMsg || type == QtFatalMsg) && qml) {
        QMutexLocker lock(&g_logLock);
        if (g_warnings.size() < 200) g_warnings << msg + where;
    }
}
}  // namespace

void Host::installLog() { qInstallMessageHandler(handler); }

Host::Host(const QStringList &args, QObject *parent) : QObject(parent), m_args(args) {
    for (int i = 1; i < args.size(); ++i) {
        const QString &a = args[i];
        if (a == "--self-test") m_selfTest = true;
        else if (a == "--windowed") m_windowed = true;
        else if (a == "--shots" && i + 1 < args.size()) m_shotsDir = args[++i];
        else if (a.startsWith("--shots=")) m_shotsDir = a.mid(8);
    }
}

bool Host::inSession() const { return qEnvironmentVariable("NIMBUS_SESSION") == "1"; }
QString Host::version() const { return QStringLiteral(HEARTH_QML_VERSION); }
bool Host::hasSdl() const {
#ifdef HAVE_SDL2
    return true;
#else
    return false;
#endif
}

void Host::log(const QString &text) { writeLine(QString("[hearth-qml] %1 qml: %2").arg(QTime::currentTime().toString("HH:mm:ss"), text)); }
QString Host::env(const QString &name) const { return qEnvironmentVariable(name.toUtf8().constData()); }
void Host::quit(int code) {
    log(QString("leaving (exit code %1)").arg(code));
    emit leaving(code);
    QCoreApplication::exit(code);
}
void Host::restartHearth() {
    QString core = qEnvironmentVariable("HEARTH_CORE_BIN");
    if (core.isEmpty()) core = "/usr/lib/nimbus/nimbus";
    QStringList a;
    for (int i = 1; i < m_args.size(); ++i) if (m_args[i] != "--core-stdio" && m_args[i] != "--self-test") a << m_args[i];
    log("starting Hearth again: " + core + " " + a.join(' '));
    QProcess::startDetached(core, a);
    quit(0);
}
bool Host::makeDir(const QString &path) { return QDir().mkpath(path); }
QString Host::fileUrl(const QString &path) const { return QUrl::fromLocalFile(path).toString(); }
bool Host::fileExists(const QString &path) const { return QFileInfo::exists(path); }
QStringList Host::warnings() const { QMutexLocker lock(&g_logLock); return g_warnings; }
void Host::clearWarnings() { QMutexLocker lock(&g_logLock); g_warnings.clear(); }
qint64 Host::now() const { return QDateTime::currentMSecsSinceEpoch(); }
