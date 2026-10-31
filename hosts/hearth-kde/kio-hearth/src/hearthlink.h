// One request to the running Hearth Link service, the way `hearth-link call` sends it (hosts/hearth-link ipc.rs):
// a JSON line {"id", "cmd", "args"} on its user socket, answered by {"id", "ok", "data"} or {"id", "ok": false,
// "error"}. The socket is $HEARTH_LINK_SOCKET, else $XDG_RUNTIME_DIR/hearth-link/link.sock (lib.rs socket_path).
// Blocking: a KIO worker is its own process and does one thing at a time.
#pragma once

#include <QJsonObject>
#include <QJsonValue>
#include <QString>

namespace Hearth
{

class LinkClient
{
public:
    explicit LinkClient(const QString &socketPath = QString());

    static QString defaultSocketPath();
    QString socketPath() const
    {
        return m_path;
    }

    // true with the answer's "data" in *data; false with Hearth Link's own words (or why it couldn't be asked) in
    // *error.
    bool call(const QString &cmd, const QJsonObject &args, QJsonValue *data, QString *error, int timeoutMs = 150000) const;

private:
    QString m_path;
};

// The answer line's parts (for the tests too): true with "data", false with "error".
bool parseAnswer(const QByteArray &line, QJsonValue *data, QString *error);

} // namespace Hearth
