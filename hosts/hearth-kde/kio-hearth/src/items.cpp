#include "items.h"
#include "hearthurl.h"

#include <QJsonArray>

namespace Hearth
{

static bool plainName(const QString &n)
{
    return !n.isEmpty() && !n.contains(QLatin1Char('/')) && !n.startsWith(QLatin1Char('.')) && !n.contains(QChar(0));
}

QList<Server> serversFromStatus(const QJsonObject &status)
{
    QList<Server> out;
    const QJsonArray servers = status.value(QStringLiteral("servers")).toArray();
    for (const QJsonValue &v : servers) {
        const QJsonObject o = v.toObject();
        Server s;
        s.id = o.value(QStringLiteral("id")).toString().toLower();
        if (!isServerId(s.id)) {
            continue;
        }
        s.known = true;
        s.name = o.value(QStringLiteral("name")).toString();
        if (s.name.isEmpty()) {
            s.name = QStringLiteral("Hearth Storage");
        }
        // Only a server that said "media": false has it off (an older answer without it: try).
        s.media = o.value(QStringLiteral("media")).toBool(true);
        const QJsonArray shares = o.value(QStringLiteral("shares")).toArray();
        for (const QJsonValue &sv : shares) {
            const QJsonObject so = sv.toObject();
            Item it;
            it.name = so.value(QStringLiteral("id")).toString().toLower();
            if (!isShareId(it.name)) {
                continue;
            }
            it.display = so.value(QStringLiteral("name")).toString();
            if (it.display.isEmpty()) {
                it.display = QStringLiteral("Shared folder");
            }
            it.dir = true;
            it.icon = QStringLiteral("folder-network");
            s.shares << it;
        }
        out << s;
    }
    return out;
}

Server findServer(const QJsonObject &status, const QString &id)
{
    const QList<Server> all = serversFromStatus(status);
    for (const Server &s : all) {
        if (s.id == id) {
            return s;
        }
    }
    return Server();
}

QList<Item> itemsFromLs(const QJsonObject &ls)
{
    QList<Item> out;
    const QJsonArray entries = ls.value(QStringLiteral("entries")).toArray();
    for (const QJsonValue &v : entries) {
        const QJsonObject o = v.toObject();
        Item it;
        it.name = o.value(QStringLiteral("name")).toString();
        if (!plainName(it.name)) {
            continue; // hidden files are never given out by the server; anything else odd is left out here too
        }
        it.display = it.name;
        it.dir = o.value(QStringLiteral("dir")).toBool();
        it.size = it.dir ? 0 : qMax<qint64>(0, qint64(o.value(QStringLiteral("size")).toDouble()));
        it.mtime = qMax<qint64>(0, qint64(o.value(QStringLiteral("mtime")).toDouble()));
        out << it;
    }
    return out;
}

QString readOnlyMessage()
{
    return QStringLiteral("Hearth Storage's shared folders can't be changed from here: copy what you need to this device");
}

} // namespace Hearth
