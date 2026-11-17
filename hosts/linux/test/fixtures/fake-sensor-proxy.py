#!/usr/bin/python3
"""A stand-in iio-sensor-proxy for the Desktop Mode test (never installed): net.hadess.SensorProxy on the SESSION bus
with a light sensor, as hearth-screens reads it with HEARTH_SCREENS_SENSOR_BUS=session.

  fake-sensor-proxy.py [LUX]        LightLevel in lux (default 120); ClaimLight / ReleaseLight are counted
"""
import sys

import gi
gi.require_version('Gio', '2.0')
from gi.repository import Gio, GLib  # noqa: E402

XML = '''<node><interface name="net.hadess.SensorProxy">
  <method name="ClaimLight"/><method name="ReleaseLight"/>
  <property name="HasAmbientLight" type="b" access="read"/>
  <property name="LightLevelUnit" type="s" access="read"/>
  <property name="LightLevel" type="d" access="read"/>
</interface></node>'''
LUX = float(sys.argv[1]) if len(sys.argv) > 1 else 120.0
claims = {'n': 0}


def call(conn, sender, path, iface, method, params, inv):
    if method == 'ClaimLight':
        claims['n'] += 1
    elif method == 'ReleaseLight':
        claims['n'] = max(0, claims['n'] - 1)
    print('%s (claims %d)' % (method, claims['n']), flush=True)
    inv.return_value(None)


def prop(conn, sender, path, iface, name):
    return {'HasAmbientLight': GLib.Variant('b', True), 'LightLevelUnit': GLib.Variant('s', 'lux'),
            'LightLevel': GLib.Variant('d', LUX)}[name]


def acquired(conn, name):
    info = Gio.DBusNodeInfo.new_for_xml(XML)
    conn.register_object('/net/hadess/SensorProxy', info.interfaces[0], call, prop, None)


def got(conn, name):
    print('fake iio-sensor-proxy: %s, %g lux' % (name, LUX), flush=True)


Gio.bus_own_name(Gio.BusType.SESSION, 'net.hadess.SensorProxy', Gio.BusNameOwnerFlags.NONE, acquired, got,
                 lambda c, n: sys.exit('fake iio-sensor-proxy: the name was taken'))
GLib.MainLoop().run()
