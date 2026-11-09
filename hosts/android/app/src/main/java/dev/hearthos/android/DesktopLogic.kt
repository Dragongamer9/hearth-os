package dev.hearthos.android

import java.io.ByteArrayOutputStream
import java.io.EOFException
import java.io.IOException
import java.io.InputStream
import java.security.MessageDigest
import java.security.SecureRandom
import java.util.Base64
import java.util.Locale

/*
 * Hearth Linux stage 2 (the Linux desktop inside Hearth), the parts without Android in them so the unit tests run on
 * a plain JVM: the WebSocket protocol (RFC 6455) noVNC speaks, VNC's password file (DES, as `vncpasswd -f` writes it),
 * what the desktop needs (packages, its size, the script that starts it) and apt's progress lines. WsBridge.kt is the
 * WebSocket ↔ VNC bridge, LinuxEnv.kt puts them together. See docs/ANDROID-LINUX.md.
 */

// ==================================================================== the desktop
object DesktopPlan {
    /** One apt job installs what the desktop needs: XFCE, a terminal in it, D-Bus for the session, TigerVNC's X
     *  server (Xvnc, for the desktop inside Hearth) and the X keyboard data. */
    val PACKAGES = listOf("xfce4", "xfce4-terminal", "dbus-x11", "tigervnc-standalone-server", "xkb-data")

    /** What must be in the Linux for the desktop to start (path in the rootfs → the package that has it). */
    val NEEDS = listOf("usr/bin/startxfce4" to "xfce4", "usr/bin/dbus-launch" to "dbus-x11", "usr/bin/Xvnc" to "tigervnc-standalone-server")

    /** The X display Xvnc runs as inside the Linux (Termux:X11 is :0). */
    const val DISPLAY = 1

    /** The session itself, the same for both ways of showing it. */
    const val SESSION = "dbus-launch --exit-with-session startxfce4"

    /** Where the VNC password file goes, inside the Linux (and under the rootfs on Android's side). */
    const val PASSWD = "home/${LinuxSources.USER}/.vnc/hearth-passwd"

    val MODES = setOf("hearth", "x11")

    fun mode(raw: String?): String {
        val m = (raw ?: "").trim().lowercase(Locale.US)
        if (m !in MODES) throw IllegalArgumentException("Unknown way to show the desktop: '${m.take(20)}' (hearth or x11)")
        return m
    }

    /**
     * The desktop's size from the screen's size in device pixels: about 1.5 device pixels per desktop pixel (so text
     * is readable on a handheld: 1920×1200 → 1280×800), the long side kept between 1024 and 1920 when the screen
     * allows. Width a multiple of 8, height of 2 (what X servers like).
     */
    fun geometry(w: Int, h: Int): Pair<Int, Int> {
        if (w !in 200..16384 || h !in 200..16384) throw IllegalArgumentException("That screen size doesn't look right (${w}×${h})")
        val long = maxOf(w, h).toDouble()
        var scale = 1.5
        if (long / scale < 1024) scale = maxOf(1.0, long / 1024)
        if (long / scale > 1920) scale = long / 1920
        val dw = ((w / scale).toInt() / 8 * 8).coerceAtLeast(320)
        val dh = ((h / scale).toInt() / 2 * 2).coerceAtLeast(240)
        return dw to dh
    }

    /**
     * The desktop inside Hearth, as one shell script run as the hearth user (stopping that job ends everything:
     * PRoot's --kill-on-exit): TigerVNC's Xvnc on 127.0.0.1:[port] only, VNC passwords only (the random one in
     * [PASSWD]), then the XFCE session on it; when the session ends, Xvnc goes too.
     */
    fun script(width: Int, height: Int, port: Int): String {
        require(width in 320..8192 && height in 240..8192) { "bad size" }
        require(port in 1024..65535) { "bad port" }
        val d = DISPLAY
        return listOf(
            "export DISPLAY=:$d XDG_RUNTIME_DIR=/tmp/runtime-${LinuxSources.USER} XDG_SESSION_TYPE=x11",
            "mkdir -p \"\$XDG_RUNTIME_DIR\" && chmod 700 \"\$XDG_RUNTIME_DIR\"",
            // The X socket folder made first: X servers not running as root won't make it themselves (Xtrans).
            "mkdir -p /tmp/.X11-unix && chmod 1777 /tmp/.X11-unix 2>/dev/null",
            "rm -f /tmp/.X$d-lock /tmp/.X11-unix/X$d",
            // Xvnc's own parameters as -name=value (its documented form); -geometry/-depth are the X server's.
            "Xvnc :$d -geometry ${width}x$height -depth 24 -rfbport=$port -localhost=1 -SecurityTypes=VncAuth " +
                "-rfbauth=/$PASSWD -AlwaysShared=1 -desktop=Hearth -nolisten tcp &",
            "xpid=\$!",
            "i=0; while [ ! -S /tmp/.X11-unix/X$d ] && [ \$i -lt 150 ]; do kill -0 \$xpid 2>/dev/null || { echo '[Hearth: Xvnc stopped]'; exit 1; }; sleep 0.1; i=\$((i+1)); done",
            "echo '[Hearth: desktop ready]'",
            SESSION,
            "kill \$xpid 2>/dev/null; wait \$xpid 2>/dev/null",
        ).joinToString("\n")
    }

    /** apt's command for installing the desktop (as root): fresh package lists, then the packages, with machine
     *  readable progress (APT::Status-Fd) on the same output. */
    fun installCommand(): String = "apt-get update && " +
        LinuxInput.shellJoin(listOf("apt-get", "-y", "--no-install-recommends", "-o", "APT::Status-Fd=1", "install") + PACKAGES)
}

// ==================================================================== apt's progress
/** apt-get with -o APT::Status-Fd=1 prints "dlstatus:<n>:<pct>:<text>" while downloading and "pmstatus:<pkg>:<pct>:
 *  <text>" while installing (a package name can have ":arch" in it). */
object AptStatus {
    data class Line(val phase: String, val pct: Double, val text: String)

    private val RE = Regex("^(dl|pm)status:(.*?):([0-9]+(?:\\.[0-9]+)?):(.*)$")

    fun parse(line: String): Line? {
        val m = RE.find(line.trim()) ?: return null
        val pct = m.groupValues[3].toDoubleOrNull()?.coerceIn(0.0, 100.0) ?: return null
        return Line(m.groupValues[1], pct, m.groupValues[4].trim())
    }

    /** Overall progress of the desktop install: the package lists 0–5 %, the download 5–50 %, installing 50–99 %. */
    fun overall(l: Line): Int = if (l.phase == "dl") (5 + l.pct * 0.45).toInt() else (50 + l.pct * 0.49).toInt()
}

// ==================================================================== VNC's password file
/**
 * DES (FIPS 46-3), the one block cipher VNC authentication uses. Only encryption of single 8-byte blocks: that's all
 * the password file needs. Checked against the JVM's own DES and the standard test vector in the unit tests.
 */
object Des {
    private val IP = intArrayOf(58, 50, 42, 34, 26, 18, 10, 2, 60, 52, 44, 36, 28, 20, 12, 4, 62, 54, 46, 38, 30, 22, 14, 6,
        64, 56, 48, 40, 32, 24, 16, 8, 57, 49, 41, 33, 25, 17, 9, 1, 59, 51, 43, 35, 27, 19, 11, 3, 61, 53, 45, 37, 29, 21, 13, 5,
        63, 55, 47, 39, 31, 23, 15, 7)
    private val FP = intArrayOf(40, 8, 48, 16, 56, 24, 64, 32, 39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22, 62, 30,
        37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28, 35, 3, 43, 11, 51, 19, 59, 27, 34, 2, 42, 10, 50, 18, 58, 26,
        33, 1, 41, 9, 49, 17, 57, 25)
    private val E = intArrayOf(32, 1, 2, 3, 4, 5, 4, 5, 6, 7, 8, 9, 8, 9, 10, 11, 12, 13, 12, 13, 14, 15, 16, 17, 16, 17, 18, 19,
        20, 21, 20, 21, 22, 23, 24, 25, 24, 25, 26, 27, 28, 29, 28, 29, 30, 31, 32, 1)
    private val P = intArrayOf(16, 7, 20, 21, 29, 12, 28, 17, 1, 15, 23, 26, 5, 18, 31, 10, 2, 8, 24, 14, 32, 27, 3, 9, 19, 13,
        30, 6, 22, 11, 4, 25)
    private val PC1 = intArrayOf(57, 49, 41, 33, 25, 17, 9, 1, 58, 50, 42, 34, 26, 18, 10, 2, 59, 51, 43, 35, 27, 19, 11, 3, 60,
        52, 44, 36, 63, 55, 47, 39, 31, 23, 15, 7, 62, 54, 46, 38, 30, 22, 14, 6, 61, 53, 45, 37, 29, 21, 13, 5, 28, 20, 12, 4)
    private val PC2 = intArrayOf(14, 17, 11, 24, 1, 5, 3, 28, 15, 6, 21, 10, 23, 19, 12, 4, 26, 8, 16, 7, 27, 20, 13, 2, 41, 52,
        31, 37, 47, 55, 30, 40, 51, 45, 33, 48, 44, 49, 39, 56, 34, 53, 46, 42, 50, 36, 29, 32)
    private val SHIFTS = intArrayOf(1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1)
    private val S = arrayOf(
        intArrayOf(14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7, 0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8,
            4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0, 15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13),
        intArrayOf(15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10, 3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5,
            0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15, 13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9),
        intArrayOf(10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8, 13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1,
            13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7, 1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12),
        intArrayOf(7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15, 13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9,
            10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4, 3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14),
        intArrayOf(2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9, 14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6,
            4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14, 11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3),
        intArrayOf(12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11, 10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8,
            9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6, 4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13),
        intArrayOf(4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1, 13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6,
            1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2, 6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12),
        intArrayOf(13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7, 1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2,
            7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8, 2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11),
    )

    /** Picks bits of [x] ([n] bits wide, bit 1 = the highest) in the order [t] lists them. */
    private fun perm(x: Long, n: Int, t: IntArray): Long {
        var r = 0L
        for (p in t) r = (r shl 1) or ((x ushr (n - p)) and 1L)
        return r
    }

    private fun long(b: ByteArray): Long { var v = 0L; for (i in 0 until 8) v = (v shl 8) or (b[i].toLong() and 0xff); return v }
    private fun bytes(v: Long): ByteArray = ByteArray(8) { i -> (v ushr (56 - 8 * i)).toByte() }

    /** One 8-byte block encrypted with an 8-byte key (standard DES: the low bit of each key byte is parity, unused). */
    fun encryptBlock(key: ByteArray, block: ByteArray): ByteArray {
        require(key.size == 8 && block.size == 8) { "DES works on 8 bytes" }
        val k = perm(long(key), 64, PC1)
        var c = (k ushr 28) and 0xFFFFFFFL
        var d = k and 0xFFFFFFFL
        val sub = LongArray(16)
        for (i in 0 until 16) {
            repeat(SHIFTS[i]) {
                c = ((c shl 1) or (c ushr 27)) and 0xFFFFFFFL
                d = ((d shl 1) or (d ushr 27)) and 0xFFFFFFFL
            }
            sub[i] = perm((c shl 28) or d, 56, PC2)
        }
        val ip = perm(long(block), 64, IP)
        var l = ip ushr 32
        var r = ip and 0xFFFFFFFFL
        for (i in 0 until 16) {
            val e = perm(r, 32, E) xor sub[i]
            var s = 0L
            for (j in 0 until 8) {
                val six = ((e ushr (42 - 6 * j)) and 0x3F).toInt()
                val row = ((six and 0x20) ushr 4) or (six and 1)
                val col = (six ushr 1) and 0xF
                s = (s shl 4) or S[j][row * 16 + col].toLong()
            }
            val nr = l xor perm(s, 32, P)
            l = r; r = nr
        }
        return bytes(perm((r shl 32) or l, 64, FP))
    }
}

/**
 * VNC passwords. A password file (what `vncpasswd -f` writes and Xvnc's -rfbauth reads) is the password's first 8
 * bytes (zero-padded) DES-encrypted with VNC's fixed key. VNC's DES code (d3des) reads each key byte with its bits in
 * reverse order, so as standard DES the key is the fixed key with every byte's bits reversed.
 */
object VncPassword {
    private val FIXED_KEY = byteArrayOf(23, 82, 107, 6, 35, 78, 88, 7)
    /** Letters and digits that can't be mistaken for each other; 8 of them (VNC uses only the first 8 characters). */
    private const val ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789"

    fun reverseBits(b: Byte): Byte = (Integer.reverse(b.toInt() and 0xff) ushr 24).toByte()
    fun vncKey(key: ByteArray): ByteArray = ByteArray(key.size) { reverseBits(key[it]) }

    /** The 8 bytes of a password file for [password] (only its first 8 characters count, as in VNC). */
    fun file(password: String): ByteArray {
        val pw = ByteArray(8)
        val raw = password.toByteArray(Charsets.ISO_8859_1)
        System.arraycopy(raw, 0, pw, 0, minOf(8, raw.size))
        return Des.encryptBlock(vncKey(FIXED_KEY), pw)
    }

    fun random(rng: SecureRandom = SecureRandom()): String = String(CharArray(8) { ALPHABET[rng.nextInt(ALPHABET.length)] })
}

// ==================================================================== WebSocket (RFC 6455)
object WsProto {
    const val GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
    const val CONT = 0; const val TEXT = 1; const val BINARY = 2; const val CLOSE = 8; const val PING = 9; const val PONG = 10
    /** The biggest frame Hearth takes from the page (noVNC sends small ones: keys, pointer, its settings). */
    const val MAX_IN = 1 shl 20

    fun acceptKey(key: String): String = Base64.getEncoder().encodeToString(
        MessageDigest.getInstance("SHA-1").digest((key.trim() + GUID).toByteArray(Charsets.US_ASCII)))

    /** A frame's header: FIN + [opcode], then the length (7 bits, or 126 + 16 bits, or 127 + 64 bits), then the
     *  masking key when [mask] is given (only clients mask; Hearth's own frames never are). */
    fun header(opcode: Int, len: Long, fin: Boolean = true, mask: ByteArray? = null): ByteArray {
        require(len >= 0)
        val o = ByteArrayOutputStream(14)
        o.write((if (fin) 0x80 else 0) or (opcode and 0x0f))
        val m = if (mask != null) 0x80 else 0
        when {
            len < 126 -> o.write(m or len.toInt())
            len <= 0xFFFF -> { o.write(m or 126); o.write((len ushr 8).toInt() and 0xff); o.write(len.toInt() and 0xff) }
            else -> { o.write(m or 127); for (i in 7 downTo 0) o.write((len ushr (8 * i)).toInt() and 0xff) }
        }
        if (mask != null) { require(mask.size == 4); o.write(mask) }
        return o.toByteArray()
    }

    /** A whole frame (header + payload, masked with [mask] when given). */
    fun encode(opcode: Int, payload: ByteArray, mask: ByteArray? = null, fin: Boolean = true): ByteArray {
        val h = header(opcode, payload.size.toLong(), fin, mask)
        val out = ByteArray(h.size + payload.size)
        System.arraycopy(h, 0, out, 0, h.size)
        for (i in payload.indices) out[h.size + i] = if (mask != null) (payload[i].toInt() xor mask[i and 3].toInt()).toByte() else payload[i]
        return out
    }

    fun closePayload(code: Int, reason: String = ""): ByteArray {
        val r = reason.toByteArray(Charsets.UTF_8).let { if (it.size > 120) it.copyOf(120) else it }
        return byteArrayOf((code ushr 8).toByte(), code.toByte()) + r
    }

    class Frame(val fin: Boolean, val opcode: Int, val payload: ByteArray)

    /**
     * The next frame from [input] (unmasked), or null when the stream ends cleanly before one starts. A client's
     * frames must be masked ([requireMask]); reserved bits, unknown opcodes, oversized or fragmented control frames
     * and frames bigger than [maxPayload] are refused (IOException: the connection is closed).
     */
    fun read(input: InputStream, requireMask: Boolean = true, maxPayload: Int = MAX_IN): Frame? {
        val b0 = input.read()
        if (b0 < 0) return null
        val b1 = byte(input)
        if (b0 and 0x70 != 0) throw IOException("WebSocket: reserved bits set")
        val opcode = b0 and 0x0f
        val fin = b0 and 0x80 != 0
        if (opcode !in setOf(CONT, TEXT, BINARY, CLOSE, PING, PONG)) throw IOException("WebSocket: unknown opcode $opcode")
        val masked = b1 and 0x80 != 0
        if (requireMask && !masked) throw IOException("WebSocket: a client frame without a mask")
        var len = (b1 and 0x7f).toLong()
        if (len == 126L) len = ((byte(input) shl 8) or byte(input)).toLong()
        else if (len == 127L) { len = 0; repeat(8) { len = (len shl 8) or byte(input).toLong() }; if (len < 0) throw IOException("WebSocket: bad length") }
        if (opcode >= 8 && (len > 125 || !fin)) throw IOException("WebSocket: bad control frame")
        if (len > maxPayload) throw IOException("WebSocket: frame too big ($len bytes)")
        val mask = if (masked) ByteArray(4).also { full(input, it) } else null
        val p = ByteArray(len.toInt())
        full(input, p)
        if (mask != null) for (i in p.indices) p[i] = (p[i].toInt() xor mask[i and 3].toInt()).toByte()
        return Frame(fin, opcode, p)
    }

    private fun byte(i: InputStream): Int { val b = i.read(); if (b < 0) throw EOFException("WebSocket: the connection ended in a frame"); return b }
    private fun full(i: InputStream, b: ByteArray) {
        var got = 0
        while (got < b.size) { val n = i.read(b, got, b.size - got); if (n < 0) throw EOFException("WebSocket: the connection ended in a frame"); got += n }
    }
}

/** The opening handshake: the page's HTTP request, checked, and Hearth's answer. */
object WsHandshake {
    class Request(val method: String, val path: String, val headers: Map<String, String>) {
        fun header(name: String): String? = headers[name.lowercase(Locale.US)]
    }

    /** The request's head (up to the empty line), at most 8 KB, or null when the connection ends first. */
    fun readHead(input: InputStream, max: Int = 8192): String? {
        val o = ByteArrayOutputStream()
        var state = 0
        while (true) {
            val b = input.read()
            if (b < 0) return null
            o.write(b)
            if (o.size() > max) throw IOException("WebSocket: request too big")
            state = when {
                b == '\r'.code && (state == 0 || state == 2) -> state + 1
                b == '\n'.code && (state == 1 || state == 3) -> state + 1
                b == '\r'.code -> 1
                else -> 0
            }
            if (state == 4) return String(o.toByteArray(), Charsets.ISO_8859_1)
        }
    }

    fun parse(head: String): Request? {
        val lines = head.split("\r\n")
        val parts = lines.firstOrNull()?.split(' ') ?: return null
        if (parts.size != 3 || !parts[2].startsWith("HTTP/1.")) return null
        val h = HashMap<String, String>()
        for (l in lines.drop(1)) {
            if (l.isEmpty()) continue
            val c = l.indexOf(':'); if (c <= 0) return null
            val k = l.substring(0, c).trim().lowercase(Locale.US)
            val v = l.substring(c + 1).trim()
            h[k] = if (k in h) h[k] + ", " + v else v
        }
        return Request(parts[0], parts[1], h)
    }

    private fun sameSecret(a: String, b: String) = MessageDigest.isEqual(a.toByteArray(Charsets.UTF_8), b.toByteArray(Charsets.UTF_8))

    /** Why [r] is refused (null: it's fine). The path must be "/" + [token]; [origin], when given, must match the
     *  request's Origin exactly (WebView always sends one: Hearth's page is https://appassets.androidplatform.net). */
    fun check(r: Request, token: String, origin: String?): String? {
        if (r.method != "GET") return "not GET"
        if (!sameSecret(r.path, "/$token")) return "wrong token"
        if (r.header("upgrade")?.lowercase(Locale.US)?.split(',')?.any { it.trim() == "websocket" } != true) return "not a WebSocket upgrade"
        if (r.header("connection")?.lowercase(Locale.US)?.split(',')?.any { it.trim() == "upgrade" } != true) return "no Connection: Upgrade"
        if (r.header("sec-websocket-version")?.trim() != "13") return "WebSocket version isn't 13"
        val key = r.header("sec-websocket-key")?.trim() ?: return "no Sec-WebSocket-Key"
        val raw = runCatching { Base64.getDecoder().decode(key) }.getOrNull()
        if (raw == null || raw.size != 16) return "bad Sec-WebSocket-Key"
        if (origin != null && r.header("origin") != origin) return "wrong origin"
        return null
    }

    /** The 101 answer; "binary" is the only subprotocol Hearth speaks (sent back only when the page offered it). */
    fun accept(r: Request): String {
        val offered = r.header("sec-websocket-protocol")?.split(',')?.map { it.trim() } ?: emptyList()
        return "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n" +
            "Sec-WebSocket-Accept: ${WsProto.acceptKey(r.header("sec-websocket-key")!!)}\r\n" +
            (if ("binary" in offered) "Sec-WebSocket-Protocol: binary\r\n" else "") + "\r\n"
    }

    fun refuse(code: Int = 403): String = "HTTP/1.1 $code ${if (code == 403) "Forbidden" else "Bad Request"}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"

    /** A random secret for the WebSocket's path: 128 bits, hex. */
    fun token(rng: SecureRandom = SecureRandom()): String = ByteArray(16).also { rng.nextBytes(it) }.joinToString("") { "%02x".format(it) }
}
