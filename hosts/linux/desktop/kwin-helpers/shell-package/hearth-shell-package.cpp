// The "Plasma/Shell" package type for KWin's screen locker, for Debian trixie (docs/KWIN-INPUT.md).
// KWin's locker (kscreenlocker_greet, Plasma 6.3) reads its lock screen from a Plasma/Shell package
// (lockscreen/LockScreen.qml), but on Debian trixie no package ships that type: newer libplasma has it
// (plasma_shell), Plasma 6.3 had it only inside plasmashell. So Hearth's lock screen (kwin-lock) couldn't load and
// KWin's locker would stay on its emergency screen. This is that type with only what the locker reads. Installed by
// kwin-lock/install.sh only when nothing else provides Plasma/Shell.
#include <KPackage/Package>
#include <KPackage/PackageStructure>
#include <KPluginFactory>

class HearthShellPackage : public KPackage::PackageStructure
{
    Q_OBJECT
public:
    using KPackage::PackageStructure::PackageStructure;

    void initPackage(KPackage::Package *package) override
    {
        package->setDefaultPackageRoot(QStringLiteral("plasma/shells/"));
        package->addDirectoryDefinition("lockscreen", QStringLiteral("lockscreen"));
        package->addFileDefinition("lockscreenmainscript", QStringLiteral("lockscreen/LockScreen.qml"));
        package->setRequired("lockscreenmainscript", true);
        package->addDirectoryDefinition("images", QStringLiteral("images"));
        package->addDirectoryDefinition("config", QStringLiteral("config"));
    }
};

K_PLUGIN_CLASS_WITH_JSON(HearthShellPackage, "hearth-shell-package.json")

#include "hearth-shell-package.moc"
