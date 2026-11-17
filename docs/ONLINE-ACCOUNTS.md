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
                                              calendar, account, uid} (start/end in ms; recurring ones expanded)
hearth-accounts calendars [ID]                every calendar (url, name, color)
hearth-accounts addressbooks [ID]             every address book (for Hearth Contacts' CardDAV)
hearth-accounts token ID [mail]               a fresh OAuth access token (refreshed from the keyring when needed)
hearth-accounts credentials ID [mail]         {user, password} or {user, token}, for an app's own DAV/IMAP
hearth-accounts mount ID                      the account's files mounted with gvfs: {path}
```

- **The calendar widget** (feature/widgets' `WidgetCalendar.qml` reads `hearth-widgets events`) can add
  `hearth-accounts events` to Hearth Time's events: the shape is the same.
- **Hearth Contacts** keeps its own CardDAV accounts today (`hosts/hearth-contacts`); it can list
  `hearth-accounts addressbooks` and take `credentials` instead of asking again.
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
2. OAuth consent screen: External; scopes `openid`, `email`, `.../auth/calendar.readonly`, `.../auth/carddav`,
   `https://mail.google.com/` (Gmail is a restricted scope: Google's verification is needed before people outside
   the test users can sign in).
3. Credentials › Create OAuth client id › Application type **Desktop app**. Its redirect is the loopback
   (`http://127.0.0.1:<port>`), which Hearth uses with PKCE.
4. Copy the client id into `clientId`. Google's token endpoint still asks Desktop-app clients for their "client
   secret" even with PKCE (Google documents it as not confidential for installed apps), so put it in
   `clientSecret` in `/etc/hearth/online-accounts.json` on the device or the image, **never in this repo**.

Hearth never signs in to a real account in CI: the tests use fake servers on localhost only.

## Tests

`hosts/linux/test/test_online_accounts.py` (workflow `online-accounts.yml`, which also draws the page at 1920×1080):
fake CalDAV/CardDAV/WebDAV, Nextcloud Login Flow v2, an OAuth server that checks PKCE, Microsoft Graph and IMAP, all
on 127.0.0.1, with a stand-in `secret-tool` that logs its command lines (a secret must only arrive on its stdin).

## Not yet

- Google Drive and OneDrive files (gvfs's Google backend needs GNOME Online Accounts; OneDrive has no gvfs backend).
- Writing events back (the API reads calendars; Hearth Time keeps its own events).
- Needs testing on a device with a real Nextcloud, and with Google/Microsoft once the client ids exist.
