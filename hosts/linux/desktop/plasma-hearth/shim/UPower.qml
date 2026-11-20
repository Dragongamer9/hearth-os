pragma Singleton

// The battery for Mantel's bar (plasma-hearth): Plasma's powermanagement source (told when it changes).
import QtQuick
import org.kde.plasma.plasma5support as P5Support

Item {
    P5Support.DataSource { id: pm; engine: "powermanagement"; connectedSources: ["Battery"] }
    readonly property var b: pm.data["Battery"] || ({})
    readonly property var displayDevice: ({
        isLaptopBattery: b["Has Battery"] === true || b["Has Cumulative"] === true,
        percentage: (Number(b["Percent"]) || 0) / 100,
        state: b["State"] === "Charging" ? 1 : b["State"] === "FullyCharged" ? 4 : b["State"] === "Discharging" ? 2 : 0
    })
}
