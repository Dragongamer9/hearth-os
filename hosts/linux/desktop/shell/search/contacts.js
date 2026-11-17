.pragma library
// Search plugin: Hearth Contacts (like KRunner's contacts runner): your contacts from Hearth Contacts
// (~/.local/share/hearth-contacts/books/*/*.vcf, on this computer and the Online accounts' address books it keeps),
// read by hearth-search-sources pim. By name, e-mail, phone or company. Enter copies the e-mail (or the phone number);
// the second row copies the phone number.

function results(qRaw, contacts) {
    var q = String(qRaw || "").trim().toLowerCase();
    if (q.length < 2 || !contacts || !contacts.length) return [];
    var out = [], digits = q.replace(/[^0-9]/g, "");
    for (var i = 0; i < contacts.length && out.length < 4; i++) {
        var c = contacts[i], n = String(c.name || "").toLowerCase(), em = (c.emails || []).join(" ").toLowerCase(), org = String(c.org || "").toLowerCase();
        var ph = (c.phones || []).join(" ").replace(/[^0-9 ]/g, "");
        var s = n.indexOf(q) === 0 ? 3 : n.indexOf(" " + q) >= 0 ? 2 : (n.indexOf(q) >= 0 || em.indexOf(q) >= 0 || org.indexOf(q) >= 0 || (digits.length >= 4 && ph.indexOf(digits) >= 0)) ? 1 : 0;
        if (!s) continue;
        var e = (c.emails || [])[0], p = (c.phones || [])[0];
        var bits = [e, p, c.org].filter(function (x) { return !!x; }).join(" · ");
        out.push({ plugin: "contacts", group: "Contacts", title: c.name || e || p, sub: bits || "Contact", icon: "user", score: 640 + s * 50 - i,
                   action: e || p ? { type: "copy", value: e || p } : { type: "exec", value: ["hearth-contacts"] } });
        if (e && p && out.length < 4)
            out.push({ plugin: "contacts", group: "Contacts", title: "Copy " + (c.name || "their") + "'s phone number", sub: p, icon: "phone", score: 630 + s * 50 - i,
                       action: { type: "copy", value: p } });
    }
    return out;
}
