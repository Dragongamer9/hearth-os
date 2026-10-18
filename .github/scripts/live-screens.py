#!/usr/bin/env python3
"""Boots the Hearth live ISO in QEMU and takes a screenshot every few seconds (through QEMU's monitor),
pressing Enter at a few points to go from the welcome card into the installer. Frames go to ./frames."""
import json, os, socket, subprocess, sys, time

iso, disk = sys.argv[1], sys.argv[2]
os.makedirs('frames', exist_ok=True)
ovmf_code = next(p for p in ['/usr/share/OVMF/OVMF_CODE_4M.fd', '/usr/share/OVMF/OVMF_CODE.fd'] if os.path.exists(p))
ovmf_vars = next(p for p in ['/usr/share/OVMF/OVMF_VARS_4M.fd', '/usr/share/OVMF/OVMF_VARS.fd'] if os.path.exists(p))
subprocess.run(['cp', ovmf_vars, 'vars.fd'], check=True)
qemu = subprocess.Popen([
    'qemu-system-x86_64', '-enable-kvm', '-machine', 'q35', '-cpu', 'host', '-smp', '4', '-m', '8192',
    '-drive', f'if=pflash,format=raw,readonly=on,file={ovmf_code}', '-drive', 'if=pflash,format=raw,file=vars.fd',
    '-device', 'VGA,edid=on,xres=1280,yres=800', '-display', 'none',
    '-drive', f'file={disk},if=virtio,format=qcow2', '-cdrom', iso, '-boot', 'order=d',
    '-netdev', 'user,id=n0', '-device', 'virtio-net-pci,netdev=n0',
    '-device', 'qemu-xhci', '-device', 'usb-tablet', '-device', 'usb-kbd',
    '-qmp', 'unix:qmp.sock,server,nowait', '-serial', 'file:serial.log'])
for _ in range(100):
    if os.path.exists('qmp.sock'):
        break
    time.sleep(0.2)
s = socket.socket(socket.AF_UNIX); s.connect('qmp.sock'); f = s.makefile('rw')
def cmd(name, **args):
    f.write(json.dumps({'execute': name, 'arguments': args} if args else {'execute': name}) + '\n'); f.flush()
    while True:
        r = json.loads(f.readline())
        if 'return' in r or 'error' in r:
            return r
json.loads(f.readline()); cmd('qmp_capabilities')
def key(k):
    cmd('send-key', keys=[{'type': 'qcode', 'data': k}])
def click(x, y):   # absolute (usb-tablet), 0..32767
    ax, ay = int(x / 1280 * 32767), int(y / 800 * 32767)
    cmd('input-send-event', events=[{'type': 'abs', 'data': {'axis': 'x', 'value': ax}}, {'type': 'abs', 'data': {'axis': 'y', 'value': ay}}])
    cmd('input-send-event', events=[{'type': 'btn', 'data': {'down': True, 'button': 'left'}}])
    cmd('input-send-event', events=[{'type': 'btn', 'data': {'down': False, 'button': 'left'}}])

start = time.time()
plan = {int(a): b for a, b in (x.split(':') for x in os.environ.get('PLAN', '').split(',') if x)}
done = set()
n = 0
while time.time() - start < int(os.environ.get('DURATION', '720')):
    t = int(time.time() - start)
    cmd('screendump', filename=os.path.abspath(f'frames/{n:03d}-{t:04d}s.ppm'))
    n += 1
    for at, action in plan.items():
        if t >= at and at not in done:
            done.add(at)
            print(f'{t}s: {action}', flush=True)
            if action == 'enter': key('ret')
            elif action == 'tab': key('tab')
            elif action.startswith('click'): click(*map(int, action[5:].split('x')))
    time.sleep(int(os.environ.get('EVERY', '8')))
qemu.kill()
