// Welcome & Tips' help (ui/help.json) is made here from the website's docs (tools/build-docs.py --help-json), so
// every build carries the same answers as hearthos.fun/docs. Without Python a small stand-in is written instead
// and the build goes on (the tour still works; the help says to look on the website).
use std::path::Path;
use std::process::Command;

fn main() {
    let help = Path::new("ui/help.json");
    let script = Path::new("../../tools/build-docs.py");
    for f in ["../../tools/build-docs.py", "../../docs/ROADMAP.md", "../../hosts/linux/desktop/shell/Help.qml", "../../hosts/windows/Nimbus.Host/Nimbus.Host.csproj"] {
        println!("cargo:rerun-if-changed={f}");
    }
    let made = script.exists()
        && Command::new("python3").arg(script).arg("--help-json").arg(help).status().map(|s| s.success()).unwrap_or(false);
    if !made {
        println!("cargo:warning=Welcome & Tips: couldn't make ui/help.json from tools/build-docs.py (is python3 there?); using a stand-in");
        if !help.exists() {
            let _ = std::fs::write(help, r#"{"version":"","site":"https://hearthos.fun/docs/","pages":[],"sections":[],"controls":[]}"#);
        }
    }
    tauri_build::build()
}
