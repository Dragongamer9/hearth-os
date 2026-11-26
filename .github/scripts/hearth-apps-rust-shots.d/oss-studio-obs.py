#!/usr/bin/env python3
"""A pretend OBS for Hearth Studio's CI pictures (oss-studio.sh): obs-websocket v5 on 127.0.0.1:PORT, the real protocol
(the handshake, Hello with a password challenge, Identify checked, requests answered, events sent, the sound meters
20 times a second), with the board's scenes, sources, mixer, filters, transitions and outputs, and a picture of the
program made here. Requests that change something change it here and send OBS's event, as OBS does.
Usage: oss-studio-obs.py PORT PASSWORD"""
import base64, glob, hashlib, io, json, os, socket, struct, sys, threading, time

PORT, PASSWORD = int(sys.argv[1]), sys.argv[2]
SALT, CHALLENGE = base64.b64encode(os.urandom(16)).decode(), base64.b64encode(os.urandom(16)).decode()
GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
lock = threading.Lock()
started = time.time()

S = {
    "scenes": ["Game + camera", "Just the game", "Talking (camera big)", "Xbox from Capture Card", "Starting soon", "Be right back"],
    "scene": "Game + camera",
    # top first, as OBS's own list shows them (obs-websocket gives them bottom first)
    "items": [(1, "Overlay: title", "text_ft2_source_v2", True, True), (2, "Camera · round", "v4l2_input", True, True), (3, "Game window · Quad Mech", "pipewire-window-capture-source", True, False),
              (4, "Capture Card · Xbox", "v4l2_input", False, False), (5, "Whole screen · Nvidia PC", "pipewire-screen-capture-source", False, False), (6, "Hearth Overlay stats", "browser_source", True, False)],
    "audio": [["Game and desktop", "pulse_output_capture", -8.0, False, 0.238], ["Mic · USB mic", "pulse_input_capture", -3.0, False, 0.405], ["Capture Card", "v4l2_input", -9.0, True, 0.0],
              ["Trailer music", "ffmpeg_source", -22.0, False, 0.027], ["Discord call", "pulse_output_capture", -6.0, False, 0.0058]],
    "filters": [["Noise removal", "noise_suppress_filter_v2", True], ["Leveller", "compressor_filter", True]],
    "transitions": ["Cut", "Fade", "Slide", "Ember"], "transition": "Fade", "duration": 300,
    "stream": True, "record": True, "paused": False, "replay": True, "vcam": False, "studio": False,
}

def tc(secs):
    h, r = divmod(int(secs), 3600); m, s = divmod(r, 60)
    return "%02d:%02d:%02d.000" % (h, m, s)

def font(size, bold=False):
    from PIL import ImageFont
    for pat in ["/usr/share/fonts/rsms-inter*/*.tt*", "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans%s.ttf" % ("-Bold" if bold else ""), "/usr/share/fonts/**/*.ttf"]:
        for f in sorted(glob.glob(pat, recursive=True)):
            try:
                return ImageFont.truetype(f, size)
            except Exception:
                pass
    return ImageFont.load_default()

def picture():
    from PIL import Image, ImageDraw
    W, H = 1280, 720
    k = W / 1012.0
    im = Image.new("RGB", (W, H)); d = ImageDraw.Draw(im)
    stops = [(0.0, (0x1a, 0x2a, 0x3a)), (0.40, (0x4a, 0x5a, 0x6a)), (0.55, (0x8a, 0x6a, 0x4a)), (0.56, (0x2a, 0x20, 0x18)), (1.0, (0x14, 0x0f, 0x0c))]
    for y in range(H):
        t = y / H
        for (a, ca), (b, cb) in zip(stops, stops[1:]):
            if a <= t <= b:
                u = (t - a) / max(b - a, 1e-6)
                d.line([(0, y), (W, y)], fill=tuple(int(ca[i] + (cb[i] - ca[i]) * u) for i in range(3)))
                break
    top, hh = 250 * k, 110 * k
    pts = [(0, .7), (.1, .3), (.22, .6), (.35, .2), (.48, .55), (.62, .25), (.76, .6), (.9, .3), (1, .5), (1, 1), (0, 1)]
    d.polygon([(x * W, top + y * hh) for x, y in pts], fill=(0x2a, 0x22, 0x1c))
    d.polygon([(300 * k + 420 * k * .3, 330 * k), (300 * k + 420 * k * .7, 330 * k), (720 * k, 590 * k), (300 * k, 590 * k)], fill=(0x3a, 0x2a, 0x20))
    d.rounded_rectangle([420 * k, 380 * k, 590 * k, 500 * k], radius=int(16 * k), fill=(0x4a, 0x5a, 0x3a))
    d.rounded_rectangle([470 * k, 350 * k, 540 * k, 390 * k], radius=int(8 * k), fill=(0x3a, 0x4a, 0x2a))
    d.rounded_rectangle([530 * k, 362 * k, 640 * k, 374 * k], radius=int(6 * k), fill=(0x2a, 0x3a, 0x1a))
    # the camera bubble
    cx, cy, r = W - 20 * k - 115 * k, H - 20 * k - 115 * k, 115 * k
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(0x2a, 0x1f, 0x1a), outline=(0xff, 0x8a, 0x3d), width=int(4 * k))
    d.ellipse([cx - 34 * k, cy - 70 * k, cx + 34 * k, cy - 2 * k], fill=(0xc8, 0xa0, 0x88))
    d.pieslice([cx - 70 * k, cy + 10 * k, cx + 70 * k, cy + 150 * k], 180, 360, fill=(0x5a, 0x3a, 0x2a))
    # the HUD and overlays (they're OBS's sources: the title text, the stats)
    d.rounded_rectangle([20 * k, H - 34 * k, 200 * k, H - 20 * k], radius=int(7 * k), fill=(0x10, 0x10, 0x10), outline=(0x55, 0x55, 0x55))
    d.rounded_rectangle([20 * k, H - 34 * k, 20 * k + 180 * k * .72, H - 20 * k], radius=int(7 * k), fill=(0x9b, 0xe3, 0x6b))
    d.text((20 * k, H - 58 * k), "ARMOR 72 · QUAD MECH", fill="white", font=font(int(12 * k), True))
    d.rounded_rectangle([20 * k, 20 * k, 370 * k, 56 * k], radius=int(10 * k), fill=(0x16, 0x12, 0x10), outline=(0x44, 0x40, 0x3c))
    d.rounded_rectangle([32 * k, 28 * k, 52 * k, 48 * k], radius=int(6 * k), fill=(0xff, 0x8a, 0x3d))
    d.text((60 * k, 29 * k), "Quad Mech · devlog night", fill="white", font=font(int(13 * k), True))
    d.text((240 * k, 29 * k), "· made on Hearth OS", fill=(0xd8, 0xcf, 0xc8), font=font(int(13 * k)))
    d.rounded_rectangle([W - 120 * k, 20 * k, W - 20 * k, 52 * k], radius=int(10 * k), fill=(0x16, 0x12, 0x10), outline=(0x44, 0x40, 0x3c))
    d.text((W - 110 * k, 27 * k), "118 fps · 64°C", fill=(0xd8, 0xcf, 0xc8), font=font(int(12 * k)))
    b = io.BytesIO(); im.save(b, "JPEG", quality=80)
    return "data:image/jpg;base64," + base64.b64encode(b.getvalue()).decode()

PIC = picture()

def answer(rt, d):
    ok = lambda data=None: (True, data or {})
    if rt == "GetSceneList":
        n = len(S["scenes"])
        return ok({"currentProgramSceneName": S["scene"], "currentPreviewSceneName": None, "scenes": [{"sceneName": s, "sceneIndex": n - 1 - i} for i, s in enumerate(S["scenes"])]})
    if rt == "GetSceneItemList":
        return ok({"sceneItems": [{"sceneItemId": i, "sourceName": n, "inputKind": k, "sceneItemEnabled": on, "sceneItemLocked": lk, "sceneItemIndex": 5 - x} for x, (i, n, k, on, lk) in reversed(list(enumerate(S["items"])))]})
    if rt == "GetInputList":
        names = [(n, k) for _, n, k, _, _ in S["items"]] + [(a[0], a[1]) for a in S["audio"]]
        return ok({"inputs": [{"inputName": n, "inputKind": k} for n, k in names]})
    if rt == "GetInputVolume":
        for a in S["audio"]:
            if a[0] == d.get("inputName"):
                return ok({"inputVolumeDb": a[2], "inputVolumeMul": 10 ** (a[2] / 20)})
        return (False, 604)
    if rt == "GetInputMute":
        for a in S["audio"]:
            if a[0] == d.get("inputName"):
                return ok({"inputMuted": a[3]})
        return (False, 604)
    if rt == "GetSourceFilterList":
        return ok({"filters": [{"filterName": f[0], "filterKind": f[1], "filterEnabled": f[2], "filterIndex": i} for i, f in enumerate(S["filters"])]})
    if rt == "GetSceneTransitionList":
        return ok({"currentSceneTransitionName": S["transition"], "currentSceneTransitionDuration": S["duration"], "transitions": [{"transitionName": t} for t in S["transitions"]]})
    if rt == "GetStreamServiceSettings":
        return ok({"streamServiceType": "rtmp_common", "streamServiceSettings": {"service": "Twitch", "server": "auto"}})
    if rt == "GetVideoSettings":
        return ok({"baseWidth": 1920, "baseHeight": 1080, "outputWidth": 1920, "outputHeight": 1080, "fpsNumerator": 60, "fpsDenominator": 1})
    if rt == "GetRecordDirectory":
        return ok({"recordDirectory": os.path.join(os.environ.get("HOME", "/tmp"), "Videos/Studio")})
    if rt == "GetStudioModeEnabled":
        return ok({"studioModeEnabled": S["studio"]})
    if rt == "GetVirtualCamStatus":
        return ok({"outputActive": S["vcam"]})
    if rt == "GetReplayBufferStatus":
        return ok({"outputActive": S["replay"]})
    if rt == "GetStreamStatus":
        el = time.time() - started + 42 * 60 + 13
        return ok({"outputActive": S["stream"], "outputTimecode": tc(el), "outputBytes": int(el * 6.2e6 / 8), "outputSkippedFrames": 0, "outputTotalFrames": 152040 + int((time.time() - started) * 60), "outputCongestion": 0.0, "outputReconnecting": False})
    if rt == "GetRecordStatus":
        el = time.time() - started + 42 * 60 + 10
        return ok({"outputActive": S["record"], "outputPaused": S["paused"], "outputTimecode": tc(el), "outputBytes": int(18.4 * 1024 ** 3)})
    if rt == "GetStats":
        return ok({"activeFps": 60.0, "cpuUsage": 6.1, "memoryUsage": 512.0, "renderSkippedFrames": 0, "outputSkippedFrames": 0})
    if rt == "GetSourceScreenshot":
        return ok({"imageData": PIC})
    if rt == "GetProfileParameter":
        v = {"Mode": "Simple", "StreamEncoder": "nvenc"}.get(d.get("parameterName"), "")
        return ok({"parameterValue": v, "defaultParameterValue": v})
    return ok({})

def change(rt, d, send):
    if rt == "SetCurrentProgramScene" and d.get("sceneName") in S["scenes"]:
        S["scene"] = d["sceneName"]; send({"op": 5, "d": {"eventType": "CurrentProgramSceneChanged", "eventIntent": 4, "eventData": {"sceneName": S["scene"]}}})
    elif rt == "SetSceneItemEnabled":
        S["items"] = [(i, n, k, d["sceneItemEnabled"] if i == d.get("sceneItemId") else on, lk) for i, n, k, on, lk in S["items"]]
        send({"op": 5, "d": {"eventType": "SceneItemEnableStateChanged", "eventIntent": 128, "eventData": {"sceneName": S["scene"], "sceneItemId": d.get("sceneItemId"), "sceneItemEnabled": d["sceneItemEnabled"]}}})
    elif rt == "SetInputMute":
        for a in S["audio"]:
            if a[0] == d.get("inputName"):
                a[3] = d["inputMuted"]
        send({"op": 5, "d": {"eventType": "InputMuteStateChanged", "eventIntent": 8, "eventData": {"inputName": d.get("inputName"), "inputMuted": d["inputMuted"]}}})

def frame(op, payload):
    n = len(payload)
    h = bytes([0x80 | op]) + (bytes([n]) if n < 126 else (bytes([126]) + struct.pack(">H", n) if n < 65536 else bytes([127]) + struct.pack(">Q", n)))
    return h + payload

def read_frame(c):
    def exact(n):
        b = b""
        while len(b) < n:
            x = c.recv(n - len(b))
            if not x:
                raise EOFError
            b += x
        return b
    h = exact(2); op = h[0] & 15; n = h[1] & 127
    if n == 126: n = struct.unpack(">H", exact(2))[0]
    elif n == 127: n = struct.unpack(">Q", exact(8))[0]
    mask = exact(4) if h[1] & 128 else b"\0\0\0\0"
    data = bytes(b ^ mask[i % 4] for i, b in enumerate(exact(n)))
    return op, data

def serve(c):
    head = b""
    while b"\r\n\r\n" not in head:
        head += c.recv(4096)
    key = [l.split(":", 1)[1].strip() for l in head.decode().split("\r\n") if l.lower().startswith("sec-websocket-key")][0]
    acc = base64.b64encode(hashlib.sha1((key + GUID).encode()).digest()).decode()
    c.sendall(("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: %s\r\nSec-WebSocket-Protocol: obswebsocket.json\r\n\r\n" % acc).encode())
    def send(obj):
        with lock:
            c.sendall(frame(1, json.dumps(obj).encode()))
    send({"op": 0, "d": {"obsWebSocketVersion": "5.5.2", "rpcVersion": 1, "authentication": {"challenge": CHALLENGE, "salt": SALT}}})
    op, data = read_frame(c)
    ident = json.loads(data)
    secret = base64.b64encode(hashlib.sha256((PASSWORD + SALT).encode()).digest()).decode()
    want = base64.b64encode(hashlib.sha256((secret + CHALLENGE).encode()).digest()).decode()
    if ident.get("d", {}).get("authentication") != want:
        c.sendall(frame(8, struct.pack(">H", 4009) + b"Authentication failed."))
        print("pretend OBS: wrong password", flush=True)
        return
    send({"op": 2, "d": {"negotiatedRpcVersion": 1}})
    print("pretend OBS: Hearth Studio signed in", flush=True)
    alive = [True]
    def meters():
        k = 0
        while alive[0]:
            k += 1
            wob = 1.0 + 0.04 * ((k % 7) - 3) / 3
            send({"op": 5, "d": {"eventType": "InputVolumeMeters", "eventIntent": 65536, "eventData": {"inputs": [{"inputName": a[0], "inputLevelsMul": [[a[4] * 0.6, 0.0 if a[3] else min(1.0, a[4] * wob), a[4]]] * 2} for a in S["audio"]]}}})
            time.sleep(0.05)
    threading.Thread(target=meters, daemon=True).start()
    try:
        while True:
            op, data = read_frame(c)
            if op == 8:
                break
            if op != 1:
                continue
            m = json.loads(data)
            if m.get("op") != 6:
                continue
            d = m["d"]; rt = d.get("requestType"); rd = d.get("requestData") or {}
            change(rt, rd, send)
            r = answer(rt, rd)
            status = {"result": True, "code": 100} if r[0] else {"result": False, "code": r[1], "comment": "The specified input does not support audio."}
            out = {"requestType": rt, "requestId": d.get("requestId"), "requestStatus": status}
            if r[0]:
                out["responseData"] = r[1]
            send({"op": 7, "d": out})
    except (EOFError, OSError):
        pass
    alive[0] = False

srv = socket.socket(); srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1); srv.bind(("127.0.0.1", PORT)); srv.listen(4)
print("pretend OBS on %d" % PORT, flush=True)
while True:
    c, _ = srv.accept()
    threading.Thread(target=serve, args=(c,), daemon=True).start()
