#!/usr/bin/python3
"""CI only (plasma-hearth-shots.sh extras): two apps' tray icons (StatusNotifierItem, registered with KDE's own
StatusNotifierWatcher, or with a stand-in for it when nothing has that name) and a pretend music player (MPRIS) playing
a track with its cover, for pictures of Hearth's panel's tray and what's playing. Runs until it's stopped."""
import os

from gi.repository import Gio, GLib

bus = Gio.bus_get_sync(Gio.BusType.SESSION, None)
items = []


def call(conn, sender, path, iface, method, params, inv):
    print(iface, method, flush=True)
    if method == 'RegisterStatusNotifierItem':
        svc = params.unpack()[0]
        items.append(svc if '/' in svc else svc + '/StatusNotifierItem')
        conn.emit_signal(None, '/StatusNotifierWatcher', 'org.kde.StatusNotifierWatcher', 'StatusNotifierItemRegistered',
                         GLib.Variant('(s)', (items[-1],)))
    inv.return_value(None)


WATCHER = '''<node><interface name="org.kde.StatusNotifierWatcher">
<method name="RegisterStatusNotifierItem"><arg type="s" direction="in"/></method>
<method name="RegisterStatusNotifierHost"><arg type="s" direction="in"/></method>
<property name="RegisteredStatusNotifierItems" type="as" access="read"/>
<property name="IsStatusNotifierHostRegistered" type="b" access="read"/><property name="ProtocolVersion" type="i" access="read"/>
<signal name="StatusNotifierItemRegistered"><arg type="s"/></signal><signal name="StatusNotifierItemUnregistered"><arg type="s"/></signal>
<signal name="StatusNotifierHostRegistered"/></interface></node>'''


def watcher_get(conn, sender, path, iface, prop):
    return {'RegisteredStatusNotifierItems': GLib.Variant('as', items), 'IsStatusNotifierHostRegistered': GLib.Variant('b', True),
            'ProtocolVersion': GLib.Variant('i', 0)}[prop]


has = bus.call_sync('org.freedesktop.DBus', '/org/freedesktop/DBus', 'org.freedesktop.DBus', 'NameHasOwner',
                    GLib.Variant('(s)', ('org.kde.StatusNotifierWatcher',)), None, 0, -1, None).unpack()[0]
if not has:
    bus.register_object('/StatusNotifierWatcher', Gio.DBusNodeInfo.new_for_xml(WATCHER).interfaces[0], call, watcher_get, None)
    Gio.bus_own_name_on_connection(bus, 'org.kde.StatusNotifierWatcher', 0, None, None)
print('watcher', "KDE's" if has else 'stand-in', flush=True)

ITEM = '''<node><interface name="org.kde.StatusNotifierItem">
<method name="Activate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
<method name="SecondaryActivate"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
<method name="ContextMenu"><arg type="i" direction="in"/><arg type="i" direction="in"/></method>
<method name="Scroll"><arg type="i" direction="in"/><arg type="s" direction="in"/></method>
<property name="Category" type="s" access="read"/><property name="Id" type="s" access="read"/><property name="Title" type="s" access="read"/>
<property name="Status" type="s" access="read"/><property name="IconName" type="s" access="read"/>
<property name="ToolTip" type="(sa(iiay)ss)" access="read"/><property name="ItemIsMenu" type="b" access="read"/>
<property name="Menu" type="o" access="read"/><property name="WindowId" type="i" access="read"/>
<signal name="NewIcon"/><signal name="NewStatus"><arg type="s"/></signal></interface></node>'''


def tray_item(path, ident, title, icon):
    props = {'Category': GLib.Variant('s', 'ApplicationStatus'), 'Id': GLib.Variant('s', ident), 'Title': GLib.Variant('s', title),
             'Status': GLib.Variant('s', 'Active'), 'IconName': GLib.Variant('s', icon),
             'ToolTip': GLib.Variant('(sa(iiay)ss)', ('', [], title, 'Running in the background')),
             'ItemIsMenu': GLib.Variant('b', False), 'Menu': GLib.Variant('o', '/NO_DBUSMENU'), 'WindowId': GLib.Variant('i', 0)}
    bus.register_object(path, Gio.DBusNodeInfo.new_for_xml(ITEM).interfaces[0], call, lambda c, s, p, i, prop: props[prop], None)


tray_item('/TrayMail', 'hearth-mail', 'Mail', 'mail-unread')
tray_item('/TraySync', 'hearth-sync', 'Sync', 'folder-cloud')


def register():
    for p in ('/TrayMail', '/TraySync'):
        try:
            bus.call_sync('org.kde.StatusNotifierWatcher', '/StatusNotifierWatcher', 'org.kde.StatusNotifierWatcher',
                          'RegisterStatusNotifierItem', GLib.Variant('(s)', (p,)), None, 0, 5000, None)   # a path: the watcher adds who sent it (as KDE's apps do)
        except GLib.Error as e:
            print('register failed', e.message, flush=True)
    print('tray icons registered', flush=True)
    return False


GLib.timeout_add(1500, register)

PLAYER = '''<node><interface name="org.mpris.MediaPlayer2"><method name="Raise"/><method name="Quit"/>
<property name="Identity" type="s" access="read"/><property name="CanQuit" type="b" access="read"/><property name="CanRaise" type="b" access="read"/>
<property name="HasTrackList" type="b" access="read"/><property name="DesktopEntry" type="s" access="read"/>
<property name="SupportedUriSchemes" type="as" access="read"/><property name="SupportedMimeTypes" type="as" access="read"/></interface>
<interface name="org.mpris.MediaPlayer2.Player"><method name="PlayPause"/><method name="Play"/><method name="Pause"/><method name="Next"/>
<method name="Previous"/><method name="Stop"/><method name="Seek"><arg type="x" direction="in"/></method>
<property name="PlaybackStatus" type="s" access="read"/><property name="Metadata" type="a{sv}" access="read"/>
<property name="CanControl" type="b" access="read"/><property name="CanPlay" type="b" access="read"/><property name="CanPause" type="b" access="read"/>
<property name="CanGoNext" type="b" access="read"/><property name="CanGoPrevious" type="b" access="read"/><property name="CanSeek" type="b" access="read"/>
<property name="Position" type="x" access="read"/><property name="Volume" type="d" access="read"/><property name="Rate" type="d" access="read"/>
</interface></node>'''
walls = sorted(f for f in os.listdir('/usr/share/backgrounds/hearth') if f.endswith('.jpg')) if os.path.isdir('/usr/share/backgrounds/hearth') else []
art = 'file:///usr/share/backgrounds/hearth/' + walls[0] if walls else ''
meta = {'mpris:trackid': GLib.Variant('o', '/hearth/track/1'), 'xesam:title': GLib.Variant('s', 'Embers of Hollow Pines'),
        'xesam:artist': GLib.Variant('as', ['Hearth trailer music']), 'mpris:length': GLib.Variant('x', 184000000)}
if art:
    meta['mpris:artUrl'] = GLib.Variant('s', art)
P = {'Identity': GLib.Variant('s', 'Hearth Music'), 'CanQuit': GLib.Variant('b', False), 'CanRaise': GLib.Variant('b', False),
     'HasTrackList': GLib.Variant('b', False), 'DesktopEntry': GLib.Variant('s', 'hearth-music'), 'SupportedUriSchemes': GLib.Variant('as', []),
     'SupportedMimeTypes': GLib.Variant('as', []), 'PlaybackStatus': GLib.Variant('s', 'Playing'), 'Metadata': GLib.Variant('a{sv}', meta),
     'CanControl': GLib.Variant('b', True), 'CanPlay': GLib.Variant('b', True), 'CanPause': GLib.Variant('b', True),
     'CanGoNext': GLib.Variant('b', True), 'CanGoPrevious': GLib.Variant('b', True), 'CanSeek': GLib.Variant('b', True),
     'Position': GLib.Variant('x', 62000000), 'Volume': GLib.Variant('d', 1.0), 'Rate': GLib.Variant('d', 1.0)}
for iface in Gio.DBusNodeInfo.new_for_xml(PLAYER).interfaces:
    bus.register_object('/org/mpris/MediaPlayer2', iface, call, lambda c, s, p, i, prop: P[prop], None)
Gio.bus_own_name_on_connection(bus, 'org.mpris.MediaPlayer2.hearthmusic', 0, None, None)
GLib.MainLoop().run()
