#!/usr/bin/env python3
"""Hearth Player's page in Chromium (Playwright), on sample data instead of the app: playing a video and songs,
seeking, subtitles, a screenshot, trimming with undo, song tags, podcasts and radio. Screenshots go to the folder
given as the first argument. Needs: a folder of media made with ffmpeg (second argument; see player.yml).

    python3 tests/ui_test.py shots media
"""
import http.server
import json
import os
import pathlib
import sys
import threading
import urllib.parse

from playwright.sync_api import sync_playwright

HERE = pathlib.Path(__file__).resolve().parent.parent
SHOTS = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "shots")
MEDIA = pathlib.Path(sys.argv[2] if len(sys.argv) > 2 else "media").resolve()
SHOTS.mkdir(parents=True, exist_ok=True)


class Media(http.server.BaseHTTPRequestHandler):
    """Serves the sample media the way the app's own server does (byte ranges, CORS), under /tok/f/1/<name>."""
    def log_message(self, *a):
        pass

    def do_GET(self):
        name = urllib.parse.unquote(self.path.rsplit("/", 1)[-1])
        f = MEDIA / name
        if not f.is_file():
            self.send_response(404); self.end_headers(); return
        data = f.read_bytes()
        start, end = 0, len(data) - 1
        rng = self.headers.get("Range")
        if rng and rng.startswith("bytes="):
            a, _, b = rng[6:].partition("-")
            start = int(a or 0); end = int(b) if b else end
            self.send_response(206); self.send_header("Content-Range", f"bytes {start}-{end}/{len(data)}")
        else:
            self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Accept-Ranges", "bytes")
        self.send_header("Content-Type", "video/webm" if name.endswith(".webm") else "audio/ogg")
        self.send_header("Content-Length", str(end - start + 1))
        self.end_headers()
        try:
            self.wfile.write(data[start:end + 1])
        except (BrokenPipeError, ConnectionResetError):
            pass


srv = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Media)
threading.Thread(target=srv.serve_forever, daemon=True).start()
BASE = f"http://127.0.0.1:{srv.server_address[1]}/tok/f/1/"

RSS = """<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:itunes="http://www.itunes.com/dtds/podcast-1.0.dtd"><channel>
<title>Fireside Talks</title><itunes:author>Hearth Radio</itunes:author><itunes:image href=""/>
<description>&lt;p&gt;Stories by the fire.&lt;/p&gt;</description>
<item><title>Episode 2: Embers</title><guid>ep2</guid><pubDate>Tue, 20 Oct 2026 10:00:00 GMT</pubDate>
<enclosure url="MEDIA Song%20one.ogg" type="audio/ogg" length="56718"/><itunes:duration>00:30</itunes:duration></item>
<item><title>Episode 1: Kindling</title><guid>ep1</guid><pubDate>Mon, 12 Oct 2026 10:00:00 GMT</pubDate>
<enclosure url="MEDIA Song%20two.ogg" type="audio/ogg" length="56718"/><itunes:duration>30</itunes:duration></item>
</channel></rss>"""

STATIONS = [{"stationuuid": "u1", "name": "Ember FM", "url": "MEDIA Song%20one.ogg", "url_resolved": "MEDIA Song%20one.ogg",
             "favicon": "", "country": "Ireland", "tags": "folk,celtic", "codec": "OGG", "bitrate": 128},
            {"stationuuid": "u2", "name": "Kiln Jazz", "url": "x", "url_resolved": "x", "favicon": "", "country": "Japan", "tags": "jazz", "codec": "MP3", "bitrate": 192}]

RSS = RSS.replace("MEDIA ", BASE)
STATIONS = json.loads(json.dumps(STATIONS).replace("MEDIA ", BASE))
DEMO = """
window.__demo = {
  base: %(base)s,
  files: [{name: 'Sample clip.webm', kind: 'video', size: 539710}, {name: 'Song one.ogg', kind: 'audio', size: 56718}, {name: 'Song two.ogg', kind: 'audio', size: 56718}],
  probe: p => /webm$/.test(p) ? {info: {duration: 12, streams: [{index: 0, kind: 'video', codec: 'vp9'}, {index: 1, kind: 'audio', codec: 'opus', label: 'English (OPUS stereo)'}, {index: 2, kind: 'audio', codec: 'opus', label: 'French (OPUS stereo)'}],
      chapters: [{start: 0, end: 6, title: 'Opening'}, {start: 6, end: 12, title: 'The end'}]}, subs: [{path: '/home/you/Videos/Sample clip.en.srt', label: 'English (SRT file)', lang: 'en'}], tags: null}
    : {info: {duration: 30, streams: [{index: 0, kind: 'audio', codec: 'vorbis'}], chapters: []}, subs: [], tags: {title: 'Song one', artist: 'Hearth Band', album: 'Embers', year: 2026}},
  net: u => {
    if (u.includes('itunes.apple.com')) return JSON.stringify({results: [{collectionName: 'Fireside Talks', artistName: 'Hearth Radio', feedUrl: 'https://example.com/feed.xml', artworkUrl600: ''}]});
    if (u.includes('feed.xml')) return %(rss)s;
    if (u.includes('radio-browser')) return %(stations)s;
    throw 'offline';
  },
};
""" % {"base": json.dumps(BASE), "rss": json.dumps(RSS), "stations": json.dumps(json.dumps(STATIONS))}

errors = []


def check(cond, what):
    if not cond:
        raise AssertionError(what)
    print("ok:", what)


with sync_playwright() as p:
    b = p.chromium.launch(args=["--autoplay-policy=no-user-gesture-required"])
    pg = b.new_page(viewport={"width": 1280, "height": 760})
    pg.on("pageerror", lambda e: errors.append(str(e)))
    pg.add_init_script(DEMO)
    pg.goto((HERE / "ui" / "index.html").as_uri())
    pg.wait_for_function("document.body.dataset.ready === '1'")
    pg.screenshot(path=str(SHOTS / "empty.png"))

    # Open a video from the Open view.
    pg.click("[data-view=open]")
    pg.wait_for_selector("#files .row")
    pg.screenshot(path=str(SHOTS / "open.png"))
    pg.click("#files .row:has-text('Sample clip.webm')")
    pg.wait_for_function("document.getElementById('video').currentTime > 1", timeout=15000)
    check(pg.evaluate("window.__demoAwake.includes('video')"), "keeps the screen awake while a video plays")
    check(pg.evaluate("window.__demoNowPlaying && window.__demoNowPlaying.status === 'Playing'"), "tells MPRIS it's playing")
    pg.click("body", position={"x": 5, "y": 5}) if False else None
    pg.evaluate("document.activeElement && document.activeElement.blur()")
    t0 = pg.evaluate("document.getElementById('video').currentTime")
    pg.keyboard.press("ArrowRight")
    t1 = pg.evaluate("document.getElementById('video').currentTime")
    check(t1 - t0 > 3, f"right arrow seeks forward ({t0:.1f} -> {t1:.1f})")
    pg.keyboard.press("v")
    pg.wait_for_function("[...document.getElementById('video').textTracks].some(t => t.mode === 'showing')")
    check(True, "V turns subtitles on")
    check(pg.is_visible("#chapBtn") and pg.is_visible("#audioBtn"), "chapters and sound-track buttons show for a file with them")
    pg.keyboard.press("PageDown")
    check(pg.evaluate("document.getElementById('video').currentTime") >= 6, "Page Down jumps to the next chapter")
    pg.keyboard.press("k")  # pause
    pg.evaluate("document.getElementById('video').currentTime = 2")
    pg.wait_for_timeout(400)
    pg.screenshot(path=str(SHOTS / "video.png"))
    pg.keyboard.press("s")
    pg.wait_for_function("(window.__demoCalls||[]).some(c => c[0] === 'save_frame')")
    shot = [c for c in pg.evaluate("window.__demoCalls") if c[0] == "save_frame"][0][1]
    check(shot["png"] and shot["png"].startswith("data:image/png"), "S saves the frame as a PNG drawn by the page")

    # Trim with undo, then save as a copy.
    pg.keyboard.press("e")
    pg.wait_for_selector("#editPanel:not([hidden]) [data-ed=setStart]")
    pg.evaluate("document.getElementById('video').currentTime = 3")
    pg.wait_for_timeout(200)
    pg.click("[data-ed=setStart]")
    pg.evaluate("document.getElementById('video').currentTime = 9")
    pg.wait_for_timeout(200)
    pg.click("[data-ed=setEnd]")
    pg.click("[data-ed=rotR]")
    check(pg.evaluate("getComputedStyle(document.getElementById('video')).transform") != "none", "turning shows on the video straight away")
    pg.keyboard.press("Control+z")
    check(pg.evaluate("E.s.rotate") == 0, "Ctrl+Z undoes the turn")
    pg.click("[data-ed=cutOn]")
    pg.screenshot(path=str(SHOTS / "edit-video.png"))
    pg.click("[data-ed=save]")
    pg.wait_for_selector("#saveName")
    pg.screenshot(path=str(SHOTS / "edit-save.png"))
    pg.click("[data-c=yes]")
    pg.wait_for_selector("#toast:has-text('(edited)')")
    spec = [c for c in pg.evaluate("window.__demoCalls") if c[0] == "edit_start"][0][1]["edit"]
    check(abs(spec["start"] - 3) < 0.3 and abs(spec["end"] - 9) < 0.3 and spec["cutStart"] is not None and spec["output"].endswith("(edited).webm"), f"the edit asks for the marked part: {spec}")
    pg.keyboard.press("Escape")
    pg.wait_for_selector("#editPanel", state="hidden")

    # Songs: the music view, tags.
    pg.click("[data-view=open]")
    pg.click("#files .row:has-text('Song one.ogg')")
    pg.wait_for_selector("#music:not([hidden]) .trk")
    pg.wait_for_function("document.getElementById('video').currentTime > 0.5", timeout=15000)
    check(pg.evaluate("document.querySelectorAll('.trk').length") == 2, "the music view lists the folder's songs")
    check(pg.inner_text("#mArtist") == "Hearth Band", "the song's tags show")
    pg.screenshot(path=str(SHOTS / "music.png"))
    pg.keyboard.press("e")
    pg.wait_for_selector("#tg-title")
    pg.fill("#tg-title", "Song one (live)")
    pg.click("[data-num=fadeIn][data-d='0.5']")
    pg.screenshot(path=str(SHOTS / "edit-song.png"))
    pg.click("[data-ed=saveTags]")
    pg.wait_for_selector("#toast:has-text('details')")
    tw = [c for c in pg.evaluate("window.__demoCalls") if c[0] == "tags_write"][0][1]
    check(tw["tags"]["title"] == "Song one (live)", "saving details writes the new title")
    pg.click("[data-ed=close]")
    if pg.is_visible("[data-c=yes]"):
        pg.click("[data-c=yes]")

    # Podcasts: search, subscribe, play an episode; radio: list, favourite.
    pg.click("[data-view=pods]")
    pg.fill("#podSearch", "fireside")
    pg.keyboard.press("Enter")
    pg.wait_for_selector("[data-res]")
    pg.click("[data-res]")
    pg.wait_for_selector(".ep")
    check(pg.evaluate("document.querySelectorAll('.ep').length") == 2, "subscribing lists the feed's episodes")
    pg.screenshot(path=str(SHOTS / "podcast.png"))
    pg.click(".ep [data-pa=play]")
    pg.wait_for_function("document.getElementById('video').currentTime > 0.5", timeout=15000)
    check(pg.inner_text("#mArtist") == "Fireside Talks", "an episode plays with its show's name")
    pg.click("[data-view=pods]")
    pg.click("[data-pod=radio]")
    pg.wait_for_selector("[data-st]")
    pg.click("[data-fav='0'][data-src=res]")
    check(pg.evaluate("S.radio.favs.length") == 1, "starring a station keeps it in favourites")
    pg.screenshot(path=str(SHOTS / "radio.png"))
    stored = pg.evaluate("window.__demoStore")
    check("podcasts" in stored and stored["podcasts"]["shows"][0]["title"] == "Fireside Talks", "subscriptions are saved")

    # Controller-style moving around: arrows move focus in the Open view.
    pg.click("[data-view=open]")
    pg.wait_for_selector("#files .row")
    pg.keyboard.press("ArrowDown")
    check(pg.evaluate("document.activeElement.classList.contains('row') || document.activeElement.classList.contains('place')"), "arrow keys move between items")
    b.close()

if errors:
    print("page errors:", errors)
    sys.exit(1)
print("all good")
