#include "hearthurl.h"

#include <QRegularExpression>
#include <QStringList>

namespace Hearth
{

bool isServerId(const QString &s)
{
    static const QRegularExpression re(QStringLiteral("^[0-9a-f]{16,64}$"));
    return re.match(s).hasMatch();
}

bool isShareId(const QString &s)
{
    static const QRegularExpression re(QStringLiteral("^[0-9a-f]{1,32}$"));
    return re.match(s).hasMatch();
}

QString Location::name() const
{
    const int i = path.lastIndexOf(QLatin1Char('/'));
    return i < 0 ? path : path.mid(i + 1);
}

QString Location::parentPath() const
{
    const int i = path.lastIndexOf(QLatin1Char('/'));
    return i < 0 ? QString() : path.left(i);
}

Location parseUrl(const QUrl &url)
{
    Location loc;
    if (!url.isValid() || url.scheme() != QLatin1String("hearth")) {
        return loc;
    }
    QStringList parts;
    if (!url.host().isEmpty()) {
        parts << url.host(); // hearth://SERVER/SHARE/PATH, as Hearth Files writes it
    }
    const QStringList segs = url.path().split(QLatin1Char('/'), Qt::SkipEmptyParts);
    parts << segs;
    if (!parts.isEmpty()) {
        loc.server = parts.takeFirst().toLower();
        if (!isServerId(loc.server)) {
            return Location();
        }
    }
    if (!parts.isEmpty()) {
        loc.share = parts.takeFirst().toLower();
        if (!isShareId(loc.share)) {
            return Location();
        }
    }
    for (const QString &p : std::as_const(parts)) {
        if (p == QLatin1String(".") || p == QLatin1String("..") || p.startsWith(QLatin1Char('.')) || p.contains(QChar(0))) {
            return Location();
        }
    }
    loc.path = parts.join(QLatin1Char('/'));
    loc.valid = true;
    return loc;
}

QUrl makeUrl(const QString &server, const QString &share, const QString &path)
{
    QUrl u;
    u.setScheme(QStringLiteral("hearth"));
    QString p = QStringLiteral("/");
    if (!server.isEmpty()) {
        p += server;
        if (!share.isEmpty()) {
            p += QLatin1Char('/') + share;
            if (!path.isEmpty()) {
                p += QLatin1Char('/') + path;
            }
        }
    }
    u.setPath(p);
    return u;
}

} // namespace Hearth
