# Online accounts

Settings › Accounts › Online accounts (1.14): sign in once, and Hearth's apps use the account. Hearth's own version of
KDE's Online Accounts (KAccounts), written from scratch.

| Account | How you sign in | What Hearth can use it for |
|---|---|---|
| Nextcloud | Nextcloud's own sign-in page in your browser (Login Flow v2); Nextcloud gives this device an app password | Calendar, contacts, files |
| Google | Google's sign-in page in your browser (OAuth with PKCE) | Calendar (CalDAV), contacts (CardDAV), Gmail (IMAP) |
| Microsoft | Microsoft's sign-in page in your browser (OAuth with PKCE) | Outlook calendar (Microsoft Graph), mail (IMAP) |
| CalDAV / CardDAV | The server's address, user name and password | Calendar, contacts |
| WebDAV | The address, user name and password | Files |
| IMAP | The mail server, port, user name and password | Mail |

Each account has a switch for each thing it offers (Calendar, Contacts, Files, Mail). Remove signs the device out: the
account goes from the list and its password or token from the keyring.

## Where things are kept

- **The list of accounts** and their switches: `~/.config/nimbus/online-accounts.json`, written only through
  `hearth_settings_io` (atomic, `.bak`, merged under the lock: settings never change by themselves). It never holds a
  password or token.
- **Passwords, app passwords and tokens**: only in the system keyring (the Secret Service: GNOME Keyring, which
  Hearth's session starts), through `secret-tool`, always on its stdin, never on a command line. Items have
  `application=hearth-accounts account=<id>`. With no keyring, adding an account fails and nothing is saved: a
  secret is never written to a file instead.
- **https only**: a password is never sent to a plain `http://` address (only to this device itself, for testing).

## For Hearth's apps: `hearth-accounts`

`/usr/bin/hearth-accounts` (`hosts/linux/desktop/shell/hearth_accounts.py`) answers with one JSON object, or a line
`error: ...`:

```
hearth-accounts list                          accounts, no secrets: id, kind, name, user, can, use
hearth-accounts events [--from 2026-11-16] [--days 31] [--account ID]
                                              events of every account with Calendar on, in the calendar widget's
                                              shape: {title, date, lastDate, allDay, start, end, location, color,
                                              calendar, account, uid, tz, recurring} (start/end in ms; tz is the
                                              event's own zone: "UTC", an IANA name, or "" for floating times and
                                              all-day events; date/lastDate are this device's dates, lastDate
                                              included; repeating events expanded, by the server or here when the
                                              server doesn't, with EXDATEs and moved times)
hearth-accounts calendars [ID]                every calendar (url, name, color, readOnly, account, accountName, kind)
hearth-accounts addressbooks [ID]             every address book (url, name, account, accountName, kind, user)
hearth-accounts cal-objects ID URL [--from D] [--days N]
                                              a calendar's event objects as the server keeps them: {href, etag, ics}
hearth-accounts cal-put ID < {url, ics, etag} an event saved only over that etag (If-Match; no etag: If-None-Match *):
                                              {ok, etag} | {conflict, gone, server: {ics, etag}} | {refused, message}
hearth-accounts cal-delete ID < {url, etag}   an event deleted only over that etag (same answers)
hearth-accounts cal-edit < {ics, uid, change, scope, occ, op}
                                              the event with a change made in it, nothing sent (hearth_ical.py): scope
                                              "all" or "this" (one time of a repeating event: a RECURRENCE-ID copy),
                                              op "save" or "delete-one" (an EXDATE); what Hearth doesn't know about
                                              (attendees, other apps' properties) is kept as the server sent it
hearth-accounts token ID [mail]               a fresh OAuth access token (refreshed from the keyring when needed)
hearth-accounts credentials ID [mail]         {user, password} or {user, token}, for an app's own DAV/IMAP
hearth-accounts mount ID                      the account's files mounted with gvfs: {path}
```

- **The calendar widget** (feature/widgets' `WidgetCalendar.qml` reads `hearth-widgets events`) can add
  `hearth-accounts events` to Hearth Time's events: the shape is the same.
- **Hearth Contacts** (`hosts/hearth-contacts`, src/online.rs) shows the address books of every account with
  Contacts on beside its own CardDAV accounts, and syncs them both ways signed in with what `credentials` gives at
  each sync (held in memory only; a Google token goes as Bearer). An address book that's both one of Contacts' own
  accounts and in Online accounts is shown once: Contacts asks whether to join them (its copy and sync state are
  kept; the password leaves Contacts' keyring item) or keep them apart (remembered). An account removed from Online
  accounts, or with Contacts off, is paused: its copy stays, nothing syncs.
- **Hearth Calendar** (`hosts/hearth-time`, src/accounts.rs) shows the calendars of every account with Calendar on
  and adds, changes and deletes their events on the server. It keeps a copy of each calendar and every change made
  here in `~/.local/share/hearth-calendar/accounts.json` (atomic, `.bak`, changed under its lock), so it works with
  no connection and a change waits until it can be sent (the background service tries every 2 minutes while one
  waits, every 15 otherwise). Each change goes with the etag it was made on: one made on another device in between
  comes back as a conflict, and Calendar shows both versions and lets you choose (keep the other device's, or keep
  yours: made again on the server's version). Repeating events: "this time only" or "every time". Microsoft's
  calendar (Graph, read-only scope) and calendars shared with you to look at are read-only. Google needs the full
  `.../auth/calendar` scope to change events (accounts signed in with the read-only scope can only look until they
  sign in again).
- **Hearth Files** shows each account with Files on in its left menu (`hearth-files places`); opening it mounts
  `davs://user@host/remote.php/dav/files/user/` with gvfs (the image has gvfs, gvfs-backends with its WebDAV backend
  and gvfs-fuse), answering gvfs's password question from the keyring.

## What the owner has to set up for Google and Microsoft

Hearth ships **no client ids or secrets** (`/usr/share/hearth/online-accounts/clients.json` has empty `clientId`s).
Until they're set, the Google and Microsoft cards say "Needs setting up" and explain why. To turn them on, put
the ids in `/etc/hearth/online-accounts.json` (same shape as the shipped file; only the keys you set replace the
shipped ones), or ship that file in the image:

```json
{ "providers": {
    "google":    { "clientId": "<your Google client id>.apps.googleusercontent.com", "clientSecret": "<see below>" },
    "microsoft": { "clientId": "<your Microsoft application (client) id>" } } }
```

**Microsoft** (Entra ID, portal.azure.com › App registrations › New registration):
1. Name it "Hearth OS". Supported accounts: "Accounts in any organizational directory and personal Microsoft accounts".
2. Redirect URI: platform "Public client/native (mobile & desktop)", `http://localhost` (any port is allowed for
   loopback).
3. Authentication › Allow public client flows: Yes. No client secret is needed (PKCE).
4. API permissions (delegated): `offline_access`, `openid`, `email`, `profile`, Microsoft Graph `Calendars.Read`,
   `Contacts.Read`, and Office 365 Exchange Online `IMAP.AccessAsUser.All`.
5. Copy the Application (client) id into `clientId`.

**Google** (console.cloud.google.com):
1. A project, then APIs & Services › Enable: Google Calendar API (CalDAV), Google Contacts CardDAV API (People API).
2. OAuth consent screen: External; scopes `openid`, `email`, `.../auth/calendar` (so Calendar can change events),
   `.../auth/carddav`,
   `https://mail.google.com/` (Gmail is a restricted scope: Google's verification is needed before people outside
   the test users can sign in).
3. Credentials › Create OAuth client id › Application type **Desktop app**. Its redirect is the loopback
   (`http://127.0.0.1:<port>`), which Hearth uses with PKCE.
4. Copy the client id into `clientId`. Google's token endpoint still asks Desktop-app clients for their "client
   secret" even with PKCE (Google documents it as not confidential for installed apps), so put it in
   `clientSecret` in `/etc/hearth/online-accounts.json` on the device or the image, **never in this repo**.

Hearth never signs in to a real account in CI: the tests use fake servers on localhost only.

## Tests

`hosts/linux/test/test_account_calendars.py` (the same workflow): `cal-objects`, `cal-edit`, `cal-put`, `cal-delete`
and `events` against the stateful fake CalDAV server in `hosts/linux/test/fake_dav.py` (conflicts, read-only
calendars, the password only sent to the account's own server). Hearth Time's CI (`time-apps.yml`) runs Calendar's
own code against the same fake server with the real `hearth-accounts`; Hearth Contacts' CI (`contacts.yml`) syncs an
Online accounts address book with a throwaway Radicale.

`hosts/linux/test/test_online_accounts.py` (workflow `online-accounts.yml`, which also draws the page at 1920×1080):
fake CalDAV/CardDAV/WebDAV, Nextcloud Login Flow v2, an OAuth server that checks PKCE, Microsoft Graph and IMAP, all
on 127.0.0.1, with a stand-in `secret-tool` that logs its command lines (a secret must only arrive on its stdin).

## Not yet

- Google Drive and OneDrive files (gvfs's Google backend needs GNOME Online Accounts; OneDrive has no gvfs backend).
- Changing events in Microsoft's calendar (Graph with Calendars.ReadWrite) and Microsoft's contacts (Graph, no CardDAV).
- Needs testing on a device with a real Nextcloud, and with Google/Microsoft once the client ids exist.
