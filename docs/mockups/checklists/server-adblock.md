# Server mode › Ad blocker (approved Nov 15, 2026)

Mockups: next-update/server-adblock.png
Release: 1.13

- [x] Its tile on the dashboard, and A opens this view — hosts/linux/desktop/shell/hearth-server-mode: "ads = {'id': 'ads', 'title': 'Ad blocker'"
- [x] "Ad blocker", "AdGuard Home · your home Wi-Fi and your Hearth devices over Tailscale · off until you turn it on", On / Off chip — hosts/linux/desktop/shell/AdsDetail.qml: "AdGuard Home · your home Wi-Fi and your Hearth devices over Tailscale · off until you turn it on"
- [x] Today: % of lookups blocked, ads and trackers stopped, average answer time — hosts/linux/desktop/shell/AdsDetail.qml: "\"ads and trackers stopped\""
- [x] Most blocked: the top names with their counts — os/system_files/usr/libexec/nimbus/hearth-server-dns: "top_blocked_domains"
- [x] Your devices: this PC, your Hearth devices (over Tailscale when away), each with "Using it" / "Paused 1 h" and its own switch — os/system_files/usr/libexec/nimbus/hearth-server-dns: "'state': 'Paused %s' % left("
- [x] Everything else on your Wi-Fi: through the router, with its switch — os/system_files/usr/libexec/nimbus/hearth-server-dns: "'Everything else on your Wi-Fi'"
- [x] Never stops the internet: a device goes back to normal lookups by itself within seconds when this PC can't be reached — os/system_files/usr/libexec/nimbus/hearth-dns-client: "def step(s, now=None):"
- [x] A site that breaks: allow it from here or your phone — hosts/linux/desktop/shell/AdsDetail.qml: "function allow()"
- [x] Ⓐ Turn on or off, Ⓧ Allow a site…, Ⓨ Pause for 1 hour, ☰ Settings, Ⓑ Back to the dashboard — hosts/linux/desktop/shell/AdsDetail.qml: "[\"X\", \"Allow a site…\", \"allow\"]"
- [x] Off until you turn it on — os/system_files/usr/libexec/nimbus/hearth-server-dns: "'ads': {'on': False"
