package dev.hearthos.android

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.EOFException
import java.io.IOException
import java.io.InputStream
import java.net.InetAddress
import java.net.InetSocketAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.Base64
import java.util.Random
import javax.crypto.Cipher
import javax.crypto.spec.SecretKeySpec
import kotlin.concurrent.thread

class DesktopLogicTest {
    private fun hex(b: ByteArray) = b.joinToString("") { "%02x".format(it) }
    private fun unhex(s: String) = ByteArray(s.length / 2) { s.substring(2 * it, 2 * it + 2).toInt(16).toByte() }

    // ------------------------------------------------------------ DES and VNC's password file
    @Test fun desStandardVector() {
        // The classic worked example (FIPS 46 / Grabbe): key 133457799BBCDFF1, plaintext 0123456789ABCDEF.
        assertEquals("85e813540f0ab405", hex(Des.encryptBlock(unhex("133457799bbcdff1"), unhex("0123456789abcdef"))))
    }

    @Test fun desMatchesTheJvm() {
        val r = Random(42)
        val c = Cipher.getInstance("DES/ECB/NoPadding")
        repeat(200) {
            val k = ByteArray(8).also { r.nextBytes(it) }
            val p = ByteArray(8).also { r.nextBytes(it) }
            c.init(Cipher.ENCRYPT_MODE, SecretKeySpec(k, "DES"))
            assertArrayEquals(c.doFinal(p), Des.encryptBlock(k, p))
        }
    }

    @Test fun vncPasswordFileIsWhatVncpasswdWrites() {
        // `echo password | vncpasswd -f | xxd -p` → dbd83cfd727a1458 (the well-known value; also what the JVM's DES
        // gives with VNC's fixed key, bits reversed).
        assertEquals("dbd83cfd727a1458", hex(VncPassword.file("password")))
        assertEquals(8, VncPassword.file("").size)
        assertArrayEquals(VncPassword.file("abcdefgh"), VncPassword.file("abcdefghXYZ"))   // only 8 characters count
        assertFalse(VncPassword.file("abcdefgh").contentEquals(VncPassword.file("abcdefgi")))
        assertEquals(0x80.toByte(), VncPassword.reverseBits(1))
        assertEquals(0x0f.toByte(), VncPassword.reverseBits(0xf0.toByte()))
        assertEquals(0b0110_1010.toByte(), VncPassword.reverseBits(0b0101_0110))
    }

    @Test fun vncPasswordFileMatchesTheJvmForRandomPasswords() {
        val fixed = byteArrayOf(23, 82, 107, 6, 35, 78, 88, 7)
        val c = Cipher.getInstance("DES/ECB/NoPadding")
        c.init(Cipher.ENCRYPT_MODE, SecretKeySpec(VncPassword.vncKey(fixed), "DES"))
        repeat(50) {
            val pw = VncPassword.random()
            assertEquals(8, pw.length)
            assertTrue(pw.all { it.isLetterOrDigit() })
            assertArrayEquals(c.doFinal(pw.toByteArray(Charsets.ISO_8859_1)), VncPassword.file(pw))
        }
        assertFalse(VncPassword.random() == VncPassword.random() && VncPassword.random() == VncPassword.random())
    }

    // ------------------------------------------------------------ WebSocket frames
    @Test fun acceptKeyRfcExample() {
        assertEquals("s3pPLMBiTxaQ9kYGzzhZRbK+xOo=", WsProto.acceptKey("dGhlIHNhbXBsZSBub25jZQ=="))
    }

    @Test fun headerLengths() {
        assertEquals("8200", hex(WsProto.header(2, 0)))
        assertEquals("827d", hex(WsProto.header(2, 125)))
        assertEquals("827e007e", hex(WsProto.header(2, 126)))
        assertEquals("827effff", hex(WsProto.header(2, 65535)))
        assertEquals("827f0000000000010000", hex(WsProto.header(2, 65536)))
        assertEquals("827f0000000100000001", hex(WsProto.header(2, 0x100000001L)))
        assertEquals("0201", hex(WsProto.header(2, 1, fin = false)))
        assertEquals("88fe007e01020304", hex(WsProto.header(8, 126, mask = byteArrayOf(1, 2, 3, 4))))
    }

    @Test fun decodesTheRfcMaskedHello() {
        // RFC 6455 5.7: a masked text frame "Hello".
        val f = WsProto.read(ByteArrayInputStream(unhex("818537fa213d7f9f4d5158")))!!
        assertTrue(f.fin); assertEquals(1, f.opcode); assertEquals("Hello", String(f.payload))
        // The unmasked one is refused from a client, read when masks aren't required.
        try { WsProto.read(ByteArrayInputStream(unhex("810548656c6c6f"))); fail() } catch (e: IOException) { }
        assertEquals("Hello", String(WsProto.read(ByteArrayInputStream(unhex("810548656c6c6f")), requireMask = false)!!.payload))
    }

    @Test fun roundTripsEveryLengthForm() {
        val mask = byteArrayOf(0x37, 0xfa.toByte(), 0x21, 0x3d)
        for (n in listOf(0, 1, 125, 126, 127, 65535, 65536, 70000)) {
            val p = ByteArray(n) { (it * 31 + 7).toByte() }
            val masked = WsProto.encode(2, p, mask)
            val f = WsProto.read(ByteArrayInputStream(masked))!!
            assertEquals("length $n", n, f.payload.size)
            assertArrayEquals(p, f.payload)
            val plain = WsProto.read(ByteArrayInputStream(WsProto.encode(2, p)), requireMask = false)!!
            assertArrayEquals(p, plain.payload)
        }
        // Two frames back to back, then a clean end.
        val two = ByteArrayInputStream(WsProto.encode(2, byteArrayOf(1, 2), mask) + WsProto.encode(9, byteArrayOf(9), mask))
        assertEquals(2, WsProto.read(two)!!.opcode)
        assertEquals(9, WsProto.read(two)!!.opcode)
        assertNull(WsProto.read(two))
    }

    @Test fun refusesBadFrames() {
        val mask = byteArrayOf(1, 2, 3, 4)
        fun bad(b: ByteArray, max: Int = WsProto.MAX_IN) { try { WsProto.read(ByteArrayInputStream(b), maxPayload = max); fail(hex(b.take(12).toByteArray())) } catch (e: IOException) { } }
        bad(WsProto.encode(2, ByteArray(200), mask), max = 100)                    // too big
        bad(WsProto.encode(9, ByteArray(126), mask))                                // control frame over 125
        bad(WsProto.encode(8, ByteArray(2), mask, fin = false))                     // fragmented control frame
        bad(WsProto.encode(3, ByteArray(2), mask))                                  // unknown opcode
        bad(byteArrayOf(0xc2.toByte(), 0x80.toByte(), 1, 2, 3, 4))                  // reserved bit
        bad(unhex("82ff800000000000000101020304"))                                  // 64-bit length with the top bit set
        bad(unhex("82ff7fffffffffffffff01020304"))                                  // 64-bit length, far too big
        try { WsProto.read(ByteArrayInputStream(WsProto.encode(2, ByteArray(10), mask).copyOf(9))); fail() } catch (e: EOFException) { }
        assertNull(WsProto.read(ByteArrayInputStream(ByteArray(0))))
    }

    @Test fun closePayload() {
        assertEquals("03e8", hex(WsProto.closePayload(1000)))
        assertEquals("03f3" + hex("x".toByteArray()), hex(WsProto.closePayload(1011, "x")))
    }

    // ------------------------------------------------------------ the handshake
    private val token = "0123456789abcdef0123456789abcdef"
    private fun request(path: String = "/$token", origin: String? = WsBridge.PAGE_ORIGIN, key: String? = "dGhlIHNhbXBsZSBub25jZQ==",
                        version: String = "13", proto: String? = null, upgrade: String = "websocket", connection: String = "keep-alive, Upgrade") =
        "GET $path HTTP/1.1\r\nHost: 127.0.0.1:5000\r\nUpgrade: $upgrade\r\nConnection: $connection\r\n" +
            (if (key != null) "Sec-WebSocket-Key: $key\r\n" else "") + "Sec-WebSocket-Version: $version\r\n" +
            (if (origin != null) "Origin: $origin\r\n" else "") + (if (proto != null) "Sec-WebSocket-Protocol: $proto\r\n" else "") + "\r\n"

    private fun check(head: String, origin: String? = WsBridge.PAGE_ORIGIN): String? {
        val r = WsHandshake.parse(head) ?: return "unparsable"
        return WsHandshake.check(r, token, origin)
    }

    @Test fun handshakeChecks() {
        assertNull(check(request()))
        assertNull(check(request(upgrade = "WebSocket", connection = "Upgrade")))
        assertEquals("wrong token", check(request(path = "/0123456789abcdef0123456789abcdee")))
        assertEquals("wrong token", check(request(path = "/")))
        assertEquals("wrong token", check(request(path = "/$token/x")))
        assertEquals("wrong origin", check(request(origin = "https://evil.example")))
        assertEquals("wrong origin", check(request(origin = null)))
        assertNull(check(request(origin = null), origin = null))
        assertEquals("no Sec-WebSocket-Key", check(request(key = null)))
        assertEquals("bad Sec-WebSocket-Key", check(request(key = "short")))
        assertEquals("WebSocket version isn't 13", check(request(version = "8")))
        assertEquals("not a WebSocket upgrade", check(request(upgrade = "h2c")))
        assertEquals("not GET", check(request().replaceFirst("GET", "POST")))
        assertNull(WsHandshake.parse("nonsense\r\n\r\n"))
        assertNull(WsHandshake.parse("GET / HTTP/1.1\r\nno colon here\r\n\r\n"))
    }

    @Test fun handshakeAnswer() {
        val plain = WsHandshake.accept(WsHandshake.parse(request())!!)
        assertTrue(plain.startsWith("HTTP/1.1 101 "))
        assertTrue(plain.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo=\r\n"))
        assertFalse(plain.contains("Sec-WebSocket-Protocol"))
        assertTrue(plain.endsWith("\r\n\r\n"))
        assertTrue(WsHandshake.accept(WsHandshake.parse(request(proto = "base64, binary"))!!).contains("Sec-WebSocket-Protocol: binary\r\n"))
        assertFalse(WsHandshake.accept(WsHandshake.parse(request(proto = "base64"))!!).contains("Sec-WebSocket-Protocol"))
    }

    @Test fun readsTheHeadOnly() {
        val body = request() + "\u0082\u0080rest"
        val i = ByteArrayInputStream(body.toByteArray(Charsets.ISO_8859_1))
        assertEquals(request(), WsHandshake.readHead(i))
        assertEquals(0x82, i.read())
        assertNull(WsHandshake.readHead(ByteArrayInputStream("GET / HTTP/1.1\r\n".toByteArray())))
        try { WsHandshake.readHead(ByteArrayInputStream(ByteArray(9000) { 'a'.code.toByte() })); fail() } catch (e: IOException) { }
        assertEquals(32, WsHandshake.token().length)
        assertFalse(WsHandshake.token() == WsHandshake.token())
    }

    // ------------------------------------------------------------ the bridge, end to end (a fake VNC server)
    private class FakeVnc {
        val server = ServerSocket(0, 4, InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1)))
        val port get() = server.localPort
        @Volatile var big = 0
        init {
            thread(isDaemon = true) {
                while (true) {
                    val s = try { server.accept() } catch (e: IOException) { break }
                    thread(isDaemon = true) {
                        s.use {
                            val o = s.getOutputStream()
                            o.write("RFB 003.008\n".toByteArray()); o.flush()
                            val i = s.getInputStream()
                            val buf = ByteArray(4096)
                            while (true) {
                                val n = i.read(buf); if (n < 0) break
                                if (n == 1 && buf[0] == 'B'.code.toByte()) { o.write(ByteArray(big) { (it % 251).toByte() }); o.flush(); continue }
                                o.write(buf, 0, n); o.flush()                 // echo
                            }
                        }
                    }
                }
            }
        }
        fun close() = server.close()
    }

    private fun readFull(i: InputStream, n: Int): ByteArray { val b = ByteArray(n); var g = 0; while (g < n) { val r = i.read(b, g, n - g); if (r < 0) throw EOFException(); g += r }; return b }

    private fun connect(bridge: WsBridge, path: String = "/$token", origin: String? = WsBridge.PAGE_ORIGIN): Pair<Socket, String> {
        val s = Socket()
        s.connect(InetSocketAddress("127.0.0.1", bridge.port), 2000)
        s.soTimeout = 5000
        val key = Base64.getEncoder().encodeToString(ByteArray(16) { it.toByte() })
        s.getOutputStream().write(request(path = path, origin = origin, key = key).toByteArray(Charsets.ISO_8859_1))
        val head = WsHandshake.readHead(s.getInputStream()) ?: ""
        return s to head
    }

    @Test fun bridgeCarriesVncBothWays() {
        val vnc = FakeVnc()
        val bridge = WsBridge(token, WsBridge.PAGE_ORIGIN, { Socket("127.0.0.1", vnc.port) }).start()
        try {
            val (s, head) = connect(bridge)
            assertTrue(head, head.startsWith("HTTP/1.1 101 "))
            assertTrue(head.contains("Sec-WebSocket-Accept: " + WsProto.acceptKey(Base64.getEncoder().encodeToString(ByteArray(16) { it.toByte() }))))
            val i = s.getInputStream(); val o = s.getOutputStream()
            // Xvnc speaks first: its version, in binary frames (unmasked, from the server).
            val got = StringBuilder()
            while (got.length < 12) { val f = WsProto.read(i, requireMask = false)!!; assertEquals(2, f.opcode); got.append(String(f.payload)) }
            assertEquals("RFB 003.008\n", got.toString())
            // The page's bytes (masked frames) reach Xvnc, and come back.
            val mask = byteArrayOf(9, 8, 7, 6)
            o.write(WsProto.encode(2, "RFB 003.008\n".toByteArray(), mask)); o.flush()
            val echo = StringBuilder()
            while (echo.length < 12) echo.append(String(WsProto.read(i, requireMask = false)!!.payload))
            assertEquals("RFB 003.008\n", echo.toString())
            // A big framebuffer update arrives whole, over several frames.
            vnc.big = 300_000
            o.write(WsProto.encode(2, "B".toByteArray(), mask)); o.flush()
            var total = 0
            var ok = true
            while (total < vnc.big) {
                val f = WsProto.read(i, requireMask = false, maxPayload = 1 shl 24)!!
                for (k in f.payload.indices) if (f.payload[k] != ((total + k) % 251).toByte()) ok = false
                total += f.payload.size
            }
            assertEquals(300_000, total); assertTrue("bytes out of order", ok)
            // Ping → pong with the same data.
            o.write(WsProto.encode(9, "hi".toByteArray(), mask)); o.flush()
            val pong = WsProto.read(i, requireMask = false)!!
            assertEquals(10, pong.opcode); assertEquals("hi", String(pong.payload))
            // Close → close back, then the end.
            o.write(WsProto.encode(8, WsProto.closePayload(1000), mask)); o.flush()
            var f = WsProto.read(i, requireMask = false)
            while (f != null && f.opcode != 8) f = WsProto.read(i, requireMask = false)
            assertNotNull(f)
            s.close()
        } finally { bridge.stop(); vnc.close() }
    }

    @Test fun bridgeRefusesStrangers() {
        val vnc = FakeVnc()
        val bridge = WsBridge(token, WsBridge.PAGE_ORIGIN, { Socket("127.0.0.1", vnc.port) }).start()
        try {
            val (a, h1) = connect(bridge, path = "/wrong"); a.close()
            assertTrue(h1, h1.startsWith("HTTP/1.1 403"))
            val (b, h2) = connect(bridge, origin = "https://evil.example"); b.close()
            assertTrue(h2, h2.startsWith("HTTP/1.1 403"))
            // Not WebSocket at all.
            Socket("127.0.0.1", bridge.port).use { s ->
                s.soTimeout = 5000
                s.getOutputStream().write("hello\r\n\r\n".toByteArray())
                assertTrue((WsHandshake.readHead(s.getInputStream()) ?: "").startsWith("HTTP/1.1 400"))
            }
        } finally { bridge.stop(); vnc.close() }
    }

    @Test fun bridgeClosesCleanlyWhenTheDesktopIsDown() {
        val dead = ServerSocket(0).use { it.localPort }       // nothing listens here
        val bridge = WsBridge(token, null, { Socket().apply { connect(InetSocketAddress("127.0.0.1", dead), 500) } }).start()
        try {
            val (s, head) = connect(bridge, origin = null)
            assertTrue(head.startsWith("HTTP/1.1 101 "))
            val f = WsProto.read(s.getInputStream(), requireMask = false)!!
            assertEquals(8, f.opcode)
            assertEquals(1011, ((f.payload[0].toInt() and 0xff) shl 8) or (f.payload[1].toInt() and 0xff))
            s.close()
        } finally { bridge.stop() }
        // Stopped: nothing listens anymore (the JVM finishes closing a socket another thread is accepting on a
        // moment later, so give it up to 2 s).
        var listening = true
        val until = System.currentTimeMillis() + 2000
        while (listening && System.currentTimeMillis() < until) {
            listening = try { Socket().use { it.connect(InetSocketAddress("127.0.0.1", bridge.port), 200) }; Thread.sleep(50); true } catch (e: IOException) { false }
        }
        assertFalse("still listening", listening)
    }

    // ------------------------------------------------------------ the desktop's plan
    @Test fun geometry() {
        assertEquals(1280 to 800, DesktopPlan.geometry(1920, 1200))       // the Oukitel RT8
        assertEquals(800 to 1280, DesktopPlan.geometry(1200, 1920))       // the same, upright
        assertEquals(1600 to 720, DesktopPlan.geometry(2400, 1080))       // a phone
        assertEquals(1024 to 576, DesktopPlan.geometry(1280, 720))        // small screens keep 1024 wide
        assertEquals(1920 to 1080, DesktopPlan.geometry(3840, 2160))      // big ones stop at 1920
        assertEquals(800 to 480, DesktopPlan.geometry(800, 480))          // never bigger than the screen
        for ((w, h) in listOf(0 to 0, 100 to 800, 800 to 99999, -5 to 800)) {
            try { DesktopPlan.geometry(w, h); fail("$w×$h") } catch (e: IllegalArgumentException) { }
        }
        val (w, h) = DesktopPlan.geometry(2340, 1080)
        assertEquals(0, w % 8); assertEquals(0, h % 2)
    }

    @Test fun script() {
        val s = DesktopPlan.script(1280, 800, 40123)
        assertTrue(s.contains("Xvnc :1 -geometry 1280x800 -depth 24 -rfbport=40123 -localhost=1 -SecurityTypes=VncAuth -rfbauth=/home/hearth/.vnc/hearth-passwd"))
        assertTrue(s.contains("DISPLAY=:1"))
        assertTrue(s.contains("dbus-launch --exit-with-session startxfce4"))
        assertTrue(s.lines().last().startsWith("kill \$xpid"))
        assertFalse(s.contains("SecurityTypes=None"))
        try { DesktopPlan.script(1280, 800, 80); fail() } catch (e: IllegalArgumentException) { }
        try { DesktopPlan.script(10, 800, 40000); fail() } catch (e: IllegalArgumentException) { }
    }

    @Test fun modesAndPackages() {
        assertEquals("hearth", DesktopPlan.mode("hearth"))
        assertEquals("x11", DesktopPlan.mode(" X11 "))
        for (m in listOf("", null, "vnc", "hearth; rm -rf /")) try { DesktopPlan.mode(m); fail("$m") } catch (e: IllegalArgumentException) { }
        assertTrue(DesktopPlan.PACKAGES.containsAll(listOf("xfce4", "dbus-x11", "tigervnc-standalone-server")))
        DesktopPlan.PACKAGES.forEach { assertTrue(it, LinuxInput.isPackageName(it)) }
        DesktopPlan.NEEDS.forEach { (_, pkg) -> assertTrue(pkg, pkg in DesktopPlan.PACKAGES) }
        val cmd = DesktopPlan.installCommand()
        assertTrue(cmd.startsWith("apt-get update && 'apt-get' '-y' '--no-install-recommends' '-o' 'APT::Status-Fd=1' 'install' 'xfce4'"))
    }

    @Test fun aptStatusLines() {
        val dl = AptStatus.parse("dlstatus:3:42.5:Retrieving file 3 of 120")!!
        assertEquals("dl", dl.phase); assertEquals(42.5, dl.pct, 0.0); assertEquals("Retrieving file 3 of 120", dl.text)
        assertEquals(24, AptStatus.overall(dl))
        val pm = AptStatus.parse("pmstatus:libc6:arm64:61.2:Installing libc6 (arm64)")!!
        assertEquals("pm", pm.phase); assertEquals(61.2, pm.pct, 0.0); assertEquals("Installing libc6 (arm64)", pm.text)
        assertEquals(79, AptStatus.overall(pm))
        assertEquals(99, AptStatus.overall(AptStatus.parse("pmstatus:dpkg-exec:100:Running dpkg")!!))
        assertNull(AptStatus.parse("Get:1 http://deb.debian.org/debian trixie InRelease [140 kB]"))
        assertNull(AptStatus.parse("pmstatus:x:notanumber:y"))
        assertNull(AptStatus.parse("Setting up xfce4 (4.20) ..."))
    }
}
