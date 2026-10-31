// Hearth Storage addresses.
//
// hearth:/                         the paired Hearth Storage servers
// hearth:/SERVER                   that server's shared folders
// hearth:/SERVER/SHARE/PATH        a folder or file in a shared folder
//
// SERVER is the server's Hearth Link id (16 to 64 hex characters), SHARE a shared folder's id (1 to 32 hex
// characters): the same parts as Hearth Files' hearth://SERVER/SHARE/PATH (desktop/shell/hearth-files), which is
// accepted too. The path form is what Hearth's places use, because a 64-character id is longer than a host name
// may be. Like hearth-files' parse_remote: plain parts only, nothing hidden (no ".", "..", or names starting
// with a dot), since the server never lists or gives those out either.
#pragma once

#include <QString>
#include <QUrl>

namespace Hearth
{

struct Location {
    bool valid = false;
    QString server;
    QString share;
    QString path; // inside the share, no leading or trailing slash

    // 0: the list of servers, 1: a server's shared folders, 2: in a shared folder
    int depth() const
    {
        return server.isEmpty() ? 0 : share.isEmpty() ? 1 : 2;
    }
    // The last part of the path ("" at the top of a share).
    QString name() const;
    // The folder this is in, inside the share ("" at the top).
    QString parentPath() const;
};

Location parseUrl(const QUrl &url);
QUrl makeUrl(const QString &server, const QString &share = QString(), const QString &path = QString());

bool isServerId(const QString &s);
bool isShareId(const QString &s);

} // namespace Hearth
