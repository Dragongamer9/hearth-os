package dev.hearthos.android

import java.io.BufferedInputStream
import java.io.BufferedOutputStream
import java.io.IOException
import java.io.OutputStream
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.Collections
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicInteger
import kotlin.concurrent.thread

/**
 * The desktop inside Hearth: noVNC (in Hearth's page) speaks WebSocket, Xvnc speaks VNC over TCP. This is the bridge
 * between them (what websockify does on a PC), tiny and without dependencies: a WebSocket server on 127.0.0.1, a
 * random port, that only answers at /<token> (128 random bits, new for every desktop start) and only to Hearth's
 * page origin; every binary frame's bytes go to Xvnc and Xvnc's bytes come back as binary frames.
 *
 * Any app on the device can reach 127.0.0.1, so three things keep them out: the token (the WebSocket), the VNC
 * password (Xvnc itself, random per start) and Xvnc listening on 127.0.0.1 only.
 *
 * Pure java.net: the unit tests run it on a plain JVM against a fake VNC server.
 */
class WsBridge(
    private val token: String,
    private val origin: String?,
    private val connectVnc: () -> Socket,
    private val log: (String) -> Unit = {},
) {
    private val server = ServerSocket(0, 8, InetAddress.getByAddress(byteArrayOf(127, 0, 0, 1)))
    private val open = Collections.newSetFromMap(ConcurrentHashMap<Socket, Boolean>())
    private val clients = AtomicInteger(0)
    private val refused = AtomicInteger(0)
    @Volatile private var closed = false

    val port: Int get() = server.localPort

    fun start(): WsBridge {
        thread(isDaemon = true, name = "hearth-ws") { acceptLoop() }
        return this
    }

    fun stop() {
        closed = true
        runCatching { server.close() }
        open.toList().forEach { runCatching { it.close() } }
        open.clear()
    }

    private fun acceptLoop() {
        while (!closed) {
            val s = try { server.accept() } catch (e: IOException) { break }
            if (clients.get() >= MAX_CLIENTS) { runCatching { s.close() }; continue }
            clients.incrementAndGet()
            open += s
            thread(isDaemon = true, name = "hearth-ws-client") {
                try { serve(s) } catch (e: Exception) { log("ws: ${e.message}") }
                finally { open -= s; runCatching { s.close() }; clients.decrementAndGet() }
            }
        }
    }

    private fun serve(s: Socket) {
        s.soTimeout = 5000                       // the handshake comes at once or not at all
        s.tcpNoDelay = true
        val input = BufferedInputStream(s.getInputStream(), 65536)
        val out = BufferedOutputStream(s.getOutputStream(), 65536 + 16)
        val head = WsHandshake.readHead(input) ?: return
        val req = WsHandshake.parse(head)
        val why = if (req == null) "bad request" else WsHandshake.check(req, token, origin)
        if (why != null) {
            out.write(WsHandshake.refuse(if (req == null) 400 else 403).toByteArray(Charsets.ISO_8859_1)); out.flush()
            if (refused.incrementAndGet() <= 20) log("ws: refused a connection ($why)")
            return
        }
        out.write(WsHandshake.accept(req!!).toByteArray(Charsets.ISO_8859_1)); out.flush()
        log("ws: the desktop view connected")
        s.soTimeout = 0
        val lock = Any()
        fun send(opcode: Int, payload: ByteArray) = synchronized(lock) { out.write(WsProto.encode(opcode, payload)); out.flush() }

        val vnc = try { connectVnc() } catch (e: Exception) {
            // The desktop isn't up (yet): a clean close, noVNC reports it and the page tries again.
            runCatching { send(WsProto.CLOSE, WsProto.closePayload(1011, "The desktop isn't running")) }
            return
        }
        open += vnc
        try {
            vnc.tcpNoDelay = true
            val vin = vnc.getInputStream()
            val vout = vnc.getOutputStream()
            // Xvnc → page.
            val down = thread(isDaemon = true, name = "hearth-ws-down") {
                val buf = ByteArray(65536)
                try {
                    while (true) {
                        val n = vin.read(buf); if (n < 0) break
                        synchronized(lock) { out.write(WsProto.header(WsProto.BINARY, n.toLong())); out.write(buf, 0, n); out.flush() }
                    }
                } catch (_: IOException) { }
                runCatching { send(WsProto.CLOSE, WsProto.closePayload(1000)) }
                runCatching { s.shutdownOutput() }
            }
            // Page → Xvnc.
            pump(input, vout) { op, p -> send(op, p) }
            runCatching { vnc.shutdownOutput() }
            down.join(2000)
        } finally {
            open -= vnc
            runCatching { vnc.close() }
        }
    }

    /** Frames from the page: data to Xvnc, pings answered, a close answered and the end. */
    private fun pump(input: BufferedInputStream, vout: OutputStream, send: (Int, ByteArray) -> Unit) {
        try {
            while (true) {
                val f = WsProto.read(input) ?: break
                when (f.opcode) {
                    WsProto.BINARY, WsProto.TEXT, WsProto.CONT -> { vout.write(f.payload); vout.flush() }
                    WsProto.PING -> send(WsProto.PONG, f.payload)
                    WsProto.CLOSE -> { runCatching { send(WsProto.CLOSE, f.payload.copyOf(minOf(2, f.payload.size))) }; break }
                    else -> {}
                }
            }
        } catch (e: IOException) { if (!closed) log("ws: ${e.message}") }
    }

    companion object {
        /** noVNC holds one connection; a second is a reconnect that overlaps the old one closing. */
        const val MAX_CLIENTS = 3
        /** Hearth's page: WebViewAssetLoader's origin. */
        const val PAGE_ORIGIN = "https://appassets.androidplatform.net"
    }
}
