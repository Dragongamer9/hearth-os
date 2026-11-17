//! Online accounts' calendars in Hearth Calendar (Settings › Accounts › Online accounts, docs/ONLINE-ACCOUNTS.md):
//! shown beside your own events and subscribed calendars, and changed on the server too (CalDAV: Nextcloud, Google,
//! any CalDAV server; Microsoft's calendar can only be looked at).
//!
//! Everything that talks to the server goes through `hearth-accounts` (HEARTH_ACCOUNTS_BIN for the tests), which
//! holds the passwords and tokens: this program never sees one.
//!   calendars                      which calendars there are (and which can be changed)
//!   cal-objects ACCOUNT URL        a calendar's events as the server keeps them: {href, etag, ics}
//!   cal-edit                       an event with a change made in it (nothing is sent)
//!   cal-put / cal-delete ACCOUNT   a change sent, only over the etag it was made on (If-Match)
//!   events --account ACCOUNT       Microsoft's calendar (read-only)
//!
//! paths::accounts_file() keeps a copy of each calendar, so the calendar shows and can be changed with no
//! connection, and every change made here until the server has it. It's written like every other Hearth file
//! (appfile.rs: atomic, FILE.bak, a damaged file put back, changed under its lock), so a change is never lost:
//!   {"calendars": [{id, account, accountName, kind, url, name, color, readOnly, hidden, updated, error}],
//!    "objects": {CAL: [{href, etag, ics, pending?, conflict?}]}, "graph": {CAL: [events]}}
//!   pending   {op: "save"|"delete", edits: [{op, change, scope, occ}], base: the etag the change was made on,
//!              orig: the server's ics then, since: ms}
//!   conflict  {why: "changed"|"gone"|"refused", ics, etag, message, at}: the server had someone else's change (or
//!              didn't take ours). Nothing is overwritten: the person chooses (resolve: "theirs" or "mine").

use crate::calendar::Event;
use serde_json::{json, Value};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub const PREFIX: &str = "acct-";

pub fn program() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HEARTH_ACCOUNTS_BIN").filter(|p| !p.is_empty()) { return Some(p.into()); }
    std::env::var_os("PATH").and_then(|p| std::env::split_paths(&p).map(|d| d.join("hearth-accounts")).find(|f| f.is_file()))
}

/// Runs hearth-accounts (with INPUT as JSON on its stdin): its answer, or what its "error: ..." line says.
fn run(args: &[&str], input: Option<&Value>) -> Result<Value, String> {
    let prog = program().ok_or("Online accounts aren't on this computer.")?;
    let mut child = Command::new(prog).args(args).env_remove("HEARTH_INPUT")
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() }).stdout(Stdio::piped()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Couldn't ask Online accounts ({e})."))?;
    if let (Some(v), Some(mut si)) = (input, child.stdin.take()) { let _ = si.write_all(v.to_string().as_bytes()); }
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout);
    if let Some(e) = text.lines().find_map(|l| l.strip_prefix("error: ")) { return Err(e.trim().to_string()); }
    text.lines().rev().find(|l| l.starts_with('{')).and_then(|l| serde_json::from_str(l).ok())
        .ok_or_else(|| "Online accounts didn't answer.".to_string())
}

fn fill(mut v: Value) -> Value {
    if !v.is_object() { v = json!({}); }
    if !v["calendars"].is_array() { v["calendars"] = json!([]); }
    if !v["objects"].is_object() { v["objects"] = json!({}); }
    if !v["graph"].is_object() { v["graph"] = json!({}); }
    v
}

pub fn load() -> Value { fill(crate::store::load(&crate::paths::accounts_file())) }

/// Changes the latest file under its lock; nothing is written when CHANGE fails.
fn update<T>(change: impl FnOnce(&mut Value) -> Result<T, String>) -> Result<T, String> {
    let p = crate::paths::accounts_file();
    crate::appfile::update(&p, |cur| {
        let mut v = fill(cur.unwrap_or(Value::Null));
        match change(&mut v) { Ok(t) => (Some(v), Ok(t)), Err(e) => (None, Err(e)) }
    }).map_err(|e| format!("Couldn't save {}: {e}", p.display()))?
}

fn hash(s: &str) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
    format!("{h:016x}")
}

/// A calendar's id here: the same every time for the same account and address.
pub fn cal_id(account: &str, url: &str) -> String { format!("{PREFIX}{}", &hash(&format!("{account}\n{}", url.trim_end_matches('/')))[..12]) }

/// An event's id here: its calendar and its UID.
pub fn event_id(cal: &str, uid: &str) -> String { format!("{cal}|{uid}") }

fn split_id(id: &str) -> Option<(&str, &str)> { id.split_once('|').filter(|(c, u)| c.starts_with(PREFIX) && !u.is_empty()) }

fn s(v: &Value, k: &str) -> String { v[k].as_str().unwrap_or("").to_string() }

fn cal_of<'a>(v: &'a Value, id: &str) -> Option<&'a Value> { v["calendars"].as_array()?.iter().find(|c| c["id"] == id) }

/// The calendars whose events may be changed here.
pub fn writable_ids(v: &Value) -> Vec<String> {
    v["calendars"].as_array().into_iter().flatten()
        .filter(|c| c["readOnly"] != true && !s(c, "url").is_empty())
        .map(|c| s(c, "id")).collect()
}

/// The UID of the (first) event in an object.
fn uid_of(ics: &str) -> String {
    crate::ics::parse_with_overrides(ics, "").into_iter().next().map(|e| e.id).unwrap_or_default()
}

/// Every event of every calendar that's shown (changed single times too), as the calendar draws them.
pub fn events(v: &Value) -> Vec<Event> {
    let mut out = Vec::new();
    let mut later = Vec::new();
    for c in v["calendars"].as_array().into_iter().flatten() {
        if c["hidden"] == true { continue; }
        let (id, color) = (s(c, "id"), s(c, "color"));
        for o in v["objects"][id.as_str()].as_array().into_iter().flatten() {
            if o["pending"]["op"] == "delete" { continue; }
            let sync = if o["conflict"].is_object() { "conflict" } else if o["pending"].is_object() { "pending" } else { "" };
            let evs = crate::ics::parse_with_overrides(o["ics"].as_str().unwrap_or(""), &id);
            // A changed single time replaces that time of the series.
            let moved: Vec<(String, String)> = evs.iter().filter_map(|e| e.recurrence_id.clone().map(|r| (e.id.clone(), r))).collect();
            for mut e in evs {
                if e.recurrence_id.is_none() && e.rrule.is_some() {
                    for (u, r) in &moved { if *u == e.id && !e.exdates.contains(r) { e.exdates.push(r.clone()); } }
                }
                e.id = event_id(&id, &e.id);
                e.color = color.clone();
                e.sync = sync.to_string();
                if e.recurrence_id.is_some() { later.push(e) } else { out.push(e) }
            }
        }
        // Microsoft: Graph's events as hearth-accounts gives them (already expanded; in UTC).
        for g in v["graph"][id.as_str()].as_array().into_iter().flatten() {
            let all = g["allDay"] == true;
            let at = |k: &str| chrono::DateTime::from_timestamp_millis(g[k].as_i64().unwrap_or(0)).map(|d| d.naive_utc());
            let (Some(a), Some(b)) = (at("start"), at("end")) else { continue };
            let (start, end) = if all { (s(g, "date"), chrono::NaiveDate::parse_from_str(&s(g, "lastDate"), "%Y-%m-%d").map(|d| (d + chrono::Duration::days(1)).format("%Y-%m-%d").to_string()).unwrap_or_default()) }
                else { (a.format("%Y-%m-%dT%H:%M:%S").to_string(), b.format("%Y-%m-%dT%H:%M:%S").to_string()) };
            out.push(Event { id: event_id(&id, &format!("{}{}", s(g, "uid"), g["start"])), calendar: id.clone(), title: s(g, "title"), location: s(g, "location"),
                all_day: all, start, end, tz: if all { None } else { Some("UTC".into()) }, color: color.clone(), ..Default::default() });
        }
    }
    out.extend(later);
    out
}

// ---------------------------------------------------------------------------------------------- from the servers

/// Gets every calendar again (after sending what's waiting). Returns summary().
pub fn refresh() -> Result<Value, String> {
    if program().is_none() { return Ok(summary(&load())); }
    let _ = flush();
    let got = run(&["calendars"], None)?;
    let from = (chrono::Local::now().date_naive() - chrono::Duration::days(62)).format("%Y-%m-%d").to_string();
    let mut fetched: Vec<(Value, Result<Value, String>)> = Vec::new();
    for c in got["calendars"].as_array().into_iter().flatten() {
        let (account, url) = (s(c, "account"), s(c, "url"));
        let r = if url.is_empty() {
            run(&["events", "--account", account.as_str(), "--from", from.as_str(), "--days", "500"], None)
        } else {
            run(&["cal-objects", account.as_str(), url.as_str(), "--from", from.as_str(), "--days", "500"], None)
        };
        fetched.push((c.clone(), r));
    }
    let unreachable: Vec<String> = got["errors"].as_array().into_iter().flatten().map(|e| s(e, "account")).collect();
    let account_errors: Vec<(String, String)> = got["errors"].as_array().into_iter().flatten().map(|e| (s(e, "account"), s(e, "error"))).collect();
    let now = crate::now_ms();
    update(|v| {
        let old = v["calendars"].as_array().cloned().unwrap_or_default();
        let mut cals = Vec::new();
        for (c, r) in &fetched {
            let (account, url) = (s(c, "account"), s(c, "url"));
            let id = cal_id(&account, if url.is_empty() { &account } else { &url });
            let was = old.iter().find(|o| o["id"] == id.as_str());
            let mut cal = json!({"id": id, "account": account, "accountName": s(c, "accountName"), "kind": s(c, "kind"), "url": url,
                "name": s(c, "name"), "color": if s(c, "color").is_empty() { crate::calendar::SUB_COLORS[cals.len() % 6].to_string() } else { s(c, "color") },
                "readOnly": c["readOnly"] == true || url.is_empty(), "hidden": was.map(|w| w["hidden"] == true).unwrap_or(false),
                "updated": was.map(|w| w["updated"].clone()).unwrap_or(json!(0)), "error": ""});
            match r {
                Ok(res) if url.is_empty() => { v["graph"][id.as_str()] = res["events"].clone(); cal["updated"] = json!(now); }
                Ok(res) => {
                    merge_objects(v, &id, res["objects"].as_array().cloned().unwrap_or_default());
                    cal["updated"] = json!(now);
                }
                Err(e) => cal["error"] = json!(e),
            }
            cals.push(cal);
        }
        // Calendars no longer offered: kept while they have changes not sent (or the account just can't be
        // reached), dropped otherwise.
        for o in &old {
            let id = s(o, "id");
            if cals.iter().any(|c| c["id"] == id.as_str()) { continue; }
            let waiting = v["objects"][id.as_str()].as_array().map(|l| l.iter().filter(|x| x["pending"].is_object() || x["conflict"].is_object()).count()).unwrap_or(0);
            let account = s(o, "account");
            if unreachable.contains(&account) {
                let mut k = o.clone();
                k["error"] = json!(account_errors.iter().find(|(a, _)| *a == account).map(|(_, e)| e.clone()).unwrap_or_default());
                cals.push(k);
            } else if waiting > 0 {
                let mut k = o.clone();
                k["error"] = json!(format!("This calendar isn't in Online accounts any more: {waiting} change(s) made here weren't sent."));
                k["readOnly"] = json!(true);
                cals.push(k);
            } else if let Some(m) = v["objects"].as_object_mut() { m.remove(&id); if let Some(g) = v["graph"].as_object_mut() { g.remove(&id); } }
        }
        v["calendars"] = json!(cals);
        Ok(())
    })?;
    Ok(summary(&load()))
}

/// The server's objects for a calendar, keeping every object that has a change not sent yet or a conflict.
fn merge_objects(v: &mut Value, cal: &str, server: Vec<Value>) {
    let old = v["objects"][cal].as_array().cloned().unwrap_or_default();
    let mut out: Vec<Value> = old.iter().filter(|o| o["pending"].is_object() || o["conflict"].is_object()).cloned().collect();
    for o in server {
        if out.iter().any(|k| k["href"] == o["href"]) { continue; }
        out.push(json!({"href": o["href"], "etag": o["etag"], "ics": o["ics"]}));
    }
    v["objects"][cal] = json!(out);
}

// ---------------------------------------------------------------------------------------------- to the servers

#[derive(Default)]
struct Sent { sent: usize, conflicts: usize, waiting: usize, errors: Vec<(String, String)> }

/// Sends every change made here that the server doesn't have yet (not those waiting for the person to sort out a
/// conflict). Each goes with the etag it was made on: a change made elsewhere in the meantime comes back as a
/// conflict, never overwritten. Network trouble leaves the change waiting.
fn flush() -> Result<Sent, String> {
    let snap = load();
    let mut rep = Sent::default();
    for c in snap["calendars"].as_array().into_iter().flatten() {
        let (id, account) = (s(c, "id"), s(c, "account"));
        for o in snap["objects"][id.as_str()].as_array().into_iter().flatten() {
            let p = &o["pending"];
            if !p.is_object() || o["conflict"].is_object() { continue; }
            let (href, base, since) = (s(o, "href"), s(p, "base"), p["since"].clone());
            let r = if p["op"] == "delete" {
                if base.is_empty() { Ok(json!({"ok": true})) } else { run(&["cal-delete", account.as_str()], Some(&json!({"url": href, "etag": base}))) }
            } else {
                run(&["cal-put", account.as_str()], Some(&json!({"url": href, "ics": o["ics"], "etag": base})))
            };
            let deleting = p["op"] == "delete";
            match &r {
                Ok(x) if x["ok"] == true => rep.sent += 1,
                Ok(x) if x["conflict"] == true || x["refused"] == true => rep.conflicts += 1,
                Ok(_) => rep.waiting += 1,
                Err(e) => { rep.waiting += 1; rep.errors.push((id.clone(), e.clone())); }
            }
            let now = crate::now_ms();
            update(|v| {
                let Some(list) = v["objects"][id.as_str()].as_array_mut() else { return Ok(()) };
                let Some(i) = list.iter().position(|x| x["href"] == href.as_str()) else { return Ok(()) };
                let same = list[i]["pending"]["since"] == since;
                match &r {
                    Ok(x) if x["ok"] == true => {
                        if deleting && same { list.remove(i); return Ok(()); }
                        let new_etag = s(x, "etag");
                        if same {
                            list[i]["etag"] = json!(new_etag);
                            if !s(x, "ics").is_empty() { list[i]["ics"] = x["ics"].clone(); }
                            list[i]["pending"] = Value::Null;
                        } else {
                            // Changed again here while this was being sent: that change goes next, over the new etag.
                            list[i]["etag"] = json!(new_etag.clone());
                            list[i]["pending"]["base"] = json!(new_etag);
                        }
                    }
                    Ok(x) if x["conflict"] == true => {
                        let theirs = &x["server"];
                        list[i]["conflict"] = json!({"why": if x["gone"] == true { "gone" } else { "changed" }, "ics": theirs["ics"], "etag": theirs["etag"], "at": now});
                    }
                    Ok(x) if x["refused"] == true => {
                        list[i]["conflict"] = json!({"why": "refused", "message": x["message"], "at": now});
                    }
                    _ => {}
                }
                Ok(())
            })?;
        }
    }
    let errors = rep.errors.clone();
    update(|v| {
        for c in v["calendars"].as_array_mut().into_iter().flatten() {
            if let Some((_, e)) = errors.iter().find(|(i, _)| c["id"] == i.as_str()) { c["error"] = json!(format!("Changes made here are waiting to be sent: {e}")); }
        }
        Ok(())
    })?;
    Ok(rep)
}

/// An object's change: applies EDIT ({op, change, scope, occ}) to ICS with hearth-accounts cal-edit.
fn apply(ics: &str, uid: &str, edit: &Value) -> Result<(String, String), String> {
    let r = run(&["cal-edit"], Some(&json!({"ics": ics, "uid": uid, "change": edit["change"], "scope": edit["scope"], "occ": edit["occ"], "op": edit["op"]})))?;
    Ok((s(&r, "ics"), s(&r, "uid")))
}

/// What became of one event's change: "sent", "waiting" (kept here until the server can be reached), "conflict".
fn status_of(cal: &str, href: &str) -> Value {
    let v = load();
    let o = v["objects"][cal].as_array().and_then(|l| l.iter().find(|x| x["href"] == href)).cloned();
    let err = cal_of(&v, cal).map(|c| s(c, "error")).unwrap_or_default();
    match o {
        None => json!({"status": "sent"}),
        Some(o) if o["conflict"].is_object() => json!({"status": "conflict", "conflict": conflict_view(&v, cal, &o)}),
        Some(o) if o["pending"].is_object() => json!({"status": "waiting", "error": err}),
        Some(_) => json!({"status": "sent"}),
    }
}

fn file_name(uid: &str) -> String {
    let clean: String = uid.chars().map(|c| if c.is_ascii_alphanumeric() || "-_.@".contains(c) { c } else { '_' }).collect();
    format!("{}-{}.ics", clean.trim_start_matches('.').chars().take(60).collect::<String>(), &hash(uid)[..6])
}

/// Saves an event in an Online accounts calendar (EVENT.calendar): kept here first, then sent. SCOPE "this" with
/// OCC changes only that time of a repeating event. Returns {id, status, ...} (status_of).
pub fn save(event: &Event, scope: &str, occ: Option<&str>) -> Result<Value, String> {
    let v = load();
    let cal = if let Some((c, _)) = split_id(&event.id) { c.to_string() } else { event.calendar.clone() };
    let c = cal_of(&v, &cal).ok_or("That calendar isn't here any more.")?;
    if !writable_ids(&v).contains(&cal) { return Err(format!("{} can only be looked at here.", s(c, "name"))); }
    if crate::rrule::parse_when(&event.start).is_none() { return Err("The event needs a start date.".into()); }
    if let (Some(a), Some(b)) = (crate::rrule::parse_when(&event.start), crate::rrule::parse_when(&event.end)) { if b < a { return Err("The event ends before it starts.".into()); } }
    let uid = split_id(&event.id).map(|(_, u)| u.to_string()).unwrap_or_default();
    let mut change = json!({"title": event.title.trim(), "start": event.start, "end": event.end, "allDay": event.all_day, "location": event.location,
        "notes": event.notes, "rrule": event.rrule.clone().unwrap_or_default(), "reminders": event.reminders});
    if let Some(z) = event.tz.as_deref().filter(|z| !z.is_empty()) { change["tz"] = json!(z); }
    let edit = json!({"op": "save", "change": change, "scope": if scope == "this" { "this" } else { "all" }, "occ": occ.unwrap_or("")});
    let found = if uid.is_empty() { None } else {
        v["objects"][cal.as_str()].as_array().and_then(|l| l.iter().find(|o| uid_of(o["ics"].as_str().unwrap_or("")) == uid)).cloned()
    };
    if found.as_ref().is_some_and(|o| o["conflict"].is_object()) { return Err("This event was changed on another device too: choose which to keep first.".into()); }
    if !uid.is_empty() && found.is_none() { return Err("That event isn't in the calendar any more.".into()); }
    let base_ics = found.as_ref().map(|o| s(o, "ics")).unwrap_or_default();
    let (ics, new_uid) = apply(&base_ics, &uid, &edit)?;
    let href = found.as_ref().map(|o| s(o, "href")).unwrap_or_else(|| format!("{}/{}", s(c, "url").trim_end_matches('/'), file_name(&new_uid)));
    put_pending(&cal, &href, found.is_none(), &ics, edit)?;
    let _ = flush();
    let mut st = status_of(&cal, &href);
    st["id"] = json!(event_id(&cal, &new_uid));
    Ok(st)
}

/// Keeps a change here (before anything is sent), on top of any change still waiting.
fn put_pending(cal: &str, href: &str, new: bool, ics: &str, edit: Value) -> Result<(), String> {
    let now = crate::now_ms();
    let (href, ics) = (href.to_string(), ics.to_string());
    update(|v| {
        if !v["objects"][cal].is_array() { v["objects"][cal] = json!([]); }
        let list = v["objects"][cal].as_array_mut().ok_or("damaged file")?;
        let i = match list.iter().position(|x| x["href"] == href.as_str()) {
            Some(i) => i,
            None if new => { list.push(json!({"href": href, "etag": "", "ics": ""})); list.len() - 1 }
            None => return Err("That event isn't in the calendar any more.".into()),
        };
        let o = &mut list[i];
        if o["conflict"].is_object() { return Err("This event was changed on another device too: choose which to keep first.".into()); }
        if !o["pending"].is_object() { o["pending"] = json!({"op": "save", "edits": [], "base": o["etag"], "orig": o["ics"]}); }
        let deleting = edit["op"] == "delete";
        if deleting { o["pending"]["op"] = json!("delete"); } else {
            o["pending"]["op"] = json!("save");
            o["ics"] = json!(ics);
            if let Some(e) = o["pending"]["edits"].as_array_mut() { e.push(edit); }
        }
        o["pending"]["since"] = json!(now);
        Ok(())
    })
}

/// Deletes an event (the whole series), or with OCC only that time of a repeating one.
pub fn delete(id: &str, occ: Option<&str>) -> Result<Value, String> {
    let (cal, uid) = split_id(id).ok_or("That event isn't in an Online accounts calendar.")?;
    let v = load();
    if !writable_ids(&v).contains(&cal.to_string()) { return Err("This calendar can only be looked at here.".into()); }
    let o = v["objects"][cal].as_array().and_then(|l| l.iter().find(|o| uid_of(o["ics"].as_str().unwrap_or("")) == uid)).cloned()
        .ok_or("That event isn't in the calendar any more.")?;
    if o["conflict"].is_object() { return Err("This event was changed on another device too: choose which to keep first.".into()); }
    let href = s(&o, "href");
    let repeats = crate::ics::parse(o["ics"].as_str().unwrap_or(""), cal).iter().any(|e| e.rrule.is_some());
    match occ.filter(|x| !x.is_empty()) {
        Some(x) if repeats => {
            let edit = json!({"op": "delete-one", "change": {}, "scope": "this", "occ": x});
            let (ics, _) = apply(o["ics"].as_str().unwrap_or(""), uid, &edit)?;
            put_pending(cal, &href, false, &ics, edit)?;
        }
        _ => {
            // Never on the server (made here, not sent yet): it just goes.
            let never_sent = s(&o, "etag").is_empty() && o["pending"].is_object() && s(&o["pending"], "base").is_empty();
            if never_sent {
                update(|v| { if let Some(l) = v["objects"][cal].as_array_mut() { l.retain(|x| x["href"] != href.as_str()); } Ok(()) })?;
                return Ok(json!({"status": "sent"}));
            }
            put_pending(cal, &href, false, "", json!({"op": "delete"}))?;
        }
    }
    let _ = flush();
    Ok(status_of(cal, &href))
}

/// The person chose after a conflict: KEEP "theirs" (the server's version stays, the change made here goes) or
/// "mine" (the change made here is made again on the server's version, and sent).
pub fn resolve(id: &str, keep: &str) -> Result<Value, String> {
    let (cal, uid) = split_id(id).ok_or("That event isn't in an Online accounts calendar.")?;
    let v = load();
    let o = v["objects"][cal].as_array().and_then(|l| l.iter().find(|o| o["conflict"].is_object() && (uid_of(o["ics"].as_str().unwrap_or("")) == uid || uid_of(&s(&o["pending"], "orig")) == uid))).cloned()
        .ok_or("There's nothing to choose for that event any more.")?;
    let href = s(&o, "href");
    let k = &o["conflict"];
    let why = s(k, "why");
    let p = &o["pending"];
    // "mine" on an event changed elsewhere: every change made here, made again on the server's version.
    let remade = if keep == "mine" && why == "changed" && p["op"] != "delete" {
        let mut ics = s(k, "ics");
        for e in p["edits"].as_array().into_iter().flatten() { ics = apply(&ics, uid, e)?.0; }
        Some(ics)
    } else { None };
    update(|v| {
        let Some(list) = v["objects"][cal].as_array_mut() else { return Ok(()) };
        let Some(i) = list.iter().position(|x| x["href"] == href.as_str()) else { return Ok(()) };
        let now = crate::now_ms();
        let o = &mut list[i];
        let k = o["conflict"].clone();
        match (keep, why.as_str()) {
            ("theirs", "changed") => { o["ics"] = k["ics"].clone(); o["etag"] = k["etag"].clone(); o["pending"] = Value::Null; o["conflict"] = Value::Null; }
            ("theirs", "gone") => { list.remove(i); }
            ("theirs", _) => {
                // Refused: back to what the server has (an event made here that it didn't take just goes).
                if s(&o["pending"], "orig").is_empty() { list.remove(i); } else { o["ics"] = o["pending"]["orig"].clone(); o["pending"] = Value::Null; o["conflict"] = Value::Null; }
            }
            ("mine", "changed") => {
                if let Some(ics) = &remade { o["ics"] = json!(ics); }
                o["pending"]["base"] = k["etag"].clone();
                o["pending"]["orig"] = k["ics"].clone();
                o["etag"] = k["etag"].clone();
                o["pending"]["since"] = json!(now);
                o["conflict"] = Value::Null;
            }
            ("mine", "gone") => {
                if o["pending"]["op"] == "delete" { list.remove(i); } else {
                    // Deleted on the other device; put back as this computer has it (only where there's none now).
                    o["pending"]["base"] = json!("");
                    o["etag"] = json!("");
                    o["pending"]["since"] = json!(now);
                    o["conflict"] = Value::Null;
                }
            }
            ("mine", _) => { o["pending"]["since"] = json!(now); o["conflict"] = Value::Null; }
            _ => return Err("Choose \"theirs\" or \"mine\".".into()),
        }
        Ok(())
    })?;
    let _ = flush();
    let mut st = status_of(cal, &href);
    st["id"] = json!(id);
    Ok(st)
}

/// Shows or hides an Online accounts calendar (only this computer's view of it).
pub fn set_hidden(id: &str, hidden: bool) -> Result<(), String> {
    update(|v| {
        if let Some(c) = v["calendars"].as_array_mut().and_then(|l| l.iter_mut().find(|c| c["id"] == id)) { c["hidden"] = json!(hidden); }
        Ok(())
    })
}

// ---------------------------------------------------------------------------------------------- for the page

/// One event of an object for the conflict dialog: title, when, where.
fn describe(ics: &str, uid: &str) -> Value {
    let ev = crate::ics::parse(ics, "");
    match ev.iter().find(|e| e.id == uid).or(ev.first()) {
        Some(e) => json!({"title": e.title, "start": e.start, "end": e.end, "allDay": e.all_day, "location": e.location, "tz": e.tz, "rrule": e.rrule}),
        None => Value::Null,
    }
}

fn conflict_view(v: &Value, cal: &str, o: &Value) -> Value {
    let k = &o["conflict"];
    let mine_ics = s(o, "ics");
    let uid = { let u = uid_of(&mine_ics); if u.is_empty() { uid_of(&s(&o["pending"], "orig")) } else { u } };
    let c = cal_of(v, cal);
    json!({"id": event_id(cal, &uid), "calendar": cal, "calendarName": c.map(|c| s(c, "name")).unwrap_or_default(),
        "accountName": c.map(|c| s(c, "accountName")).unwrap_or_default(), "why": k["why"], "message": k["message"], "at": k["at"],
        "deleting": o["pending"]["op"] == "delete",
        "mine": if o["pending"]["op"] == "delete" { Value::Null } else { describe(&mine_ics, &uid) },
        "theirs": if k["why"] == "changed" { describe(&s(k, "ics"), &uid) } else { Value::Null }})
}

/// How many changes made here are waiting to be sent.
pub fn waiting(v: &Value) -> usize {
    v["objects"].as_object().map(|m| m.values().flat_map(|l| l.as_array().cloned().unwrap_or_default()).filter(|o| o["pending"].is_object() && !o["conflict"].is_object()).count()).unwrap_or(0)
}

/// The calendars, what's waiting to be sent and the conflicts to sort out, for the Calendar page.
pub fn summary(v: &Value) -> Value {
    let mut conflicts = Vec::new();
    let mut waiting = 0;
    for c in v["calendars"].as_array().into_iter().flatten() {
        let id = s(c, "id");
        for o in v["objects"][id.as_str()].as_array().into_iter().flatten() {
            if o["conflict"].is_object() { conflicts.push(conflict_view(v, &id, o)); } else if o["pending"].is_object() { waiting += 1; }
        }
    }
    json!({"available": program().is_some(), "calendars": v["calendars"], "waiting": waiting, "conflicts": conflicts})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids() {
        assert_eq!(cal_id("dav-1", "https://x.org/cal/"), cal_id("dav-1", "https://x.org/cal"));
        assert_ne!(cal_id("dav-1", "https://x.org/a/"), cal_id("dav-1", "https://x.org/b/"));
        let c = cal_id("dav-1", "https://x.org/a/");
        assert!(c.starts_with(PREFIX));
        assert_eq!(split_id(&event_id(&c, "u@x|y")), Some((c.as_str(), "u@x|y")));
        assert_eq!(split_id("local-event"), None);
        assert!(file_name("../../etc").ends_with(".ics") && !file_name("../../etc").contains('/'));
    }

    #[test]
    fn events_from_the_copy() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:team\r\nDTSTART:20261102T150000Z\r\nDTEND:20261102T160000Z\r\nRRULE:FREQ=WEEKLY\r\nSUMMARY:Team\r\nEND:VEVENT\r\n\
BEGIN:VEVENT\r\nUID:team\r\nRECURRENCE-ID:20261109T150000Z\r\nDTSTART:20261110T170000Z\r\nDTEND:20261110T180000Z\r\nSUMMARY:Team (moved)\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let v = json!({"calendars": [{"id": "acct-1", "url": "https://x.org/c/", "name": "Work", "color": "#123456", "readOnly": false},
                                     {"id": "acct-2", "url": "", "name": "Outlook", "readOnly": true}, {"id": "acct-3", "url": "https://x.org/h/", "hidden": true}],
            "objects": {"acct-1": [{"href": "https://x.org/c/team.ics", "etag": "\"1\"", "ics": ics, "pending": {"op": "save", "since": 1}},
                                   {"href": "https://x.org/c/gone.ics", "etag": "\"2\"", "ics": ics.replace("UID:team", "UID:gone"), "pending": {"op": "delete"}}],
                        "acct-3": [{"href": "h", "etag": "", "ics": ics}]},
            "graph": {"acct-2": [{"title": "Sync", "start": 1_800_000_000_000i64, "end": 1_800_001_800_000i64, "allDay": false, "uid": "g"}]}});
        let ev = events(&v);
        assert_eq!(ev.len(), 3, "{ev:?}");
        assert_eq!(ev[0].id, "acct-1|team");
        assert_eq!(ev[0].exdates, vec!["2026-11-09T15:00:00".to_string()], "the moved time is left out of the series");
        assert_eq!((ev[0].sync.as_str(), ev[0].color.as_str()), ("pending", "#123456"));
        assert_eq!(ev[1].title, "Sync");
        assert_eq!(ev[1].tz.as_deref(), Some("UTC"));
        assert_eq!(ev[2].recurrence_id.as_deref(), Some("2026-11-09T15:00:00"));
        assert_eq!(writable_ids(&v), vec!["acct-1".to_string(), "acct-3".to_string()]);
        let o = crate::calendar::occurrences(&ev, 1_793_000_000_000, 1_795_000_000_000, &|_| false, &|e| e.color.clone());
        let moved = o.iter().find(|x| x.title == "Team (moved)").expect("moved time");
        assert_eq!(moved.occ, "2026-11-09T15:00:00");
        assert!(moved.recurring);
        assert!(!o.iter().any(|x| x.title == "Team" && x.occ == "2026-11-09T15:00:00"));
    }

    /// Against fake_dav.py (hosts/linux/test) with the real hearth-accounts: CI starts both and sets
    /// HEARTH_TIME_DAV_TEST=1, HEARTH_TIME_HOME, HEARTH_ACCOUNTS_BIN and the fake server's control address. It
    /// leaves a conflict behind on purpose, for the screenshots.
    #[test]
    fn against_a_fake_caldav_server() {
        if std::env::var("HEARTH_TIME_DAV_TEST").ok().as_deref() != Some("1") { eprintln!("HEARTH_TIME_DAV_TEST isn't 1: skipped"); return; }
        let control = std::env::var("HEARTH_TIME_DAV_CONTROL").expect("HEARTH_TIME_DAV_CONTROL");
        let sum = refresh().expect("refresh");
        eprintln!("calendars: {}", sum["calendars"]);
        let cals = sum["calendars"].as_array().unwrap();
        assert_eq!(cals.len(), 2);
        let personal = cals.iter().find(|c| c["name"] == "Personal").unwrap();
        let holidays = cals.iter().find(|c| c["name"] == "Holidays").unwrap();
        assert_eq!((personal["readOnly"] == true, holidays["readOnly"] == true), (false, true));
        let pid = s(personal, "id");
        let all = events(&load());
        let names: Vec<&str> = all.iter().map(|e| e.title.as_str()).collect();
        for n in ["Dentist", "Team meeting", "Team meeting (moved)", "Lisbon trip", "Bank holiday"] { assert!(names.contains(&n), "{n} missing: {names:?}"); }
        let dentist = all.iter().find(|e| e.title == "Dentist").unwrap().clone();
        assert_eq!(dentist.tz.as_deref(), Some("America/New_York"));

        // 1. Changed here and sent (If-Match).
        let mut e = dentist.clone();
        e.title = "Dentist (check-up)".into();
        let r = save(&e, "all", None).unwrap();
        assert_eq!(r["status"], "sent", "{r}");
        // 2. A new event in the account's calendar.
        let today = chrono::Local::now().date_naive();
        let new = Event { calendar: pid.clone(), title: "Haircut".into(), start: format!("{}T16:00:00", today + chrono::Duration::days(2)),
            end: format!("{}T16:45:00", today + chrono::Duration::days(2)), reminders: vec![15], ..Default::default() };
        let r = save(&new, "all", None).unwrap();
        assert_eq!(r["status"], "sent", "{r}");
        let haircut_id = s(&r, "id");
        // 3. One time of the repeating meeting moved, then one deleted.
        let team = all.iter().find(|e| e.title == "Team meeting").unwrap().clone();
        let occ = crate::calendar::occurrences(&[team.clone()], crate::now_ms() - 3 * 86_400_000, crate::now_ms() + 30 * 86_400_000, &|_| false, &|_| String::new());
        let first = occ.first().expect("a coming meeting").occ.clone();
        let mut one = team.clone();
        one.title = "Team meeting (this one only)".into();
        // That time's own start and end (the page sends them from the time shown).
        one.start = first.clone();
        one.end = first.replace("T15:00:00", "T16:00:00");
        let r = save(&one, "this", Some(first.as_str())).unwrap();
        assert_eq!(r["status"], "sent", "{r}");
        let r = delete(&team.id, Some(occ[1].occ.as_str())).unwrap();
        assert_eq!(r["status"], "sent", "{r}");
        refresh().unwrap();
        let all = events(&load());
        assert!(all.iter().any(|e| e.title == "Team meeting (this one only)" && e.recurrence_id.as_deref() == Some(first.as_str())));
        assert!(all.iter().any(|e| e.title == "Dentist (check-up)"));
        assert!(all.iter().any(|e| e.title == "Haircut"));
        // 4. Offline: the change is kept here (and in the .bak), and sent when the server can be reached again.
        let real = std::env::var("HEARTH_ACCOUNTS_BIN").unwrap();
        let dead = std::env::temp_dir().join(format!("hearth-accounts-offline-{}", std::process::id()));
        std::fs::write(&dead, format!("#!/bin/sh\ncase \"$1\" in cal-edit) exec \"{real}\" \"$@\" ;; esac\necho 'error: Couldn'\"'\"'t reach 127.0.0.1 (no connection).'\n")).unwrap();
        { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&dead, std::fs::Permissions::from_mode(0o755)).unwrap(); }
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &dead);
        let mut h = all.iter().find(|e| e.title == "Haircut").unwrap().clone();
        h.location = "Barber on 5th".into();
        let r = save(&h, "all", None).unwrap();
        assert_eq!(r["status"], "waiting", "{r}");
        assert!(r["error"].as_str().unwrap_or("").contains("reach"), "{r}");
        let _ = refresh();
        assert_eq!(summary(&load())["waiting"], 1, "a refresh with no connection kept the change");
        let bak = std::fs::read_to_string(crate::appfile::bak_path(&crate::paths::accounts_file())).unwrap_or_default();
        assert!(bak.contains("Barber on 5th") || std::fs::read_to_string(crate::paths::accounts_file()).unwrap().contains("Barber on 5th"));
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &real);
        refresh().unwrap();
        assert_eq!(summary(&load())["waiting"], 0);
        assert!(events(&load()).iter().any(|e| e.location == "Barber on 5th"));
        // 5. Changed on another device while a change here waited: a conflict, nothing overwritten.
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &dead);
        let mut h2 = events(&load()).into_iter().find(|e| e.id == haircut_id).unwrap();
        h2.title = "Haircut with Sam".into();
        assert_eq!(save(&h2, "all", None).unwrap()["status"], "waiting");
        let href = load()["objects"][pid.as_str()].as_array().unwrap().iter().find(|o| o["pending"].is_object()).map(|o| s(o, "href")).unwrap();
        let path = url_path(&href);
        let touch = format!("{control}/_control/touch?href={path}&summary=Haircut%20(moved%20by%20phone)");
        let out = Command::new("curl").args(["-sS", "-X", "POST", touch.as_str()]).output().unwrap();
        assert!(out.status.success(), "control: {:?}", out);
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &real);
        refresh().unwrap();
        let sum = summary(&load());
        let k = &sum["conflicts"][0];
        assert_eq!(k["why"], "changed", "{sum}");
        assert_eq!(k["mine"]["title"], "Haircut with Sam");
        assert_eq!(k["theirs"]["title"], "Haircut (moved by phone)");
        assert!(events(&load()).iter().any(|e| e.id == haircut_id && e.sync == "conflict"));
        // Keeping mine: made again on the server's version and sent.
        let r = resolve(&haircut_id, "mine").unwrap();
        assert_eq!(r["status"], "sent", "{r}");
        refresh().unwrap();
        assert!(events(&load()).iter().any(|e| e.title == "Haircut with Sam" && e.location == "Barber on 5th"));
        // 6. The same again with the meeting, left for the screenshots (and "theirs" shown in the dialog).
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &dead);
        let mut d = events(&load()).into_iter().find(|e| e.title == "Dentist (check-up)").unwrap();
        d.title = "Dentist, moved to the afternoon".into();
        d.start = d.start.replace("T10:00", "T15:00");
        d.end = d.end.replace("T11:00", "T16:00");
        assert_eq!(save(&d, "all", None).unwrap()["status"], "waiting");
        let href = load()["objects"][pid.as_str()].as_array().unwrap().iter().find(|o| o["pending"].is_object()).map(|o| s(o, "href")).unwrap();
        let touch = format!("{control}/_control/touch?href={}&summary=Dentist%20(Dr%20Lee%20is%20away)", url_path(&href));
        let out = Command::new("curl").args(["-sS", "-X", "POST", touch.as_str()]).output().unwrap();
        assert!(out.status.success());
        std::env::set_var("HEARTH_ACCOUNTS_BIN", &real);
        refresh().unwrap();
        assert_eq!(summary(&load())["conflicts"].as_array().map(|a| a.len()), Some(1));
        // A read-only calendar refuses changes here already.
        let hol = events(&load()).into_iter().find(|e| e.title == "Bank holiday").unwrap();
        assert!(save(&hol, "all", None).is_err());
        let _ = std::fs::remove_file(&dead);
        eprintln!("sent, a new event, this time only, one time deleted, offline and back, a conflict kept mine, one left: {}", summary(&load())["conflicts"]);
    }

    fn url_path(href: &str) -> String {
        let p = href.splitn(4, '/').nth(3).map(|p| format!("/{p}")).unwrap_or_default();
        p.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~/".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
    }
}
