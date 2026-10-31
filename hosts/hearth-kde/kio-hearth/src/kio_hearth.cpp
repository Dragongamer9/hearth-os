// kio_hearth: hearth:/ for every KDE app (see hearthurl.h for the addresses, docs/KDE-APPS.md for the rest).
// Read-only, like Hearth Files' Hearth Storage places: listing, details and opening/copying files from a share.
#include "hearthlink.h"
#include "hearthurl.h"
#include "httpget.h"
#include "items.h"

#include <KIO/WorkerBase>

#include <QCoreApplication>
#include <QJsonArray>
#include <QMimeDatabase>

#include <cstdio>
#include <sys/stat.h>

using namespace KIO;

class KIOPluginForMetaData : public QObject
{
    Q_OBJECT
    Q_PLUGIN_METADATA(IID "org.kde.kio.worker.hearth" FILE "hearth.json")
};

namespace
{

QString mimeFor(const QString &name)
{
    static QMimeDatabase db;
    return db.mimeTypeForFile(name, QMimeDatabase::MatchExtension).name();
}

UDSEntry toEntry(const Hearth::Item &it)
{
    UDSEntry e;
    e.reserve(8);
    e.fastInsert(UDSEntry::UDS_NAME, it.name);
    if (it.display != it.name) {
        e.fastInsert(UDSEntry::UDS_DISPLAY_NAME, it.display);
    }
    e.fastInsert(UDSEntry::UDS_FILE_TYPE, it.dir ? S_IFDIR : S_IFREG);
    e.fastInsert(UDSEntry::UDS_ACCESS, it.dir ? 0555 : 0444);
    e.fastInsert(UDSEntry::UDS_SIZE, it.size);
    if (it.mtime > 0) {
        e.fastInsert(UDSEntry::UDS_MODIFICATION_TIME, it.mtime);
    }
    // Known from the name: KIO then never has to read a file to find out what it is.
    e.fastInsert(UDSEntry::UDS_MIME_TYPE, it.dir ? QStringLiteral("inode/directory") : mimeFor(it.name));
    if (!it.icon.isEmpty()) {
        e.fastInsert(UDSEntry::UDS_ICON_NAME, it.icon);
    }
    return e;
}

Hearth::Item dirItem(const QString &name, const QString &display, const QString &icon)
{
    Hearth::Item it;
    it.name = name;
    it.display = display;
    it.dir = true;
    it.icon = icon;
    return it;
}

} // namespace

class HearthWorker : public WorkerBase
{
public:
    HearthWorker(const QByteArray &pool, const QByteArray &app)
        : WorkerBase(QByteArrayLiteral("hearth"), pool, app)
    {
    }

    WorkerResult listDir(const QUrl &url) override
    {
        const Hearth::Location loc = Hearth::parseUrl(url);
        if (!loc.valid) {
            return WorkerResult::fail(ERR_MALFORMED_URL, url.toDisplayString());
        }
        QList<Hearth::Item> items;
        if (loc.depth() < 2) {
            QJsonObject st;
            if (const WorkerResult r = status(&st); !r.success()) {
                return r;
            }
            if (loc.depth() == 0) {
                const QList<Hearth::Server> servers = Hearth::serversFromStatus(st);
                for (const Hearth::Server &s : servers) {
                    items << dirItem(s.id, s.name, QStringLiteral("network-server"));
                }
            } else {
                Hearth::Server s = Hearth::findServer(st, loc.server);
                if (s.known && s.shares.isEmpty()) {
                    // Not asked yet since Hearth Link started (or it was asleep): ask it now (wakes it if it can).
                    QJsonValue ignored;
                    QString err;
                    QJsonObject a;
                    a.insert(QStringLiteral("peer"), loc.server);
                    m_link.call(QStringLiteral("storage.servers"), a, &ignored, &err, 90000);
                    if (status(&st).success()) {
                        s = Hearth::findServer(st, loc.server);
                    }
                }
                if (!s.known) {
                    return WorkerResult::fail(ERR_DOES_NOT_EXIST, url.toDisplayString());
                }
                items = s.shares;
            }
        } else {
            QJsonObject ls;
            if (const WorkerResult r = list(loc.server, loc.share, loc.path, &ls, url); !r.success()) {
                return r;
            }
            items = Hearth::itemsFromLs(ls);
        }
        UDSEntry dot = toEntry(dirItem(QStringLiteral("."), QStringLiteral("."), QString()));
        listEntry(dot);
        for (const Hearth::Item &it : std::as_const(items)) {
            listEntry(toEntry(it));
        }
        return WorkerResult::pass();
    }

    WorkerResult stat(const QUrl &url) override
    {
        const Hearth::Location loc = Hearth::parseUrl(url);
        if (!loc.valid) {
            return WorkerResult::fail(ERR_MALFORMED_URL, url.toDisplayString());
        }
        if (loc.depth() == 0) {
            statEntry(toEntry(dirItem(QStringLiteral("hearth"), QStringLiteral("Hearth Storage"), QStringLiteral("network-server"))));
            return WorkerResult::pass();
        }
        if (loc.depth() == 1 || loc.path.isEmpty()) {
            QJsonObject st;
            if (const WorkerResult r = status(&st); !r.success()) {
                return r;
            }
            const Hearth::Server s = Hearth::findServer(st, loc.server);
            if (!s.known) {
                return WorkerResult::fail(ERR_DOES_NOT_EXIST, url.toDisplayString());
            }
            if (loc.depth() == 1) {
                statEntry(toEntry(dirItem(s.id, s.name, QStringLiteral("network-server"))));
                return WorkerResult::pass();
            }
            for (const Hearth::Item &sh : s.shares) {
                if (sh.name == loc.share) {
                    statEntry(toEntry(sh));
                    return WorkerResult::pass();
                }
            }
            // A share the status doesn't list (yet): a folder if the server can list it.
            QJsonObject ls;
            if (const WorkerResult r = list(loc.server, loc.share, QString(), &ls, url); !r.success()) {
                return r;
            }
            const QString name = ls.value(QStringLiteral("share")).toString();
            statEntry(toEntry(dirItem(loc.share, name.isEmpty() ? QStringLiteral("Shared folder") : name, QStringLiteral("folder-network"))));
            return WorkerResult::pass();
        }
        // In a share: the entry from its folder's list (the server's storage.ls is what Hearth Link offers).
        QJsonObject ls;
        if (const WorkerResult r = list(loc.server, loc.share, loc.parentPath(), &ls, url); !r.success()) {
            return r;
        }
        const QString name = loc.name();
        const QList<Hearth::Item> items = Hearth::itemsFromLs(ls);
        for (const Hearth::Item &it : items) {
            if (it.name == name) {
                statEntry(toEntry(it));
                return WorkerResult::pass();
            }
        }
        return WorkerResult::fail(ERR_DOES_NOT_EXIST, url.toDisplayString());
    }

    WorkerResult mimetype(const QUrl &url) override
    {
        const Hearth::Location loc = Hearth::parseUrl(url);
        if (!loc.valid) {
            return WorkerResult::fail(ERR_MALFORMED_URL, url.toDisplayString());
        }
        mimeType(loc.depth() < 2 || loc.path.isEmpty() ? QStringLiteral("inode/directory") : mimeFor(loc.name()));
        return WorkerResult::pass();
    }

    WorkerResult get(const QUrl &url) override
    {
        const Hearth::Location loc = Hearth::parseUrl(url);
        if (!loc.valid) {
            return WorkerResult::fail(ERR_MALFORMED_URL, url.toDisplayString());
        }
        if (loc.depth() < 2 || loc.path.isEmpty()) {
            return WorkerResult::fail(ERR_IS_DIRECTORY, url.toDisplayString());
        }
        QJsonObject st;
        if (status(&st).success()) {
            const Hearth::Server s = Hearth::findServer(st, loc.server);
            if (s.known && !s.media) {
                return WorkerResult::fail(ERR_WORKER_DEFINED,
                                          QStringLiteral("Opening files from %1 needs Media turned on there (Settings › Hearth Storage › Media), "
                                                         "or copy the file to this device with Hearth Files")
                                              .arg(s.name));
            }
        }
        QJsonObject a;
        a.insert(QStringLiteral("server"), loc.server);
        a.insert(QStringLiteral("share"), loc.share);
        a.insert(QStringLiteral("path"), loc.path);
        QJsonValue v;
        QString err;
        if (!m_link.call(QStringLiteral("storage.stream"), a, &v, &err)) {
            return linkError(err, url);
        }
        const QJsonObject o = v.toObject();
        Hearth::HttpTarget target;
        if (!Hearth::parseLocalUrl(o.value(QStringLiteral("url")).toString(), &target)) {
            return WorkerResult::fail(ERR_WORKER_DEFINED, QStringLiteral("Hearth Link gave a link that isn't on this device"));
        }
        const QString name = o.value(QStringLiteral("name")).toString().isEmpty() ? loc.name() : o.value(QStringLiteral("name")).toString();
        Hearth::HttpGet http;
        Hearth::HttpHead head;
        if (!http.open(target, &head, &err)) {
            return WorkerResult::fail(ERR_WORKER_DEFINED, err);
        }
        mimeType(mimeFor(name));
        if (head.contentLength >= 0) {
            totalSize(filesize_t(head.contentLength));
        }
        QByteArray buf(256 * 1024, Qt::Uninitialized);
        qint64 got = 0;
        for (;;) {
            if (wasKilled()) {
                return WorkerResult::pass();
            }
            const qint64 n = http.read(buf.data(), buf.size());
            if (n == 0) {
                break;
            }
            if (n < 0) {
                return WorkerResult::fail(ERR_WORKER_DEFINED, QStringLiteral("Hearth Storage stopped sending %1").arg(name));
            }
            data(QByteArray(buf.constData(), int(n)));
            got += n;
            processedSize(filesize_t(got));
            if (head.contentLength >= 0 && got >= head.contentLength) {
                break;
            }
        }
        if (head.contentLength >= 0 && got < head.contentLength) {
            return WorkerResult::fail(ERR_WORKER_DEFINED, QStringLiteral("%1 ended early: is the server still on, with Media on?").arg(name));
        }
        data(QByteArray());
        return WorkerResult::pass();
    }

    // Nothing on Hearth Storage is changed from here (Hearth Link offers no request that would).
    WorkerResult put(const QUrl &, int, JobFlags) override
    {
        return readOnly();
    }
    WorkerResult mkdir(const QUrl &, int) override
    {
        return readOnly();
    }
    WorkerResult del(const QUrl &, bool) override
    {
        return readOnly();
    }
    WorkerResult rename(const QUrl &, const QUrl &, JobFlags) override
    {
        return readOnly();
    }
    WorkerResult chmod(const QUrl &, int) override
    {
        return readOnly();
    }

private:
    WorkerResult readOnly()
    {
        return WorkerResult::fail(ERR_WORKER_DEFINED, Hearth::readOnlyMessage());
    }

    WorkerResult linkError(const QString &err, const QUrl &url)
    {
        if (err.contains(QLatin1String("isn't there anymore")) || err.contains(QLatin1String("isn't shared"))) {
            return WorkerResult::fail(ERR_DOES_NOT_EXIST, url.toDisplayString());
        }
        if (err.contains(QLatin1String("a file, not a folder"))) {
            return WorkerResult::fail(ERR_IS_FILE, url.toDisplayString());
        }
        if (err.contains(QLatin1String("That's a folder"))) {
            return WorkerResult::fail(ERR_IS_DIRECTORY, url.toDisplayString());
        }
        return WorkerResult::fail(ERR_WORKER_DEFINED, err);
    }

    WorkerResult status(QJsonObject *st)
    {
        QJsonValue v;
        QString err;
        if (!m_link.call(QStringLiteral("storage.status"), QJsonObject(), &v, &err, 20000)) {
            return WorkerResult::fail(ERR_WORKER_DEFINED, err);
        }
        *st = v.toObject();
        return WorkerResult::pass();
    }

    WorkerResult list(const QString &server, const QString &share, const QString &path, QJsonObject *ls, const QUrl &url)
    {
        QJsonObject a;
        a.insert(QStringLiteral("server"), server);
        a.insert(QStringLiteral("share"), share);
        a.insert(QStringLiteral("path"), path);
        QJsonValue v;
        QString err;
        if (!m_link.call(QStringLiteral("storage.ls"), a, &v, &err)) {
            return linkError(err, url);
        }
        *ls = v.toObject();
        return WorkerResult::pass();
    }

    Hearth::LinkClient m_link;
};

extern "C" Q_DECL_EXPORT int kdemain(int argc, char **argv)
{
    QCoreApplication app(argc, argv);
    app.setApplicationName(QStringLiteral("kio_hearth"));
    if (argc != 4) {
        fprintf(stderr, "Usage: kio_hearth protocol domain-socket1 domain-socket2\n");
        return -1;
    }
    HearthWorker worker(argv[2], argv[3]);
    worker.dispatchLoop();
    return 0;
}

#include "kio_hearth.moc"
