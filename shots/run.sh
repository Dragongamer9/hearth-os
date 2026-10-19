set -x
dnf5 -y install xorg-x11-server-Xvfb ImageMagick xwd || dnf -y install xorg-x11-server-Xvfb ImageMagick
id tester || useradd -m tester
cat > /tmp/inner.sh <<'IN'
export HOME=/home/tester DISPLAY=:99 LIBGL_ALWAYS_SOFTWARE=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 NO_AT_BRIDGE=1
Xvfb :99 -screen 0 1280x800x24 >/tmp/xvfb.log 2>&1 &
sleep 3
dbus-run-session -- /usr/lib/nimbus/nimbus --windowed > /shots/run.log 2>&1 &
sleep 20
import -window root /shots/hearth-os.png || xwd -root -silent | convert xwd:- /shots/hearth-os.png
IN
chmod 755 /tmp/inner.sh
setpriv --reuid=tester --regid=tester --init-groups bash /tmp/inner.sh
