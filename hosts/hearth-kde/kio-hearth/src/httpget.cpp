#include "httpget.h"

#include <QElapsedTimer>
#include <QHostAddress>
#include <QList>
#include <QUrl>

#include <cstring>

namespace Hearth
{

bool parseLocalUrl(const QString &url, HttpTarget *target)
{
    const QUrl u(url, QUrl::StrictMode);
    if (!u.isValid() || u.scheme() != QLatin1String("http") || u.host() != QLatin1String("127.0.0.1") || u.port() <= 0
        || u.port() > 65535 || !u.userInfo().isEmpty()) {
        return false;
    }
    target->port = quint16(u.port());
    target->path = u.path(QUrl::FullyEncoded).toLatin1();
    if (target->path.isEmpty() || !target->path.startsWith('/')) {
        return false;
    }
    return true;
}

bool parseHead(const QByteArray &head, HttpHead *h)
{
    const QList<QByteArray> lines = head.split('\n');
    if (lines.isEmpty()) {
        return false;
    }
    const QList<QByteArray> first = lines.first().trimmed().split(' ');
    if (first.size() < 2 || !first.at(0).startsWith("HTTP/1.")) {
        return false;
    }
    bool ok = false;
    h->status = first.at(1).toInt(&ok);
    if (!ok) {
        return false;
    }
    for (int i = 1; i < lines.size(); ++i) {
        const QByteArray l = lines.at(i).trimmed();
        const int c = l.indexOf(':');
        if (c <= 0) {
            continue;
        }
        const QByteArray k = l.left(c).trimmed().toLower();
        const QByteArray v = l.mid(c + 1).trimmed();
        if (k == "content-length") {
            bool lok = false;
            const qint64 n = v.toLongLong(&lok);
            if (lok && n >= 0) {
                h->contentLength = n;
            }
        } else if (k == "content-type") {
            h->contentType = v;
        }
    }
    return true;
}

bool HttpGet::open(const HttpTarget &target, HttpHead *head, QString *error, int timeoutMs)
{
    m_sock.connectToHost(QHostAddress(QHostAddress::LocalHost), target.port);
    if (!m_sock.waitForConnected(10000)) {
        *error = QStringLiteral("Hearth Link's stream didn't answer");
        return false;
    }
    m_sock.write("GET " + target.path + " HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    m_sock.flush();
    QByteArray buf;
    QElapsedTimer t;
    t.start();
    int end = -1;
    while ((end = buf.indexOf("\r\n\r\n")) < 0) {
        if (buf.size() > 16384 || t.elapsed() > timeoutMs) {
            *error = QStringLiteral("Hearth Link's stream didn't answer");
            return false;
        }
        if (m_sock.bytesAvailable() > 0 || m_sock.waitForReadyRead(1000)) {
            buf += m_sock.readAll();
        } else if (m_sock.state() != QAbstractSocket::ConnectedState) {
            buf += m_sock.readAll();
            if (buf.indexOf("\r\n\r\n") < 0) {
                *error = QStringLiteral("Hearth Link's stream closed the connection");
                return false;
            }
        }
    }
    if (!parseHead(buf.left(end), head)) {
        *error = QStringLiteral("Hearth Link's stream gave an answer that can't be read");
        return false;
    }
    m_pending = buf.mid(end + 4);
    if (head->status == 404) {
        *error = QStringLiteral("That link to the file ran out: open it again");
        return false;
    }
    if (head->status != 200) {
        *error = QStringLiteral("Hearth Storage couldn't send that file (%1)").arg(head->status);
        return false;
    }
    return true;
}

qint64 HttpGet::read(char *buf, qint64 max, int timeoutMs)
{
    if (!m_pending.isEmpty()) {
        const qint64 n = qMin<qint64>(max, m_pending.size());
        std::memcpy(buf, m_pending.constData(), size_t(n));
        m_pending.remove(0, int(n));
        return n;
    }
    QElapsedTimer t;
    t.start();
    while (m_sock.bytesAvailable() <= 0) {
        if (m_sock.state() != QAbstractSocket::ConnectedState) {
            return m_sock.bytesAvailable() > 0 ? m_sock.read(buf, max) : 0;
        }
        if (t.elapsed() > timeoutMs) {
            return -1;
        }
        m_sock.waitForReadyRead(1000);
    }
    return m_sock.read(buf, max);
}

} // namespace Hearth
