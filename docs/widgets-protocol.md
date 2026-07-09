# Forge Remote-Access Widgets — Wire Protocol

Version: 1 — **Rust-only extensions. NOT part of the frozen v1.0 API contract**
(`docs/api-contract.md` is unchanged by this document). These endpoints exist
only in `crates/forge-server`, only when the corresponding cargo feature is
compiled in, and only when the app opts in at runtime with a `with_*()`
builder or env flag.

| Endpoint            | Cargo feature | Runtime opt-in                          | Frontend widget  |
|---------------------|---------------|------------------------------------------|------------------|
| `WS /api/term`      | `term` (SSH: `term-ssh`) | `.with_term()` / `.with_term_from_env()` + `FORGE_TERM_ENABLE` | `@forge/term`    |
| `WS /api/desktop/vnc` | `vnc`       | `.with_vnc()` / `.with_vnc_from_env()` + `FORGE_VNC_ENABLE`   | `@forge/desktop` |
| `WS /api/desktop/rdp` | `rdp`       | `.with_rdp()` / `.with_rdp_from_env()` + `FORGE_RDP_ENABLE`   | `@forge/desktop` |

`widgets` is a convenience feature enabling all four. The server's `default`
features do **not** include any of them.

## Safety model — read this first

- **`/api/term` is RCE by design.** A `local` session hands every
  authenticated user a real shell running as the server's uid. An `ssh`
  session makes the server open outbound SSH connections with
  caller-supplied credentials. Enable it only in trusted dev contexts, behind
  auth, never on a shared or internet-facing deployment.
- **VNC/RDP are SSRF-shaped.** The server opens outbound TCP connections to a
  caller-supplied `host:port`. Mitigation: `DesktopConfig::allow_hosts`
  (env: `FORGE_DESKTOP_ALLOW_HOSTS`, comma-separated) restricts targets;
  unset means *any host*. `TermConfig::allow_hosts` does the same for SSH.
- **Dev-permissive verifiers.** The SSH client accepts any host key
  (`check_server_key` → yes), and the RDP client's TLS upgrade uses
  ironrdp-tls's `NoCertificateVerification`. Neither authenticates the
  server end of the connection — fine on a lab network, unacceptable across
  untrusted networks.
- **Credentials in frames.** `start`/`connect` messages carry passwords in
  the clear inside the WebSocket. Run behind TLS in any real deployment, and
  never log client control frames.

## Authentication

The widget routes sit in the protected router group: with auth enabled, the
upgrade request must carry a valid JWT or it is rejected **before** the
WebSocket handshake (401). Browsers cannot set headers on `WebSocket`, so the
token travels as `?token=<jwt>` — `api.wsUrl('/api/term')` from
`@forge/client` builds exactly this. In auth-disabled mode the routes are
open, like every other endpoint.

## Framing (both protocols)

Every connection speaks two frame kinds:

- **JSON text frames** — control messages, tagged with `"type"`.
- **Binary frames** — payload: raw tty bytes on `/api/term`, RGBA rect
  frames on `/api/desktop/*`.

The first client frame must be the session-opening control message (`start` /
`connect`). Anything else — including a binary frame — draws
`{"type":"error", ...}` followed by a close. Unknown/malformed control frames
after session start are ignored. There is no reconnect protocol: when the
socket dies the session dies, and the client reconnects from scratch.

## `/api/term`

### Client → server

Binary frames are raw bytes written to the tty (keystrokes, pastes).

```jsonc
// first frame, exactly once
{"type": "start", "mode": "local" | "ssh",
 "host": "…", "port": 22, "username": "…", "password": "…",  // ssh only
 "cols": 80, "rows": 24}

{"type": "resize", "cols": 120, "rows": 40}
```

- `mode:"local"` — gated on `TermConfig::allow_local`. Spawns
  `TermConfig::shell` → `$SHELL` → `/bin/sh` on a PTY.
- `mode:"ssh"` — gated on `TermConfig::allow_ssh`, the `term-ssh` feature,
  and `allow_hosts`. Requires `host`, `username`, `password`
  (password auth only in v1). Default port 22.

### Server → client

Binary frames are raw tty output.

```jsonc
{"type": "ready"}                       // session established, tty is live
{"type": "exit", "code": 130}           // process/channel exit code, then close
{"type": "error", "message": "…"}      // fatal; always followed by close
```

## `/api/desktop/vnc` and `/api/desktop/rdp`

One wire format for both: the **backend** decodes the desktop protocol and
streams dumb RGBA rects; the widget only blits them and forwards input.

### Client → server (JSON only)

```jsonc
// first frame, exactly once
{"type": "connect", "host": "…", "port": 5900,
 "username": "…", "password": "…"}

{"type": "key",   "code": "KeyA", "key": "a", "down": true}
{"type": "mouse", "x": 10, "y": 20, "buttons": 1}
{"type": "wheel", "dx": 0, "dy": -102.5}
{"type": "cad"}
```

- `connect` — `host` required; `allow_hosts` gated. VNC: default port 5900,
  `password` optional (VncAuth when the server demands it), `username`
  ignored. RDP: default port 3389, `username` **and** `password` required;
  `"DOMAIN\\user"` selects an explicit domain.
- `key` — `code` is the layout-independent `KeyboardEvent.code`; `key`
  carries the produced character for the VNC Unicode-keysym path.
- `mouse` — framebuffer coordinates; `buttons` is the
  `PointerEvent.buttons` bitmask (1 left, 2 right, 4 middle). The backend
  maps to protocol button masks and diffs press/release.
- `wheel` — browser `deltaX`/`deltaY` sign convention; the backend converts
  (VNC: button-mask pulses, RDP: ±120 notches).
- `cad` — the backend synthesizes Ctrl+Alt+Del: presses in order, releases
  in reverse.

### Server → client

```jsonc
{"type": "ready",  "width": 1280, "height": 800}  // handshake done; actual size
{"type": "resize", "width": 640,  "height": 480}  // server-side resolution change
{"type": "error",  "message": "…"}                // fatal; followed by close
{"type": "closed"}                                 // remote ended the session
```

`ready`/`resize` carry the **server's actual** framebuffer size (RDP requests
1280×800 but the server may override).

### Binary rect frame

Every framebuffer update is one binary frame (`proto::encode_rect`):

| Offset | Size | Field                              |
|-------:|-----:|------------------------------------|
| 0      | 1    | version = `1`                      |
| 1      | 1    | encoding = `0` (raw RGBA)          |
| 2      | 2    | x (u16 LE)                         |
| 4      | 2    | y (u16 LE)                         |
| 6      | 2    | w (u16 LE)                         |
| 8      | 2    | h (u16 LE)                         |
| 10     | w·h·4| pixels, row-major RGBA             |

Clients must ignore frames with an unknown version; the encoding byte
reserves room for per-rect compression later.

## Known approximations (v1)

- **Keymap is US-layout v1.** `KeyboardEvent.code` → X11 keysym (VNC) /
  set-1 scancode (RDP) via static US tables. VNC printables use the produced
  `key` character (Latin-1 direct, otherwise `0x01000000 + codepoint`
  Unicode keysym), so non-US layouts mostly work on VNC; RDP scancodes are
  positional, so the remote OS's layout decides what a key types.
- **Terminal colors are an approximation.** `@forge/term` derives the xterm
  ANSI palette from the active Forge theme tokens — close, not colorimetric.
- **RDP reactivation is unsupported.** A server-initiated Deactivate-All
  (resolution renegotiation, some reconnect flows) yields
  `{"type":"error","message":"…reconnect…"}` — reconnect from the client.
- VNC uses **Raw + DesktopSize encodings only**: correctness over bandwidth;
  every update is a full raw RGBA rect.
