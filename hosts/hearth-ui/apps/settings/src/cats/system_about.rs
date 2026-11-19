//! System › About, the Info Center pages and Install Hearth (Mantel's SetAbout.qml "about" part, SetInfoCenter.qml,
//! SetInstall.qml). About reads the same facts as Mantel's (os-release, /usr/share/nimbus/version and its date, DMI,
//! /proc), Rename goes through hearth-admin hostname; the Info Center pages are hearth-info's, one per part, with the
//! parts as tabs along the top and Copy this page. Install Hearth only matters on the live USB.

use super::*;
use hearth_ui::ui::{self, btn, tb, ts, Kind};
use iced::widget::{container, mouse_area, Row};

pub const ABOUT: &str = ". /etc/os-release 2>/dev/null; echo \"os=${PRETTY_NAME:-Linux}\"; echo \"ver=${IMAGE_VERSION:-$VERSION_ID}\"; \
echo \"hver=$(cat /usr/share/nimbus/version 2>/dev/null)\"; echo \"hdate=$(cat /usr/share/nimbus/version-date 2>/dev/null)\"; \
echo \"host=$(cat /etc/hostname 2>/dev/null || hostname)\"; echo \"model=$(cat /sys/devices/virtual/dmi/id/product_name 2>/dev/null)\"; \
echo \"vendor=$(cat /sys/devices/virtual/dmi/id/sys_vendor 2>/dev/null)\"; echo \"cpu=$(sed -n 's/^model name[[:space:]]*: //p' /proc/cpuinfo | head -1)\"; \
echo \"cores=$(nproc 2>/dev/null)\"; echo \"mem=$(awk '/MemTotal/ {printf \"%.0f GB\", $2/1048576}' /proc/meminfo)\"; \
echo \"gpu=$(lspci 2>/dev/null | sed -n 's/.*\\(VGA\\|3D\\|Display\\)[^:]*: //p' | head -1)\"; \
echo \"disk=$(df -h --output=avail,size /var/home 2>/dev/null | tail -1 | awk '{print $1\" free of \"$2}')\"; echo \"kernel=$(uname -r)\"; \
echo \"plasma=$(plasmashell --version 2>/dev/null | head -1)\"; echo \"user=$(id -un)\"";

/// "1.14.0" + "2026-11-02" as people see it: "1.14 · November 2, 2026".
pub fn version_words(v: &str, date: &str) -> String {
    if v.is_empty() || v == "0.0.0" {
        return String::new();
    }
    let shown = data::shown_version(v);
    let d: Vec<i64> = date.split('-').filter_map(|x| x.parse().ok()).collect();
    if d.len() == 3 && (1..=12).contains(&d[1]) {
        const M: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        return format!("{shown} · {} {}, {}", M[(d[1] - 1) as usize], d[2], d[0]);
    }
    shown
}

pub fn view<'a>(a: &'a App, page: &'static str) -> El<'a> {
    match page {
        "about" => about(a),
        "install" => install(a),
        _ => info(a, page),
    }
}

fn about(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let f = kv(st.raw("about"));
    let g = |k: &str| vs(&f, k);
    let mut c = Column::new().spacing(10.0);
    let hv = version_words(&g("hver"), &g("hdate"));
    let booted = data::shown_version(&st.updates.booted);
    let version = if !hv.is_empty() { hv } else if !booted.is_empty() { booted } else { g("ver") };
    let fedora = st.updates.booted_image.contains("hearth-os:") || g("os").contains("Fedora") || g("os").to_lowercase().contains("anatase");
    let logo = container(tb("H", 30.0, p.ink)).center_x(Length::Fixed(64.0)).center_y(Length::Fixed(64.0)).style(ui::boxed(p.accent, 16.0, None));
    c = c.push(row![logo, column![tb("Hearth", sz(22.0), p.text), t(if version.is_empty() { String::new() } else { format!("Hearth OS {version}{}", if fedora { " · Fedora edition" } else { "" }) }, sz(13.0), p.muted)].spacing(2.0)].spacing(16.0).align_y(iced::Center));
    c = c.push(match &st.rename {
        Some(n) => card(&p, crate::cats::home::rename_box(&p, n, Msg::RenameText, Msg::RenameSave, Msg::RenameCancel)),
        None => srow(&p, G::Info, if a.sh.info.host.is_empty() { "This PC".to_string() } else { a.sh.info.host.clone() }, "This PC's name on your network and your other devices", btn(&p, "Rename", Kind::Normal, false, Some(Msg::Rename))),
    });
    let cpu = if g("cpu").is_empty() { String::new() } else { format!("{}{}", data::short_cpu(&g("cpu")), if g("cores").is_empty() { String::new() } else { format!(" ({} threads)", g("cores")) }) };
    let device = [g("vendor"), g("model")].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" ");
    let desktop = if !g("plasma").is_empty() { format!("KDE Plasma · {}", g("plasma").replace("plasmashell ", "")) } else { a.sh.info.desktop.clone() };
    let facts: Vec<(&str, String)> = vec![
        ("Device", device),
        ("Name", g("host")),
        ("Signed in as", g("user")),
        ("Processor", cpu),
        ("Graphics", data::short_gpu(&g("gpu"))),
        ("Memory", g("mem")),
        ("Storage", g("disk")),
        ("System", g("os")),
        ("Version", version.clone()),
        ("Image", st.updates.booted_image.clone()),
        ("Kernel", g("kernel")),
        ("Desktop", desktop),
    ];
    let mut fc = Column::new().spacing(10.0);
    for (k, v) in facts.into_iter().filter(|x| !x.1.is_empty()) {
        fc = fc.push(fact(&p, k, v));
    }
    if !st.read("about") {
        fc = fc.push(t("Reading…", sz(13.0), p.muted));
    }
    c = c.push(card(&p, fc));
    // Mantel's Resource Monitor is a Quickshell window; here its live picture of the PC is Task Manager's Performance.
    c = c.push(link_row(&p, G::Pulse, "Resource Monitor", "See what this PC's parts and each program use, live (Task Manager › Performance)", Msg::App(sv(&["hearth-task-manager", "--page", "performance:cpu"]))));
    c = c.push(link_row(&p, G::Info, "Info Center", "Processor, graphics card, memory, Vulkan and OpenGL, PCI and USB devices, energy", Msg::Go("info-cpu")));
    c = c.push(link_row(&p, G::Info, "System Info", "Everything about this PC's parts, to copy or send", Msg::App(sv(&["hearth-system-info"]))));
    c.into()
}

const TABS: [(&str, &str); 10] = [
    ("info-cpu", "Processor"),
    ("info-gpu", "Graphics card"),
    ("info-memory", "Memory"),
    ("info-graphics", "Vulkan & OpenGL"),
    ("info-pci", "PCI"),
    ("info-usb", "USB"),
    ("info-energy", "Energy"),
    ("info-sensors", "Temperatures & fans"),
    ("info-modules", "Kernel modules"),
    ("info-interrupts", "Interrupts"),
];

/// The Info Center's parts as tabs (on every info page, Energy too).
pub fn tabs<'a>(p: &Pal, page: &str) -> El<'a> {
    let p = *p;
    let mut r = Row::new().spacing(6.0);
    for (id, label) in TABS {
        let lit = id == page;
        let tab = container(t1(label, sz(13.0), if lit { p.ink } else { p.text })).padding(iced::Padding { top: 7.0, right: 14.0, bottom: 7.0, left: 14.0 }).style(ui::boxed(if lit { p.accent } else { p.panel }, 17.0, None));
        r = r.push(if lit { El::from(tab) } else { mouse_area(tab).on_press(Msg::Go(id)).interaction(iced::mouse::Interaction::Pointer).into() });
    }
    r.wrap().into()
}

fn as_text(v: &Value) -> String {
    if let Some(items) = at(v, "items").as_array() {
        return items.iter().map(|i| ["id", "kind", "title", "sub", "value"].iter().map(|k| vs(i, k)).filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" · ")).collect::<Vec<_>>().join("\n");
    }
    va(v, "sections")
        .iter()
        .map(|s| format!("{}\n{}", vs(s, "title"), va(s, "rows").iter().map(|r| format!("  {}: {}", vs(r, "0"), vs(r, "1"))).collect::<Vec<_>>().join("\n")))
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn info<'a>(a: &'a App, page: &'static str) -> El<'a> {
    let p = a.sh.p;
    let st = &a.system;
    let v = st.j(page);
    let mut c = Column::new().spacing(10.0).push(tabs(&p, page));
    if !st.has(page) {
        if !st.read(page) {
            c = c.push(info_row(&p, G::Info, "Reading…", ""));
        } else {
            c = c.push(card(&p, column![ts("Couldn't read it", sz(16.0), p.text), fact(&p, "Problem", st.raw(page).to_string())].spacing(8.0)));
        }
        return c.into();
    }
    for s in va(v, "sections") {
        let mut sc = Column::new().spacing(6.0).push(ts(vs(s, "title"), sz(16.0), p.text));
        let meter = va(s, "meter");
        if meter.len() == 2 {
            sc = sc.push(bar_of(&p, vf(&meter[0], "", 0.0), vf(&meter[1], "", 1.0)));
        }
        for r in va(s, "rows") {
            sc = sc.push(fact(&p, vs(r, "0"), vs(r, "1")));
        }
        for gg in va(s, "gauges") {
            let val = vf(gg, "value", 0.0);
            let top = vf(gg, "top", 100.0).max(1.0);
            let hot = vb(gg, "hot");
            let col = if hot { p.danger } else if val / top > 0.75 { p.warn } else { p.accent };
            let txt = vs(gg, "text");
            let extra = txt.split_once('(').map(|x| x.1.trim_end_matches(')').to_string()).unwrap_or_default();
            sc = sc.push(row![
                container(t1(vs(gg, "label"), sz(13.0), p.muted)).width(Length::Fixed(sz(190.0))),
                container(ui::bar(vec![((val / top).clamp(0.02, 1.0) as f32, col)], Pal::mix(p.panel2, p.text, 0.08), 10.0, 5.0)).width(Length::Fill),
                container(t1(format!("{} °C", val.round()), sz(13.0), if hot { p.danger } else { p.text })).width(Length::Fixed(70.0)),
                container(t1(extra, sz(12.0), p.muted)).width(Length::Fixed(150.0))
            ].spacing(10.0).align_y(iced::Center));
        }
        let bars = va(s, "bars");
        if !bars.is_empty() {
            let top = vf(s, "top", 1.0).max(1.0);
            let mut br = Row::new().spacing(3.0).align_y(iced::Alignment::End);
            for b in bars {
                let f = (vf(b, "", 0.0) / top).clamp(0.0, 1.0) as f32;
                br = br.push(container(container(iced::widget::Space::new()).width(Length::Fixed(9.0)).height(Length::Fixed((46.0 * f).max(1.0))).style(ui::boxed(p.accent, 2.0, None))).height(Length::Fixed(46.0)).align_y(iced::Alignment::End).style(ui::boxed(p.panel3, 2.0, None)));
            }
            sc = sc.push(br);
        }
        c = c.push(card(&p, sc));
    }
    if let Some(items) = at(v, "items").as_array() {
        let what = match page {
            "info-modules" => "modules loaded",
            "info-interrupts" => "interrupts",
            _ => "devices",
        };
        c = c.push(field(&p, if page == "info-modules" { "Find a module" } else if page == "info-interrupts" { "Find an interrupt" } else { "Find a device" }, st.txt("find"), |s| Msg::Text("find", s)).width(Length::Fixed(sz(320.0))));
        c = c.push(head(&p, format!("{} {what}{}", items.len(), if vs(v, "note").is_empty() { String::new() } else { format!(" · {}", vs(v, "note")) })));
        let q = st.txt("find").to_lowercase();
        for i in items.iter().filter(|i| q.is_empty() || format!("{} {} {} {}", vs(i, "title"), vs(i, "sub"), vs(i, "kind"), vs(i, "value")).to_lowercase().contains(&q)).take(80) {
            let kind = vs(i, "kind");
            let g = if page == "info-usb" { G::Link } else if page == "info-modules" { G::Grid } else if page == "info-interrupts" { G::Pulse } else if ["VGA", "3D", "Display"].iter().any(|k| kind.contains(k)) { G::Display } else if kind.to_lowercase().contains("audio") { G::Sound } else if kind.contains("Network") || kind.contains("Ethernet") { G::Wifi } else if ["SATA", "NVM", "storage", "Non-Volatile"].iter().any(|k| kind.contains(k)) { G::Drive } else { G::Gear };
            let title = if vs(i, "title").is_empty() { vs(i, "id") } else { vs(i, "title") };
            let sub = [kind, vs(i, "sub"), vs(i, "id")].iter().filter(|x| !x.is_empty()).cloned().collect::<Vec<_>>().join(" · ");
            c = c.push(value_row(&p, g, title, sub, vs(i, "value"), false));
        }
    }
    let text = as_text(v);
    c = c.push(act_row(&p, G::List, "Copy this page", "As text, for a bug report or a forum post", "Copy", Some(Msg::Seq(vec![Msg::App(sv(&["sh", "-c", "printf '%s' \"$1\" | wl-copy", "sh", &text])), Msg::Exec(Ex::none().ok("Copied"))]))));
    c.into()
}

/// Install Hearth: on the live USB only (Hearth's installer, /usr/libexec/nimbus/hearth-install); elsewhere it says so.
fn install(a: &App) -> El<'_> {
    let p = a.sh.p;
    let st = &a.system;
    let live = st.raw("live").trim() == "live";
    let mut c = Column::new().spacing(10.0);
    if live {
        c = c.push(card(&p, column![
            ts("Install Hearth OS on this PC", sz(17.0), p.text),
            t("Next to Windows, SteamOS or any other Linux on the PC (it makes room first), or on a whole drive; then your account, and the install itself.", sz(13.0), p.muted),
            row![btn(&p, "Open the installer", Kind::Primary, false, Some(Msg::App(sv(&["nimbus", "--windowed", "--page", "install"]))))].spacing(8.0)
        ].spacing(10.0)));
    } else {
        c = c.push(card(&p, column![ts("Hearth OS is installed on this PC", sz(16.0), p.text), t("Install Hearth is for the live USB: start this PC from a Hearth USB stick to put Hearth OS on another drive or next to Windows. Make a bootable USB on hearthos.fun.", sz(13.0), p.muted)].spacing(6.0)));
    }
    c.into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert_eq!(version_words("1.14.0", "2026-11-02"), "1.14 · November 2, 2026");
        assert_eq!(version_words("1.14.1", ""), "1.14.1");
        assert_eq!(version_words("0.0.0", ""), "");
    }
}
