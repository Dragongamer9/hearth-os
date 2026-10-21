package dev.hearthos.android

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.File
import java.nio.file.Files
import java.nio.file.LinkOption

class LinuxLogicTest {
    // ------------------------------------------------------------ a small tar writer for the tests
    private class Tar {
        val out = ByteArrayOutputStream()
        fun entry(name: String, type: Char, data: ByteArray = ByteArray(0), mode: Int = 0b110_100_100, link: String = "", prefix: String = "") {
            val h = ByteArray(512)
            fun put(off: Int, len: Int, s: String) { val b = s.toByteArray(); System.arraycopy(b, 0, h, off, minOf(b.size, len)) }
            fun oct(off: Int, len: Int, v: Long) = put(off, len, v.toString(8).padStart(len - 1, '0'))
            put(0, 100, name); oct(100, 8, mode.toLong()); oct(108, 8, 0); oct(116, 8, 0)
            oct(124, 12, if (type == '0' || type == 'L' || type == 'x') data.size.toLong() else 0); oct(136, 12, 0)
            h[156] = type.code.toByte(); put(157, 100, link); put(257, 6, "ustar"); put(263, 2, "00"); put(345, 155, prefix)
            for (i in 148 until 156) h[i] = ' '.code.toByte()
            val sum = h.sumOf { it.toInt() and 0xff }
            put(148, 8, sum.toString(8).padStart(6, '0') + "\u0000 ")
            out.write(h)
            if (type == '0' || type == 'L' || type == 'x') { out.write(data); out.write(ByteArray(((512 - data.size % 512) % 512))) }
        }
        fun file(name: String, text: String, mode: Int = 0b110_100_100) = entry(name, '0', text.toByteArray(), mode)
        fun bytes(): ByteArray { out.write(ByteArray(1024)); return out.toByteArray() }
    }

    private fun tmp(): File = Files.createTempDirectory("hearth-linux-test").toFile()
    private fun mode(f: File) = Files.getPosixFilePermissions(f.toPath(), LinkOption.NOFOLLOW_LINKS)

    // ------------------------------------------------------------ tar paths
    @Test fun tarPathsNeverEscape() {
        assertEquals("a/b", TarPath.safe("./a/b"))
        assertEquals("etc/passwd", TarPath.safe("/etc/passwd"))      // a leading / is dropped, as GNU tar does
        assertNull(TarPath.safe("../x"))
        assertNull(TarPath.safe("a/../../x"))
        assertNull(TarPath.safe("a/.."))
        assertNull(TarPath.safe("./"))
        assertNull(TarPath.safe("a\u0000b"))
        assertEquals("etc/hosts", TarPath.safe("debian-trixie-aarch64/etc/hosts", 1))
        assertNull(TarPath.safe("debian-trixie-aarch64/", 1))
        assertEquals("usr/bin", TarPath.safe("./top//usr/./bin/", 1))
    }

    // ------------------------------------------------------------ extracting
    @Test fun extractsFilesFoldersLinksAndModes() {
        val t = Tar()
        t.entry("top/", '5', mode = 0b111_101_101)
        t.entry("top/bin/", '5', mode = 0b111_101_101)
        t.file("top/bin/sh", "#!shell", 0b111_101_101)
        t.entry("top/usr/bin/", '5')
        t.entry("top/usr/bin/sh2", '2', link = "/bin/sh")              // absolute target: the Linux's, kept as is
        t.entry("top/bin/sh.hard", '1', link = "top/bin/sh")
        t.entry("top/ro/", '5', mode = 0b101_101_101)                  // a read-only folder with a file in it
        t.file("top/ro/file", "x")
        t.entry("top/dev/null", '3')                                    // devices are skipped
        val root = File(tmp(), "rootfs")
        val x = TarExtractor(root, 1)
        x.extract(ByteArrayInputStream(t.bytes()))
        assertEquals("#!shell", File(root, "bin/sh").readText())
        assertTrue(File(root, "bin/sh").canExecute())
        assertTrue(Files.isSymbolicLink(File(root, "usr/bin/sh2").toPath()))
        assertEquals("/bin/sh", Files.readSymbolicLink(File(root, "usr/bin/sh2").toPath()).toString())
        assertEquals("#!shell", File(root, "bin/sh.hard").readText())
        assertEquals("x", File(root, "ro/file").readText())
        assertTrue("the owner keeps write on folders", java.nio.file.attribute.PosixFilePermission.OWNER_WRITE in mode(File(root, "ro")))
        assertFalse(File(root, "dev/null").exists())
        assertEquals(2, x.skipped)                                      // the top folder itself and the device
    }

    @Test fun neverWritesOutsideTheRoot() {
        val base = tmp()
        val outside = File(base, "outside").apply { mkdirs() }
        val t = Tar()
        t.entry("top/evil", '2', link = outside.absolutePath)          // a link to somewhere else…
        t.file("top/evil/pwned", "no")                                  // …and a file through it: refused
        t.file("top/../escape", "no")
        t.file("../../escape2", "no")
        t.entry("top/hard", '1', link = "../../etc/passwd")             // a hard link out: refused
        t.entry("top/hard2", '1', link = "top/evil/x")                  // a hard link through a symlink: refused
        t.file("top/ok", "yes")
        val root = File(base, "rootfs")
        val x = TarExtractor(root, 1)
        x.extract(ByteArrayInputStream(t.bytes()))
        assertEquals(0, outside.listFiles()!!.size)
        assertFalse(File(base, "escape").exists())
        assertFalse(File(base, "escape2").exists())
        assertFalse(File(root, "hard").exists())
        assertFalse(File(root, "hard2").exists())
        assertEquals("yes", File(root, "ok").readText())
        assertTrue(Files.isSymbolicLink(File(root, "evil").toPath()))
    }

    @Test fun longNamesAndPax() {
        val long = "top/" + "d".repeat(120) + "/file.txt"
        val t = Tar()
        t.entry("././@LongLink", 'L', (long + "\u0000").toByteArray())
        t.file(long.take(99), "gnu")
        val pax = "path=top/pax/name.txt\n"
        var n = pax.length + 2
        while ("$n $pax".length != n) n = "$n $pax".length
        val rec = "$n $pax"
        t.entry("PaxHeaders/x", 'x', rec.toByteArray())
        t.file("short", "pax")
        t.file("name.txt", "ustar prefix", 0b110_100_100)
        t.entry("name2.txt", '0', "pre".toByteArray(), prefix = "top/pre")
        val root = File(tmp(), "r")
        TarExtractor(root, 1).extract(ByteArrayInputStream(t.bytes()))
        assertEquals("gnu", File(root, "d".repeat(120) + "/file.txt").readText())
        assertEquals("pax", File(root, "pax/name.txt").readText())
        assertEquals("pre", File(root, "pre/name2.txt").readText())
    }

    @Test fun damagedArchivesFail() {
        val b = Tar().apply { file("top/a", "hello") }.bytes()
        b[0] = 'X'.code.toByte()                                       // the checksum no longer matches
        try { TarExtractor(File(tmp(), "r"), 1).extract(ByteArrayInputStream(b)); fail("no error") } catch (e: java.io.IOException) {}
        val cut = Tar().apply { file("top/a", "x".repeat(2000)) }.bytes().copyOf(900)
        try { TarExtractor(File(tmp(), "r2"), 1).extract(ByteArrayInputStream(cut)); fail("no error") } catch (e: java.io.EOFException) {}
    }

    // ------------------------------------------------------------ deleting
    @Test fun deleteTreeNeverFollowsLinks() {
        val base = tmp()
        val keep = File(base, "keep").apply { mkdirs() }; File(keep, "precious").writeText("1")
        val tree = File(base, "tree"); File(tree, "a/b").mkdirs(); File(tree, "a/b/f").writeText("x")
        Files.createSymbolicLink(File(tree, "a/link").toPath(), keep.toPath())
        Files.createSymbolicLink(File(tree, "abs").toPath(), File("/").toPath())
        File(tree, "a/b").setWritable(false)
        SafeFs.deleteTree(tree)
        assertFalse(tree.exists())
        assertEquals("1", File(keep, "precious").readText())
    }

    // ------------------------------------------------------------ input
    @Test fun packageNames() {
        assertEquals(listOf("vim", "g++", "libc6-dev", "python3.13"), LinuxInput.packages(listOf("vim", " g++ ", "libc6-dev", "python3.13", "vim")))
        for (bad in listOf("-o", "Vim", "a;b", "a b", "../x", "", "\$(x)", "a/b")) {
            try { LinuxInput.packages(listOf(bad)); fail("accepted '$bad'") } catch (e: IllegalArgumentException) {}
        }
        assertEquals(listOf("vim", "git", "htop"), LinuxInput.splitPackages(" vim, git  htop "))
        assertEquals(listOf("apt-get", "-y", "--no-install-recommends", "install", "vim"), LinuxInput.aptArgs("install", listOf("vim")))
        assertEquals(listOf("apt-get", "-y", "upgrade"), LinuxInput.aptArgs("upgrade", emptyList()))
        try { LinuxInput.aptArgs("purge-all", listOf("x")); fail() } catch (e: IllegalArgumentException) {}
    }

    @Test fun commands() {
        assertEquals("ls -la", LinuxInput.command("  ls\u0000 -la "))
        try { LinuxInput.command(" \u0000 "); fail() } catch (e: IllegalArgumentException) {}
        try { LinuxInput.command("x".repeat(LinuxInput.MAX_COMMAND + 1)); fail() } catch (e: IllegalArgumentException) {}
        assertEquals("'apt-get' 'install' 'it'\\''s'", LinuxInput.shellJoin(listOf("apt-get", "install", "it's")))
    }

    // ------------------------------------------------------------ PRoot
    @Test fun prootArgs() {
        val spec = ProotSpec(proot = "/lib/libproot.so", rootfs = "/d/linux/rootfs", sysdata = "/d/linux/sysdata",
            command = listOf("/bin/sh", "-lc", "echo hi"), readable = { it == "/proc/version" })
        val a = ProotArgs.build(spec)
        assertEquals("/lib/libproot.so", a.first())
        assertEquals(listOf("/bin/sh", "-lc", "echo hi"), a.takeLast(3))
        for (f in listOf("--kill-on-exit", "--link2symlink", "--rootfs=/d/linux/rootfs", "--bind=/dev", "--bind=/proc", "--bind=/sys",
            "--change-id=1000:1000", "--cwd=/home/hearth", "--bind=/d/linux/rootfs/tmp:/dev/shm", "--bind=/d/linux/sysdata/loadavg:/proc/loadavg"))
            assertTrue("missing $f in $a", f in a)
        assertFalse("-0" in a)
        assertFalse("a readable /proc file gets no stand-in", a.any { it.endsWith(":/proc/version") })
        assertFalse(a.any { it.contains("sdcard") })

        val r = ProotArgs.build(spec.copy(root = true, sdcard = "/storage/emulated/0", hasDevFd = false))
        assertTrue("-0" in r); assertTrue("--cwd=/root" in r)
        assertFalse(r.any { it.startsWith("--change-id") })
        assertTrue("--bind=/storage/emulated/0:/mnt/sdcard" in r)
        assertTrue("--bind=/proc/self/fd:/dev/fd" in r)
        assertTrue("the command comes after every option", r.indexOf("/bin/sh") > r.indexOfLast { it.startsWith("--") })
    }

    @Test fun prootEnv() {
        val e = ProotArgs.env("/d/rootfs", "/d/ptmp", "/lib/libproot-loader.so", "/lib/libproot-loader32.so", root = false, extra = mapOf("DISPLAY" to ":0"))
        assertEquals("/home/hearth", e["HOME"]); assertEquals("hearth", e["USER"])
        assertEquals("/lib/libproot-loader.so", e["PROOT_LOADER"]); assertEquals("/d/ptmp", e["PROOT_TMP_DIR"])
        assertEquals("/d/rootfs/.l2s", e["PROOT_L2S_DIR"]); assertEquals(":0", e["DISPLAY"]); assertEquals("/tmp", e["TMPDIR"])
        assertTrue(e["PATH"]!!.startsWith("/usr/local/sbin:"))
        assertFalse(e.keys.any { it.startsWith("LD_") })
        assertEquals("/root", ProotArgs.env("/r", "/t", "/l", null, root = true)["HOME"])
    }

    // ------------------------------------------------------------ the fake /proc files
    @Test fun fakeProc() {
        val stat = FakeProc.stat(4).lines()
        val cpu = stat[0].removePrefix("cpu").trim().split(" ").map { it.toLong() }
        val each = (1..4).map { stat[it].split(" ").drop(1).map { v -> v.toLong() } }
        assertEquals(cpu, (0 until cpu.size).map { i -> each.sumOf { it[i] } })
        assertTrue(stat[4].startsWith("cpu3 ")); assertFalse(stat[5].startsWith("cpu"))
        assertEquals("1000.00 1920.00\n", FakeProc.uptime(4))                 // 4 CPUs × 48000 idle ticks / 100
        assertTrue(FakeProc.LOADAVG.matches(Regex("[0-9.]+ [0-9.]+ [0-9.]+ \\d+/\\d+ \\d+\n")))
        assertTrue(FakeProc.VERSION.startsWith("Linux version "))
        val e = FakeProc.entries(2)
        assertTrue(e.all { it.guest.startsWith("/proc/") && it.content.endsWith("\n") && !it.name.contains('/') })
        assertEquals(e.size, e.map { it.name }.toSet().size)
    }

    // ------------------------------------------------------------ setup files
    @Test fun setupFiles() {
        val p = LinuxSetup.addUser("passwd", "root:x:0:0:root:/root:/bin/bash\n")
        assertTrue(p.endsWith("hearth:x:1000:1000:Hearth:/home/hearth:/bin/bash\n"))
        assertEquals(p, LinuxSetup.addUser("passwd", p))
        assertEquals("a\nhearth:x:1000:\n", LinuxSetup.addUser("group", "a"))
        assertEquals("en_US.UTF-8 UTF-8\n# de_DE.UTF-8 UTF-8\n", LinuxSetup.localeGen("# en_US.UTF-8 UTF-8\n# de_DE.UTF-8 UTF-8\n"))
        assertTrue(LinuxSetup.RESOLV.contains("nameserver 8.8.8.8") && LinuxSetup.RESOLV.contains("nameserver 1.1.1.1"))
        assertTrue(LinuxSetup.HOSTS.startsWith("127.0.0.1"))
    }

    @Test fun sourcesArePinned() {
        assertTrue(LinuxSources.ROOTFS_URLS.isNotEmpty() && LinuxSources.ROOTFS_URLS.all { it.startsWith("https://") && it.endsWith(".tar.xz") })
        assertTrue(LinuxSources.ROOTFS_SHA256.matches(Regex("[0-9a-f]{64}")))
    }
}
