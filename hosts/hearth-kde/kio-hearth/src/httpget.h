// A file's bytes from Hearth Link's local stream (storage.stream gives http://127.0.0.1:<port>/<token>/<name>, the
// same link Hearth Player plays from; Hearth Link fetches the bytes from the server over its encrypted connection as
// they're asked for). Only this device's loopback address is ever connected to.
#pragma once

#include <QByteArray>
#include <QString>
#include <QTcpSocket>

namespace Hearth
{

struct HttpTarget {
    quint16 port = 0;
    QByteArray path; // "/<token>/<name>"
};

struct HttpHead {
    int status = 0;
    qint64 contentLength = -1;
    QByteArray contentType;
};

// Only http://127.0.0.1:PORT/... (anything else: false).
bool parseLocalUrl(const QString &url, HttpTarget *target);
// The response's head (up to the empty line): status and the headers used here.
bool parseHead(const QByteArray &head, HttpHead *h);

class HttpGet
{
public:
    // Connects, sends GET, reads the head. false with *error when that fails or the status isn't 200.
    bool open(const HttpTarget &target, HttpHead *head, QString *error, int timeoutMs = 60000);
    // Up to max bytes of the body: >0 bytes read, 0 the end (the server closed), -1 an error or no data in time.
    qint64 read(char *buf, qint64 max, int timeoutMs = 120000);

private:
    QTcpSocket m_sock;
    QByteArray m_pending;
};

} // namespace Hearth
