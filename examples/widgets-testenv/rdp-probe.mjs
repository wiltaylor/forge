// Headless probe for the RDP widget WS endpoint: log in, connect to the
// testenv xrdp container, reassemble the rect frames into a framebuffer, and
// dump it as a PPM screenshot for visual inspection.
//
// This is the regression check for the xrdp bitmap-stride corruption fixed in
// vendor/ironrdp-session (upstream Devolutions/IronRDP#1251): xrdp pads bitmap
// width to a multiple of 4, and an unpatched blit shears every tile
// diagonally. No unit test can cover it — it needs a real width-padded bitmap
// stream from xrdp — so re-run this after touching the RDP pipeline or the
// vendored crate:
//
//   just widgets-testenv-up
//   cd examples/rust-demo && FORGE_PORT=8766 cargo run -p rust-demo   # flags in .env
//   node examples/widgets-testenv/rdp-probe.mjs                      # needs Node >= 22
//   magick rdp-fb.ppm rdp-fb.png
//
// Good output: xrdp greeter with straight window borders and legible
// "Login to <host>" text. Sheared/garbled output means a stride regression.
import { writeFileSync } from 'node:fs';

const BASE = process.env.BASE ?? 'http://127.0.0.1:8766';
const OUT = process.env.OUT ?? 'rdp-fb.ppm';
const WAIT_MS = Number(process.env.WAIT_MS ?? 6000);

const login = await fetch(`${BASE}/api/auth/login`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ username: 'admin', password: 'admin' }),
});
if (!login.ok) throw new Error(`login failed: ${login.status} ${await login.text()}`);
const body = await login.json();
const token = body.data?.token ?? body.token;
if (!token) throw new Error(`no token in login response: ${JSON.stringify(body)}`);

const ws = new WebSocket(`${BASE.replace('http', 'ws')}/api/desktop/rdp?token=${encodeURIComponent(token)}`);
ws.binaryType = 'arraybuffer';

let fb = null, fbw = 0, fbh = 0, rects = 0;

ws.onopen = () => {
  ws.send(JSON.stringify({ type: 'connect', host: '127.0.0.1', port: 3389, username: 'forge', password: 'forge' }));
};
ws.onmessage = (ev) => {
  if (typeof ev.data === 'string') {
    const msg = JSON.parse(ev.data);
    console.log('ctrl:', ev.data);
    if (msg.type === 'ready') {
      fbw = msg.width; fbh = msg.height;
      fb = new Uint8Array(fbw * fbh * 4);
    }
    return;
  }
  const buf = new Uint8Array(ev.data);
  const dv = new DataView(ev.data);
  const x = dv.getUint16(2, true), y = dv.getUint16(4, true);
  const w = dv.getUint16(6, true), h = dv.getUint16(8, true);
  rects++;
  if (rects <= 30) console.log(`rect ${rects}: x=${x} y=${y} w=${w} h=${h} bytes=${buf.length - 10}`);
  if (!fb) return;
  for (let row = 0; row < h; row++) {
    const src = 10 + row * w * 4;
    const dst = ((y + row) * fbw + x) * 4;
    fb.set(buf.subarray(src, src + w * 4), dst);
  }
};
ws.onclose = () => console.log('ws closed');
ws.onerror = (e) => console.log('ws error', e.message ?? '');

await new Promise((r) => setTimeout(r, WAIT_MS));
console.log(`total rects: ${rects}`);
if (fb) {
  const px = fbw * fbh;
  const rgb = Buffer.alloc(px * 3);
  for (let i = 0; i < px; i++) {
    rgb[i * 3] = fb[i * 4];
    rgb[i * 3 + 1] = fb[i * 4 + 1];
    rgb[i * 3 + 2] = fb[i * 4 + 2];
  }
  writeFileSync(OUT, Buffer.concat([Buffer.from(`P6\n${fbw} ${fbh}\n255\n`), rgb]));
  console.log(`wrote ${OUT} (${fbw}x${fbh})`);
}
ws.close();
process.exit(0);
