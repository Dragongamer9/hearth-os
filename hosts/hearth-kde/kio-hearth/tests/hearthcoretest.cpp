#include "hearthlink.h"
#include "hearthurl.h"
#include "httpget.h"
#include "items.h"

#include <QJsonArray>
#include <QJsonDocument>
#include <QTemporaryDir>
#include <QTest>

using namespace Hearth;

static const QString SERVER = QStringLiteral("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");

class HearthCoreTest : public QObject
{
    Q_OBJECT
private Q_SLOTS:
    void addresses()
    {
        Location l = parseUrl(QUrl(QStringLiteral("hearth:/")));
        QVERIFY(l.valid);
        QCOMPARE(l.depth(), 0);

        l = parseUrl(QUrl(QStringLiteral("hearth:/") + SERVER));
        QVERIFY(l.valid);
        QCOMPARE(l.depth(), 1);
        QCOMPARE(l.server, SERVER);

        l = parseUrl(QUrl(QStringLiteral("hearth:/%1/ab12/Films/Old/a b.mkv").arg(SERVER)));
        QVERIFY(l.valid);
        QCOMPARE(l.depth(), 2);
        QCOMPARE(l.share, QStringLiteral("ab12"));
        QCOMPARE(l.path, QStringLiteral("Films/Old/a b.mkv"));
        QCOMPARE(l.name(), QStringLiteral("a b.mkv"));
        QCOMPARE(l.parentPath(), QStringLiteral("Films/Old"));

        // Hearth Files' own form, with a short id as the host.
        l = parseUrl(QUrl(QStringLiteral("hearth://0123456789abcdef/ab12/x.txt")));
        QVERIFY(l.valid);
        QCOMPARE(l.server, QStringLiteral("0123456789abcdef"));
        QCOMPARE(l.path, QStringLiteral("x.txt"));

        // Round trip.
        QCOMPARE(parseUrl(makeUrl(SERVER, QStringLiteral("ab12"), QStringLiteral("a/b"))).path, QStringLiteral("a/b"));
        QCOMPARE(makeUrl(SERVER).path(), QStringLiteral("/") + SERVER);
    }

    void refusedAddresses()
    {
        QVERIFY(!parseUrl(QUrl(QStringLiteral("file:///tmp"))).valid);
        QVERIFY(!parseUrl(QUrl(QStringLiteral("hearth:/notanid"))).valid);
        QVERIFY(!parseUrl(QUrl(QStringLiteral("hearth:/0123456789abcde"))).valid); // 15: too short
        QVERIFY(!parseUrl(QUrl(QStringLiteral("hearth:/%1/xyz").arg(SERVER))).valid);
        QVERIFY(!parseUrl(QUrl(QStringLiteral("hearth:/%1/ab12/.ssh").arg(SERVER))).valid);
        QVERIFY(!parseUrl(QUrl(QStringLiteral("hearth:/%1/ab12/a/.hidden/b").arg(SERVER))).valid);
        // QUrl takes ".." out itself; whatever is left must still be inside the share.
        const Location l = parseUrl(QUrl(QStringLiteral("hearth:/%1/ab12/a/../../..").arg(SERVER)));
        QVERIFY(!l.valid || l.depth() < 2 || !l.path.contains(QLatin1String("..")));
    }

    void answers()
    {
        QJsonValue data;
        QString err;
        QVERIFY(parseAnswer("{\"id\":1,\"ok\":true,\"data\":{\"a\":1}}", &data, &err));
        QCOMPARE(data.toObject().value(QStringLiteral("a")).toInt(), 1);
        QVERIFY(!parseAnswer("{\"id\":1,\"ok\":false,\"error\":\"That isn't a shared folder\"}", &data, &err));
        QCOMPARE(err, QStringLiteral("That isn't a shared folder"));
        QVERIFY(!parseAnswer("not json", &data, &err));
    }

    void linkNotRunning()
    {
        QTemporaryDir d;
        LinkClient c(d.filePath(QStringLiteral("link.sock")));
        QJsonValue data;
        QString err;
        QVERIFY(!c.call(QStringLiteral("storage.status"), QJsonObject(), &data, &err, 2000));
        QCOMPARE(err, QStringLiteral("Hearth Link isn't running"));
    }

    void defaultSocket()
    {
        qputenv("HEARTH_LINK_SOCKET", "/x/y.sock");
        QCOMPARE(LinkClient::defaultSocketPath(), QStringLiteral("/x/y.sock"));
        qunsetenv("HEARTH_LINK_SOCKET");
        qputenv("XDG_RUNTIME_DIR", "/run/user/1000");
        QCOMPARE(LinkClient::defaultSocketPath(), QStringLiteral("/run/user/1000/hearth-link/link.sock"));
    }

    void servers()
    {
        const QByteArray json = "{\"servers\":[\n"
            "            {\"id\":\"" + SERVER.toLatin1() + "\",\"name\":\"Den PC\",\"media\":false,\"shares\":[{\"id\":\"AB12\",\"name\":\"Movies\"},{\"id\":\"zz\",\"name\":\"bad\"}]},\n"
            "            {\"id\":\"nope\",\"name\":\"bad\"},\n"
            "            {\"id\":\"fedcba9876543210\",\"name\":\"\",\"shares\":null}]}";
        const QJsonObject st = QJsonDocument::fromJson(json).object();
        const QList<Server> s = serversFromStatus(st);
        QCOMPARE(s.size(), 2);
        QCOMPARE(s[0].name, QStringLiteral("Den PC"));
        QVERIFY(!s[0].media);
        QCOMPARE(s[0].shares.size(), 1);
        QCOMPARE(s[0].shares[0].name, QStringLiteral("ab12"));
        QCOMPARE(s[0].shares[0].display, QStringLiteral("Movies"));
        QCOMPARE(s[1].name, QStringLiteral("Hearth Storage"));
        QVERIFY(s[1].media);
        QVERIFY(findServer(st, SERVER).known);
        QVERIFY(!findServer(st, QStringLiteral("1111111111111111")).known);
    }

    void folderEntries()
    {
        const QJsonObject ls = QJsonDocument::fromJson("{\"share\":\"Movies\",\"path\":\"\",\"entries\":[\n"
            "            {\"name\":\"Films\",\"dir\":true,\"size\":0,\"mtime\":1700000000},\n"
            "            {\"name\":\"notes.txt\",\"dir\":false,\"size\":11,\"mtime\":1700000001},\n"
            "            {\"name\":\".hidden\",\"dir\":false,\"size\":1},\n"
            "            {\"name\":\"a/b\",\"dir\":false,\"size\":1},\n"
            "            {\"name\":\"\",\"dir\":false}]}")
                                   .object();
        const QList<Item> items = itemsFromLs(ls);
        QCOMPARE(items.size(), 2);
        QVERIFY(items[0].dir);
        QCOMPARE(items[1].size, qint64(11));
        QCOMPARE(items[1].mtime, qint64(1700000001));
    }

    void streamLinks()
    {
        HttpTarget t;
        QVERIFY(parseLocalUrl(QStringLiteral("http://127.0.0.1:4711/abcd/film.mkv"), &t));
        QCOMPARE(t.port, quint16(4711));
        QCOMPARE(t.path, QByteArray("/abcd/film.mkv"));
        QVERIFY(!parseLocalUrl(QStringLiteral("http://example.com:4711/abcd/film.mkv"), &t));
        QVERIFY(!parseLocalUrl(QStringLiteral("https://127.0.0.1:4711/a"), &t));
        QVERIFY(!parseLocalUrl(QStringLiteral("http://127.0.0.1/a"), &t));
        QVERIFY(!parseLocalUrl(QStringLiteral("http://user@127.0.0.1:1/a"), &t));
    }

    void streamHead()
    {
        HttpHead h;
        QVERIFY(parseHead("HTTP/1.1 200 OK\r\nContent-Type: video/x-matroska\r\ncontent-length: 1234\r\nAccept-Ranges: bytes", &h));
        QCOMPARE(h.status, 200);
        QCOMPARE(h.contentLength, qint64(1234));
        QCOMPARE(h.contentType, QByteArray("video/x-matroska"));
        HttpHead n;
        QVERIFY(parseHead("HTTP/1.1 404 Not Found\r\nContent-Length: 0", &n));
        QCOMPARE(n.status, 404);
        HttpHead bad;
        QVERIFY(!parseHead("garbage", &bad));
    }

    void readOnlyWords()
    {
        QVERIFY(readOnlyMessage().contains(QLatin1String("can't be changed from here")));
    }
};

QTEST_GUILESS_MAIN(HearthCoreTest)
#include "hearthcoretest.moc"
