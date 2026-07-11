# forge-egui

The Forge design system for native desktop UIs: an egui widget kit, a
token-exact theme, an optional app runtime over eframe, and (behind cargo
features) the streaming widgets — embedded terminal, VNC and RDP viewers —
driven by `forge-core`'s engines entirely in-process.

See the [repo README](../../README.md#desktop-uis-forge-egui) for the full
tour, `examples/egui-gallery` for the living widget catalogue
(`just egui-gallery`), and `examples/egui-demo` for a complete native app on
the forge-core doc store / actions / event bus (`just egui-demo`).

## Features

| feature    | adds                                                          |
| ---------- | ------------------------------------------------------------- |
| `fonts` *(default)* | embedded IBM Plex Sans + JetBrains Mono (SIL OFL)     |
| `images`   | image loading (Avatar sources, link cards)                     |
| `markdown` | `Markdown` renderer (pulldown-cmark)                           |
| `chat`     | chat kit — transcripts, tool calls, prompts, composer (implies `markdown`) |
| `code`     | `CodeView`/`DiffView` with syntect highlighting + annotations  |
| `calendar` | `Calendar`/`DatePicker` (time crate)                           |
| `full`     | all of the above                                               |
| `term`     | embedded terminal over forge-core's PTY engine                 |
| `term-ssh` | SSH sessions for the terminal                                  |
| `vnc` / `rdp` | remote-desktop viewers over forge-core's engines            |
| `widgets`  | `term` + `term-ssh` + `vnc` + `rdp`                            |
| `wgpu`     | eframe wgpu renderer instead of the default glow               |

With `default-features = false` the crate is UI-only and tokio-free; egui's
built-in fonts are used.

## Fonts

The `fonts` feature embeds IBM Plex Sans (Regular/Medium/SemiBold) and
JetBrains Mono (Regular/Bold), both licensed under the SIL Open Font License
1.1 — full texts in [`LICENSES/`](LICENSES/). Named families
`plex-sans-medium`, `plex-sans-semibold`, and `jetbrains-mono-bold` carry the
extra weights (egui allows one weight per family); pick them via
`Theme::font(ctx, FontWeight::…, size)`.
