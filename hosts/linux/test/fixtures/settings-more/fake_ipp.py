"""A stand-in CUPS for the Desktop Mode test's Ink & toner picture: answers IPP Get-Printer-Attributes on a unix socket
with made-up marker levels (python3 fake_ipp.py SOCKET). Never installed anywhere."""
import os
import socketserver
import struct
import sys

PRINTERS = {
    'HP_LaserJet_M209': [('Black toner', 12, '#000000', 'toner', 15)],
    'Brother_DCP_L2550': [('Cyan', 64, '#00FFFF', 'ink-cartridge', 10), ('Magenta', 8, '#FF00FF', 'ink-cartridge', 10),
                          ('Yellow', 41, '#FFFF00', 'ink-cartridge', 10), ('Black', 87, '#000000', 'ink-cartridge', 10)],
}


def attr(tag, name, value):
    v = struct.pack('>i', value) if isinstance(value, int) else value.encode()
    return struct.pack('>BH', tag, len(name)) + name.encode() + struct.pack('>H', len(v)) + v


def answer(body):
    uri = body.split(b'printer-uri', 1)[-1]
    name = next((n for n in PRINTERS if n.encode() in uri), '')
    out = struct.pack('>BBHI', 1, 1, 0, 1) + b'\x01' + attr(0x47, 'attributes-charset', 'utf-8') + b'\x04'
    out += attr(0x42, 'printer-name', name)
    for key, idx, tag in (('marker-names', 0, 0x42), ('marker-levels', 1, 0x21), ('marker-colors', 2, 0x42),
                          ('marker-types', 3, 0x44), ('marker-low-levels', 4, 0x21)):
        for j, m in enumerate(PRINTERS.get(name, [])):
            out += attr(tag, key if j == 0 else '', m[idx])
    return out + b'\x03'


class H(socketserver.StreamRequestHandler):
    def handle(self):
        head = b''
        while b'\r\n\r\n' not in head:
            c = self.rfile.read(1)
            if not c:
                return
            head += c
        n = 0
        for line in head.split(b'\r\n'):
            if line.lower().startswith(b'content-length:'):
                n = int(line.split(b':')[1])
        body = self.rfile.read(n)
        data = answer(body)
        self.wfile.write(b'HTTP/1.1 200 OK\r\nContent-Type: application/ipp\r\nContent-Length: %d\r\nConnection: close\r\n\r\n' % len(data) + data)


if __name__ == '__main__':
    path = sys.argv[1]
    try:
        os.remove(path)
    except OSError:
        pass
    socketserver.UnixStreamServer(path, H).serve_forever()
