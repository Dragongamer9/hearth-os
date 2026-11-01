// A local test page for the viewer's real pointer (apps-create.yml): the page (127.0.0.1:<port>) holds a frame
// from another site (localhost:<port + 1>, so Chromium puts it in its own process, like a captcha's or a player's)
// with a button that only takes real clicks (event.isTrusted). A real click there turns the page's title into
// "trusted-click-ok"; scripted clicks (which the frame tries on itself) change nothing. Nothing here talks to a
// real site.
//   node tests/pointer.mjs 8770
import http from 'node:http';

const port = +process.argv[2] || 8770;

const top = `<!doctype html><html><head><meta charset="utf-8"><title>Pointer test</title></head>
<body style="margin:0;background:#1d2733;color:#fff;font:18px sans-serif;height:100vh">
<p style="position:absolute;left:200px;top:140px;margin:0">The frame below is another site; its button only takes real clicks.</p>
<iframe id="f" src="http://localhost:${port + 1}/frame" style="position:absolute;left:200px;top:200px;width:500px;height:300px;border:0;background:#fff"></iframe>
<script>
  addEventListener('message', e => {
    if (e.origin !== 'http://localhost:${port + 1}') return;
    if (e.data === 'trusted-click') document.title = 'trusted-click-ok';
  });
</script></body></html>`;

const frame = `<!doctype html><html><head><meta charset="utf-8"></head>
<body style="margin:0;background:#f4ece3;font:18px sans-serif">
<button id="b" style="position:absolute;left:100px;top:100px;width:200px;height:80px;font-size:20px">I'm not a robot</button>
<script>
  const b = document.getElementById('b');
  b.addEventListener('click', e => {
    if (!e.isTrusted) { return; }
    b.textContent = 'Real click!'; parent.postMessage('trusted-click', '*');
  });
  // Scripted clicks must not count.
  b.click(); b.dispatchEvent(new MouseEvent('click', { bubbles: true }));
</script></body></html>`;

const serve = (body, host) => http.createServer((req, res) => {
  res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
  res.end(body);
}).listen(host === 'localhost' ? port + 1 : port);   // every address: localhost may be ::1

serve(top, '127.0.0.1');
serve(frame, 'localhost');
console.log(`pointer test page: http://127.0.0.1:${port}/ (frame http://localhost:${port + 1}/frame)`);
