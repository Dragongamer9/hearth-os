// Hearth's Game Mode drawn by Qt Quick (Experimental): docs/RENDERERS.md. Everything Hearth does is the Hearth core's
// (the same program as the usual Hearth: hearth-core and Game Mode), started windowless as `nimbus --core-stdio`; this
// program only draws the screens (qml/) and does what the core asks of its windows. Started by the `nimbus` program
// itself when Settings › Display › Renderer is "Qt (Experimental)", so the session and its watchdog don't change;
// `nimbus` waits for it and counts a crash (3 within a few minutes give the Qt renderer up).
//
//   hearth-qml [Hearth's own arguments: --session --windowed --page X]
//   hearth-qml --self-test [--shots DIR]   load every screen against the core in HEARTH_CORE_BIN (a fake one in CI),
//                                           save a picture of each, and leave 0 when none had a QML error
#include "corelink.h"
#include "host.h"
#include "pad.h"

#include <QFontDatabase>
#include <QGuiApplication>
#include <QQmlApplicationEngine>
#include <QQmlContext>
#include <QQuickWindow>
#include <QTimer>
#include <csignal>
#include <sys/prctl.h>

int main(int argc, char **argv) {
    // Started by `nimbus` (which waits for it): leave with it, so a stopped session leaves nothing behind.
    if (qEnvironmentVariableIsSet("HEARTH_QT_PARENT")) prctl(PR_SET_PDEATHSIG, SIGTERM);
    // In Game Mode (gamescope) Hearth's windows are X11 windows, found by their titles ("Hearth", "Hearth Overlay").
    if (!qEnvironmentVariableIsSet("QT_QPA_PLATFORM") && qEnvironmentVariable("NIMBUS_SESSION") == "1" && qEnvironmentVariableIsSet("DISPLAY"))
        qputenv("QT_QPA_PLATFORM", "xcb");
    // The software scene graph when asked (CI, or a PC whose OpenGL doesn't work: HEARTH_QT_SOFTWARE=1).
    if (qEnvironmentVariable("HEARTH_QT_SOFTWARE") == "1") QQuickWindow::setGraphicsApi(QSGRendererInterface::Software);
    Host::installLog();

    QGuiApplication app(argc, argv);
    app.setApplicationName("hearth-qml");
    app.setApplicationDisplayName("Hearth");
    app.setDesktopFileName("hearth");

    Host host(app.arguments());
    host.log(QString("starting: Qt %1, %2 platform, %3 scene graph%4")
                 .arg(QString::fromLatin1(qVersion()), QGuiApplication::platformName(),
                      QQuickWindow::graphicsApi() == QSGRendererInterface::Software ? "software" : "hardware",
                      host.selfTest() ? ", self-test" : ""));

    // Anybody (SIL Open Font License), the face of Hearth's website: headings only. Built in when CI made it.
    const int fid = QFontDatabase::addApplicationFont(":/fonts/anybody-hearth.ttf");
    if (fid >= 0) { const QStringList fam = QFontDatabase::applicationFontFamilies(fid); if (!fam.isEmpty()) host.setDisplayFont(fam.first()); }

    CoreLink core(&host);
    Pad pad(&host);

    QQmlApplicationEngine engine;
    engine.rootContext()->setContextProperty("hostApp", &host);
    engine.rootContext()->setContextProperty("coreLink", &core);
    engine.rootContext()->setContextProperty("padInput", &pad);
    QObject::connect(&engine, &QQmlApplicationEngine::objectCreationFailed, &app, [] { QCoreApplication::exit(2); }, Qt::QueuedConnection);
    engine.loadFromModule("HearthQml", "Main");
    if (engine.rootObjects().isEmpty()) { host.log("the screens couldn't be loaded"); return 2; }

    // The self-test never hangs CI: two minutes at most.
    if (host.selfTest()) QTimer::singleShot(120000, &app, [&host] { host.log("self-test: took too long"); QCoreApplication::exit(3); });
    core.start();
    const int code = app.exec();
    core.stop();
    return code;
}
