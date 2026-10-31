// hearth:/ through KIO, as Hearth Files and every KDE file dialog use it: KIO starts the built worker (found through
// QT_PLUGIN_PATH, set by CTest) in its own process; this test plays Hearth Link (its socket, answering
// storage.status / storage.ls / storage.stream like hosts/hearth-link storage.rs) and Hearth Link's local stream.
#include <KIO/ListJob>
#include <KIO/MkdirJob>
#include <KIO/StatJob>
#include <KIO/StoredTransferJob>
#include <KIO/UDSEntry>
#include <KProtocolInfo>

#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLocalServer>
#include <QLocalSocket>
#include <QTcpServer>
#include <QTcpSocket>
#include <QTemporaryDir>
#include <QTest>

static const QString SERVER = QStringLiteral("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
static const QByteArray BODY = QByteArray("hello from the den\n").repeated(20000); // ~380 KB: several pieces

class KioHearthTest : public QObject
{
    Q_OBJECT
private:
    QTemporaryDir m_dir;
    QLocalServer m_link;
    QTcpServer m_http;
    QStringList m_calls;

    QJsonObject answer(const QString &cmd, const QJsonObject &a, QString *error)
    {
        m_calls << cmd;
        if (cmd == QLatin1String("storage.status")) {
            QJsonObject share{{QStringLiteral("id"), QStringLiteral("ab12")}, {QStringLiteral("name"), QStringLiteral("Movies")}};
            QJsonObject server{{QStringLiteral("id"), SERVER},
                               {QStringLiteral("name"), QStringLiteral("Den PC")},
                               {QStringLiteral("media"), true},
                               {QStringLiteral("shares"), QJsonArray{share}}};
            return QJsonObject{{QStringLiteral("servers"), QJsonArray{server}}};
        }
        if (cmd == QLatin1String("storage.ls")) {
            if (a.value(QStringLiteral("server")).toString() != SERVER || a.value(QStringLiteral("share")).toString() != QLatin1String("ab12")) {
                *error = QStringLiteral("That isn't a shared folder");
                return {};
            }
            const QString path = a.value(QStringLiteral("path")).toString();
            auto e = [](const QString &n, bool dir, qint64 size) {
                return QJsonObject{{QStringLiteral("name"), n}, {QStringLiteral("dir"), dir}, {QStringLiteral("size"), size}, {QStringLiteral("mtime"), 1700000000}};
            };
            if (path.isEmpty()) {
                return QJsonObject{{QStringLiteral("share"), QStringLiteral("Movies")},
                                   {QStringLiteral("entries"),
                                    QJsonArray{e(QStringLiteral("Films"), true, 0), e(QStringLiteral("notes.txt"), false, BODY.size()), e(QStringLiteral(".secret"), false, 1)}}};
            }
            if (path == QLatin1String("Films")) {
                return QJsonObject{{QStringLiteral("entries"), QJsonArray{e(QStringLiteral("a.mkv"), false, 5)}}};
            }
            if (path == QLatin1String("notes.txt")) {
                *error = QStringLiteral("That's a file, not a folder");
                return {};
            }
            *error = QStringLiteral("That folder can't be opened");
            return {};
        }
        if (cmd == QLatin1String("storage.stream")) {
            return QJsonObject{{QStringLiteral("url"), QStringLiteral("http://127.0.0.1:%1/tok/notes.txt").arg(m_http.serverPort())},
                               {QStringLiteral("name"), QStringLiteral("notes.txt")}};
        }
        *error = QStringLiteral("Unknown command");
        return {};
    }

    template<typename J>
    bool run(J *job)
    {
        job->setUiDelegate(nullptr);
        return job->exec();
    }

    QUrl url(const QString &rest = QString())
    {
        return QUrl(QStringLiteral("hearth:/") + rest);
    }

private Q_SLOTS:
    void initTestCase()
    {
        QVERIFY(m_dir.isValid());
        const QString sock = m_dir.filePath(QStringLiteral("link.sock"));
        QVERIFY(m_link.listen(sock));
        qputenv("HEARTH_LINK_SOCKET", sock.toLocal8Bit());
        connect(&m_link, &QLocalServer::newConnection, this, [this] {
            while (QLocalSocket *c = m_link.nextPendingConnection()) {
                connect(c, &QLocalSocket::readyRead, c, [this, c] {
                    if (!c->canReadLine()) {
                        return;
                    }
                    const QJsonObject req = QJsonDocument::fromJson(c->readLine()).object();
                    QString err;
                    const QJsonObject data = answer(req.value(QStringLiteral("cmd")).toString(), req.value(QStringLiteral("args")).toObject(), &err);
                    QJsonObject rep{{QStringLiteral("id"), req.value(QStringLiteral("id"))}, {QStringLiteral("ok"), err.isEmpty()}};
                    if (err.isEmpty()) {
                        rep.insert(QStringLiteral("data"), data);
                    } else {
                        rep.insert(QStringLiteral("error"), err);
                    }
                    c->write(QJsonDocument(rep).toJson(QJsonDocument::Compact) + '\n');
                    c->flush();
                });
                connect(c, &QLocalSocket::disconnected, c, &QObject::deleteLater);
            }
        });
        QVERIFY(m_http.listen(QHostAddress::LocalHost, 0));
        connect(&m_http, &QTcpServer::newConnection, this, [this] {
            while (QTcpSocket *c = m_http.nextPendingConnection()) {
                auto *buf = new QByteArray;
                connect(c, &QTcpSocket::readyRead, c, [c, buf] {
                    *buf += c->readAll();
                    if (!buf->contains("\r\n\r\n")) {
                        return;
                    }
                    if (buf->startsWith("GET /tok/")) {
                        c->write("HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: " + QByteArray::number(BODY.size()) + "\r\nConnection: close\r\n\r\n");
                        c->write(BODY);
                    } else {
                        c->write("HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    }
                    c->disconnectFromHost();
                });
                connect(c, &QTcpSocket::disconnected, c, [c, buf] {
                    delete buf;
                    c->deleteLater();
                });
            }
        });
        QVERIFY2(KProtocolInfo::isKnownProtocol(QStringLiteral("hearth")), "the hearth worker isn't found (QT_PLUGIN_PATH)");
        QCOMPARE(KProtocolInfo::protocolClass(QStringLiteral("hearth")), QStringLiteral(":local"));
    }

    void listServers()
    {
        auto *job = KIO::listDir(url(), KIO::HideProgressInfo);
        KIO::UDSEntryList all;
        connect(job, &KIO::ListJob::entries, this, [&all](KIO::Job *, const KIO::UDSEntryList &l) {
            all += l;
        });
        QVERIFY2(run(job), qPrintable(job->errorString()));
        QStringList names;
        for (const KIO::UDSEntry &e : std::as_const(all)) {
            if (e.stringValue(KIO::UDSEntry::UDS_NAME) != QLatin1String(".")) {
                names << e.stringValue(KIO::UDSEntry::UDS_NAME) + QLatin1Char('=') + e.stringValue(KIO::UDSEntry::UDS_DISPLAY_NAME);
            }
        }
        QCOMPARE(names, QStringList{SERVER + QStringLiteral("=Den PC")});
    }

    void listShares()
    {
        auto *job = KIO::listDir(url(SERVER), KIO::HideProgressInfo);
        KIO::UDSEntryList all;
        connect(job, &KIO::ListJob::entries, this, [&all](KIO::Job *, const KIO::UDSEntryList &l) {
            all += l;
        });
        QVERIFY2(run(job), qPrintable(job->errorString()));
        bool found = false;
        for (const KIO::UDSEntry &e : std::as_const(all)) {
            if (e.stringValue(KIO::UDSEntry::UDS_NAME) == QLatin1String("ab12")) {
                found = e.isDir() && e.stringValue(KIO::UDSEntry::UDS_DISPLAY_NAME) == QLatin1String("Movies");
            }
        }
        QVERIFY(found);
    }

    void listFolder()
    {
        auto *job = KIO::listDir(url(SERVER + QStringLiteral("/ab12")), KIO::HideProgressInfo);
        KIO::UDSEntryList all;
        connect(job, &KIO::ListJob::entries, this, [&all](KIO::Job *, const KIO::UDSEntryList &l) {
            all += l;
        });
        QVERIFY2(run(job), qPrintable(job->errorString()));
        QMap<QString, KIO::UDSEntry> byName;
        for (const KIO::UDSEntry &e : std::as_const(all)) {
            byName.insert(e.stringValue(KIO::UDSEntry::UDS_NAME), e);
        }
        QVERIFY(byName.contains(QStringLiteral("Films")));
        QVERIFY(byName.value(QStringLiteral("Films")).isDir());
        QVERIFY(byName.contains(QStringLiteral("notes.txt")));
        QCOMPARE(byName.value(QStringLiteral("notes.txt")).numberValue(KIO::UDSEntry::UDS_SIZE), qint64(BODY.size()));
        QCOMPARE(byName.value(QStringLiteral("notes.txt")).stringValue(KIO::UDSEntry::UDS_MIME_TYPE), QStringLiteral("text/plain"));
        QVERIFY2(!byName.contains(QStringLiteral(".secret")), "hidden files are never listed");
    }

    void statFile()
    {
        auto *job = KIO::stat(url(SERVER + QStringLiteral("/ab12/notes.txt")), KIO::StatJob::SourceSide, KIO::StatDefaultDetails, KIO::HideProgressInfo);
        QVERIFY2(run(job), qPrintable(job->errorString()));
        QVERIFY(!job->statResult().isDir());
        QCOMPARE(job->statResult().numberValue(KIO::UDSEntry::UDS_SIZE), qint64(BODY.size()));

        auto *missing = KIO::stat(url(SERVER + QStringLiteral("/ab12/nothere.txt")), KIO::StatJob::SourceSide, KIO::StatDefaultDetails, KIO::HideProgressInfo);
        QVERIFY(!run(missing));
        QCOMPARE(missing->error(), int(KIO::ERR_DOES_NOT_EXIST));
    }

    void readFile()
    {
        auto *job = KIO::storedGet(url(SERVER + QStringLiteral("/ab12/notes.txt")), KIO::NoReload, KIO::HideProgressInfo);
        QVERIFY2(run(job), qPrintable(job->errorString()));
        QCOMPARE(job->data().size(), BODY.size());
        QCOMPARE(job->data(), BODY);
    }

    void listingAFileSaysSo()
    {
        auto *job = KIO::listDir(url(SERVER + QStringLiteral("/ab12/notes.txt")), KIO::HideProgressInfo);
        QVERIFY(!run(job));
        QCOMPARE(job->error(), int(KIO::ERR_IS_FILE));
    }

    void readOnly()
    {
        auto *job = KIO::mkdir(url(SERVER + QStringLiteral("/ab12/New")));
        QVERIFY(!run(job));
        // The worker's own words, or KIO's "not supported" when it stops it first (hearth.json: makedir false).
        QVERIFY2(job->errorString().contains(QLatin1String("can't be changed from here")) || job->error() == KIO::ERR_UNSUPPORTED_ACTION,
                 qPrintable(job->errorString()));
        QVERIFY(!m_calls.contains(QStringLiteral("storage.get")));
    }

    void badAddress()
    {
        auto *job = KIO::listDir(url(QStringLiteral("zz")), KIO::HideProgressInfo);
        QVERIFY(!run(job));
        QCOMPARE(job->error(), int(KIO::ERR_MALFORMED_URL));
    }
};

QTEST_GUILESS_MAIN(KioHearthTest)
#include "kiohearthtest.moc"
