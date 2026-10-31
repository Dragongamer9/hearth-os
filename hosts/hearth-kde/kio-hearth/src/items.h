// Hearth Link's answers turned into folder entries (no KIO here, so the tests check them directly).
//   storage.status → servers [{id, name, shares [{id, name}], media, ...}] (storage.rs storage_status)
//   storage.ls     → {share, path, entries [{name, dir, size, mtime}]}       (storage.rs list_dir)
#pragma once

#include <QJsonObject>
#include <QList>
#include <QString>

namespace Hearth
{

struct Item {
    QString name;    // the part of the address
    QString display; // what's shown (servers and shared folders have names; files show their own)
    bool dir = false;
    qint64 size = 0;
    qint64 mtime = 0; // seconds
    QString icon;
};

struct Server {
    QString id;
    QString name;
    bool media = true; // "Media" (playing and opening from it) on: storage.read needs it
    bool known = false;
    QList<Item> shares;
};

QList<Server> serversFromStatus(const QJsonObject &status);
// The server with that id (known = false when it isn't one).
Server findServer(const QJsonObject &status, const QString &id);
QList<Item> itemsFromLs(const QJsonObject &ls);

// Read-only message for anything that would change a shared folder (the same words as Hearth Files).
QString readOnlyMessage();

} // namespace Hearth
