#include "hearthlink.h"

#include <QDir>
#include <QElapsedTimer>
#include <QFileInfo>
#include <QJsonDocument>
#include <QLocalSocket>

#include <unistd.h>

namespace Hearth
{

LinkClient::LinkClient(const QString &socketPath)
    : m_path(socketPath.isEmpty() ? defaultSocketPath() : socketPath)
{
}

QString LinkClient::defaultSocketPath()
{
    const QString env = qEnvironmentVariable("HEARTH_LINK_SOCKET");
    if (!env.isEmpty()) {
        return env;
    }
    QString run = qEnvironmentVariable("XDG_RUNTIME_DIR");
    if (run.isEmpty()) {
        run = QDir::tempPath() + QStringLiteral("/hearth-link-%1").arg(getuid());
    }
    return run + QStringLiteral("/hearth-link/link.sock");
}

bool parseAnswer(const QByteArray &line, QJsonValue *data, QString *error)
{
    QJsonParseError pe;
    const QJsonDocument doc = QJsonDocument::fromJson(line.trimmed(), &pe);
    if (pe.error != QJsonParseError::NoError || !doc.isObject()) {
        *error = QStringLiteral("Hearth Link gave an answer that can't be read");
        return false;
    }
    const QJsonObject o = doc.object();
    if (o.value(QStringLiteral("ok")).toBool()) {
        *data = o.value(QStringLiteral("data"));
        return true;
    }
    const QString e = o.value(QStringLiteral("error")).toString();
    *error = e.isEmpty() ? QStringLiteral("Hearth Link said no") : e;
    return false;
}

bool LinkClient::call(const QString &cmd, const QJsonObject &args, QJsonValue *data, QString *error, int timeoutMs) const
{
    if (!QFileInfo::exists(m_path)) {
        *error = QStringLiteral("Hearth Link isn't running");
        return false;
    }
    QLocalSocket s;
    s.connectToServer(m_path);
    if (!s.waitForConnected(5000)) {
        *error = QStringLiteral("Hearth Link isn't running");
        return false;
    }
    QJsonObject req;
    req.insert(QStringLiteral("id"), 1);
    req.insert(QStringLiteral("cmd"), cmd);
    req.insert(QStringLiteral("args"), args);
    s.write(QJsonDocument(req).toJson(QJsonDocument::Compact) + '\n');
    s.flush();
    QByteArray buf;
    QElapsedTimer t;
    t.start();
    while (!buf.contains('\n')) {
        const qint64 left = timeoutMs - t.elapsed();
        if (left <= 0) {
            *error = QStringLiteral("Hearth Storage took too long to answer");
            return false;
        }
        if (s.bytesAvailable() > 0 || s.waitForReadyRead(int(qMin<qint64>(left, 1000)))) {
            buf += s.readAll();
            continue;
        }
        if (s.state() != QLocalSocket::ConnectedState) {
            buf += s.readAll();
            if (!buf.contains('\n')) {
                *error = QStringLiteral("Hearth Link closed the connection");
                return false;
            }
        }
    }
    return parseAnswer(buf.left(buf.indexOf('\n')), data, error);
}

} // namespace Hearth
