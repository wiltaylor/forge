#!/usr/bin/env python3
"""Build the seven probe substrates for wayfinder #75.

Two families:
  forge-*  the target repo is already a Forge app — hand-written components
           carrying Forge token and class names, as guidance-only implies.
  plain-*  no occurrence of the string "forge" anywhere.
"""
import pathlib, shutil, sys, re

# Deliberately NOT under the scratchpad: that path contains the string "forge",
# which an agent sees as its cwd and which would contaminate every negative probe.
ROOT = pathlib.Path("/tmp/claude-1000/wf/substrates")


def w(base, rel, text):
    p = base / rel
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(text.lstrip("\n"))


# ---------------------------------------------------------------- forge-solid
def forge_solid(b):
    w(b, "package.json", """
{
  "name": "opsview-web",
  "private": true,
  "type": "module",
  "scripts": { "dev": "vite", "build": "vite build" },
  "dependencies": { "solid-js": "^1.9.3" },
  "devDependencies": { "vite": "^6.0.3", "vite-plugin-solid": "^2.11.0", "typescript": "^5.7.2" }
}
""")
    w(b, "vite.config.ts", """
import { defineConfig } from 'vite';
import solid from 'vite-plugin-solid';

export default defineConfig({ plugins: [solid()] });
""")
    w(b, "index.html", """
<!doctype html>
<html lang="en">
  <head><meta charset="utf-8" /><title>opsview</title></head>
  <body><div id="root"></div><script type="module" src="/src/main.tsx"></script></body>
</html>
""")
    w(b, "src/main.tsx", """
import { render } from 'solid-js/web';
import App from './App';
import './forge/tokens.css';
import './forge/base.css';
import './forge/button.css';

render(() => <App />, document.getElementById('root')!);
""")
    w(b, "src/App.tsx", """
import { For } from 'solid-js';
import { Button } from './forge/Button';

const services = [
  { name: 'ingest', region: 'us-east-1', status: 'ok' },
  { name: 'indexer', region: 'eu-west-1', status: 'degraded' },
];

export default function App() {
  return (
    <div class="app-shell">
      <header class="app-bar">
        <span class="eyebrow">opsview</span>
        <Button variant="primary">Deploy</Button>
      </header>
      <main class="page">
        <h1 class="page-title">Services</h1>
        <ul class="stack">
          <For each={services}>
            {(s) => (
              <li class="row">
                <span>{s.name}</span>
                <span class="muted">{s.region}</span>
                <span class={`fbadge fbadge-${s.status === 'ok' ? 'ok' : 'warn'}`}>{s.status}</span>
              </li>
            )}
          </For>
        </ul>
      </main>
    </div>
  );
}
""")
    w(b, "src/forge/tokens.css", """
:root {
  --bg-0: #0B0D10;
  --bg-1: #11141A;
  --bg-2: #171B22;
  --border: #262C36;
  --fg-0: #E6EAF0;
  --fg-1: #A8B2C0;
  --fg-2: #6B7688;
  --accent: oklch(0.62 0.16 250);
  --accent-contrast: #FFFFFF;
  --ok: #3FB950;
  --warn: #D29922;
  --danger: #F85149;

  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --control-h: 32px;
  --radius: 4px;
  --dur-fast: 120ms;
}
""")
    w(b, "src/forge/base.css", """
* { box-sizing: border-box; }

body {
  margin: 0;
  background: var(--bg-0);
  color: var(--fg-0);
  font: 13px/1.5 ui-sans-serif, system-ui, sans-serif;
}

.app-shell { min-height: 100vh; display: flex; flex-direction: column; }

.app-bar {
  display: flex; align-items: center; justify-content: space-between;
  height: 48px; padding: 0 var(--space-4);
  background: var(--bg-1); border-bottom: 1px solid var(--border);
}

.page { padding: var(--space-4); }
.page-title { font-size: 15px; font-weight: 600; margin: 0 0 var(--space-3); }
.eyebrow { font-size: 11px; letter-spacing: .08em; text-transform: uppercase; color: var(--fg-2); }
.muted { color: var(--fg-1); }
.stack { list-style: none; margin: 0; padding: 0; }

.row {
  display: grid; grid-template-columns: 1fr 1fr auto; gap: var(--space-3);
  align-items: center; height: var(--control-h);
  border-bottom: 1px solid var(--border);
}

.fbadge { padding: 0 var(--space-2); border-radius: var(--radius); font-size: 11px; }
.fbadge-ok { color: var(--ok); background: color-mix(in oklab, var(--ok) 14%, transparent); }
.fbadge-warn { color: var(--warn); background: color-mix(in oklab, var(--warn) 14%, transparent); }
""")
    w(b, "src/forge/Button.tsx", """
import { JSX, splitProps } from 'solid-js';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';

export function Button(props: { variant?: Variant } & JSX.ButtonHTMLAttributes<HTMLButtonElement>) {
  const [local, rest] = splitProps(props, ['variant', 'class', 'children']);
  return (
    <button class={`fbtn fbtn-${local.variant ?? 'secondary'} ${local.class ?? ''}`} {...rest}>
      {local.children}
    </button>
  );
}
""")
    w(b, "src/forge/button.css", """
.fbtn {
  height: var(--control-h);
  padding: 0 var(--space-3);
  display: inline-flex; align-items: center; gap: var(--space-2);
  border: 1px solid var(--border); border-radius: var(--radius);
  background: var(--bg-2); color: var(--fg-0);
  font: inherit; cursor: pointer;
  transition: background var(--dur-fast), border-color var(--dur-fast);
}

.fbtn:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }
.fbtn:disabled { opacity: .5; cursor: not-allowed; }
.fbtn-primary { background: var(--accent); color: var(--accent-contrast); border-color: transparent; }
.fbtn-ghost { background: transparent; border-color: transparent; }
.fbtn-danger { background: var(--danger); color: #fff; border-color: transparent; }
""")
    # a plain axum server alongside — the substrate for the X1 cross-check
    w(b, "server/Cargo.toml", """
[package]
name = "opsview-server"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
""")
    w(b, "server/src/main.rs", """
use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
struct Service {
    name: String,
    region: String,
    status: String,
}

async fn services() -> Json<Vec<Service>> {
    Json(vec![Service {
        name: "ingest".into(),
        region: "us-east-1".into(),
        status: "ok".into(),
    }])
}

#[tokio::main]
async fn main() {
    let app = Router::new().route("/api/services", get(services));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
""")
    w(b, "README.md", """
# opsview

Operations console. `src/` is the SolidJS front end, `server/` the axum API.

The UI components under `src/forge/` are written in this repo against the Forge
design system — there is no package to install.
""")


# -------------------------------------------------------------- forge-ratatui
def forge_ratatui(b):
    w(b, "Cargo.toml", """
[package]
name = "opsview-tui"
version = "0.1.0"
edition = "2021"

[dependencies]
ratatui = "0.30"
crossterm = "0.29"
""")
    w(b, "src/main.rs", """
mod forge;
mod screens;

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let res = screens::dashboard::run(&mut terminal);
    ratatui::restore();
    res
}
""")
    w(b, "src/forge/mod.rs", """
pub mod theme;

pub use theme::Theme;
""")
    w(b, "src/forge/theme.rs", """
use ratatui::style::Color;

/// Forge colour roles, as written for this app.
pub struct Theme {
    pub bg_0: Color,
    pub bg_1: Color,
    pub bg_2: Color,
    pub border: Color,
    pub fg_0: Color,
    pub fg_1: Color,
    pub fg_2: Color,
    pub accent: Color,
    pub accent_contrast: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_0: Color::Rgb(0x0B, 0x0D, 0x10),
            bg_1: Color::Rgb(0x11, 0x14, 0x1A),
            bg_2: Color::Rgb(0x17, 0x1B, 0x22),
            border: Color::Rgb(0x26, 0x2C, 0x36),
            fg_0: Color::Rgb(0xE6, 0xEA, 0xF0),
            fg_1: Color::Rgb(0xA8, 0xB2, 0xC0),
            fg_2: Color::Rgb(0x6B, 0x76, 0x88),
            accent: Color::Rgb(0x4C, 0x8D, 0xFF),
            accent_contrast: Color::Rgb(0xFF, 0xFF, 0xFF),
            ok: Color::Rgb(0x3F, 0xB9, 0x50),
            warn: Color::Rgb(0xD2, 0x99, 0x22),
            danger: Color::Rgb(0xF8, 0x51, 0x49),
        }
    }
}
""")
    w(b, "src/screens/mod.rs", """
pub mod dashboard;
""")
    w(b, "src/screens/dashboard.rs", """
use crate::forge::Theme;
use ratatui::{
    layout::{Constraint, Layout},
    style::Style,
    widgets::{Block, Borders, Paragraph, Row, Table},
    DefaultTerminal,
};

pub fn run(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let theme = Theme::dark();
    terminal.draw(|f| {
        let [bar, body] =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).areas(f.area());

        f.render_widget(
            Paragraph::new(" opsview ").style(Style::new().bg(theme.bg_1).fg(theme.fg_2)),
            bar,
        );

        let rows = [Row::new(vec!["ingest", "us-east-1", "ok"])];
        f.render_widget(
            Table::new(rows, [Constraint::Fill(1); 3])
                .block(Block::new().borders(Borders::ALL).border_style(Style::new().fg(theme.border)))
                .style(Style::new().bg(theme.bg_0).fg(theme.fg_0)),
            body,
        );
    })?;
    Ok(())
}
""")
    w(b, "README.md", """
# opsview-tui

Terminal front end for opsview. Widgets under `src/forge/` are written in this
repo against the Forge design system.
""")


# ----------------------------------------------------------------- forge-egui
def forge_egui(b):
    w(b, "Cargo.toml", """
[package]
name = "opsview-desktop"
version = "0.1.0"
edition = "2021"

[dependencies]
eframe = "0.30"
egui = "0.30"
""")
    w(b, "src/main.rs", """
mod forge;

use forge::Theme;

struct App {
    theme: Theme,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.theme.apply(ctx);
        egui::TopBottomPanel::top("bar").show(ctx, |ui| {
            ui.label("opsview");
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Services");
            ui.label("ingest — us-east-1 — ok");
        });
    }
}

fn main() -> eframe::Result {
    eframe::run_native(
        "opsview",
        eframe::NativeOptions::default(),
        Box::new(|_cc| Ok(Box::new(App { theme: Theme::dark() }))),
    )
}
""")
    w(b, "src/forge/mod.rs", """
pub mod theme;

pub use theme::Theme;
""")
    w(b, "src/forge/theme.rs", """
use egui::Color32;

/// Forge colour roles, as written for this app.
pub struct Theme {
    pub bg_0: Color32,
    pub bg_1: Color32,
    pub bg_2: Color32,
    pub border: Color32,
    pub fg_0: Color32,
    pub fg_1: Color32,
    pub fg_2: Color32,
    pub accent: Color32,
    pub accent_contrast: Color32,
    pub ok: Color32,
    pub warn: Color32,
    pub danger: Color32,
}

impl Theme {
    pub fn dark() -> Self {
        Self {
            bg_0: Color32::from_rgb(0x0B, 0x0D, 0x10),
            bg_1: Color32::from_rgb(0x11, 0x14, 0x1A),
            bg_2: Color32::from_rgb(0x17, 0x1B, 0x22),
            border: Color32::from_rgb(0x26, 0x2C, 0x36),
            fg_0: Color32::from_rgb(0xE6, 0xEA, 0xF0),
            fg_1: Color32::from_rgb(0xA8, 0xB2, 0xC0),
            fg_2: Color32::from_rgb(0x6B, 0x76, 0x88),
            accent: Color32::from_rgb(0x4C, 0x8D, 0xFF),
            accent_contrast: Color32::WHITE,
            ok: Color32::from_rgb(0x3F, 0xB9, 0x50),
            warn: Color32::from_rgb(0xD2, 0x99, 0x22),
            danger: Color32::from_rgb(0xF8, 0x51, 0x49),
        }
    }

    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = self.bg_0;
        visuals.window_fill = self.bg_1;
        ctx.set_visuals(visuals);
    }
}
""")
    w(b, "README.md", """
# opsview-desktop

Native desktop front end for opsview. Widgets under `src/forge/` are written in
this repo against the Forge design system.
""")


# ---------------------------------------------------------------- forge-tauri
def forge_tauri(b):
    forge_solid(b)
    shutil.rmtree(b / "server")
    w(b, "package.json", """
{
  "name": "opsview-desktop",
  "private": true,
  "type": "module",
  "scripts": { "dev": "vite", "build": "vite build", "tauri": "tauri" },
  "dependencies": { "solid-js": "^1.9.3", "@tauri-apps/api": "^2.1.1" },
  "devDependencies": {
    "@tauri-apps/cli": "^2.1.0",
    "vite": "^6.0.3",
    "vite-plugin-solid": "^2.11.0",
    "typescript": "^5.7.2"
  }
}
""")
    w(b, "src-tauri/Cargo.toml", """
[package]
name = "opsview-desktop"
version = "0.1.0"
edition = "2021"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = [] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
""")
    w(b, "src-tauri/tauri.conf.json", """
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "opsview",
  "version": "0.1.0",
  "identifier": "dev.opsview.desktop",
  "build": { "frontendDist": "../dist", "devUrl": "http://localhost:5173" },
  "app": { "windows": [{ "title": "opsview", "width": 1200, "height": 800 }] },
  "bundle": { "active": true, "targets": "all" }
}
""")
    w(b, "src-tauri/src/main.rs", """
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running application");
}
""")
    w(b, "README.md", """
# opsview

Desktop build of the opsview console. SolidJS front end in `src/`, Tauri shell in
`src-tauri/`. UI components under `src/forge/` are written in this repo against
the Forge design system.
""")


# ---------------------------------------------------------------- plain-axum
def plain_axum(b):
    w(b, "Cargo.toml", """
[package]
name = "docsvc"
version = "0.1.0"
edition = "2021"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
uuid = { version = "1", features = ["v4", "serde"] }
""")
    w(b, "src/main.rs", """
mod routes;

use axum::Router;

#[tokio::main]
async fn main() {
    let app = Router::new().nest("/api", routes::router());
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
""")
    w(b, "src/routes/mod.rs", """
pub mod health;

use axum::{routing::get, Router};

pub fn router() -> Router {
    Router::new().route("/health", get(health::health))
}
""")
    w(b, "src/routes/health.rs", """
use axum::Json;
use serde_json::{json, Value};

pub async fn health() -> Json<Value> {
    Json(json!({ "status": "ok" }))
}
""")
    w(b, "README.md", """
# docsvc

A small HTTP service. Rust, axum, tokio.
""")


# ------------------------------------------------------------- plain-fastapi
def plain_fastapi(b):
    w(b, "pyproject.toml", """
[project]
name = "docsvc"
version = "0.1.0"
requires-python = ">=3.11"
dependencies = ["fastapi>=0.115", "uvicorn[standard]>=0.32", "pydantic>=2.9"]
""")
    w(b, "app/__init__.py", "")
    w(b, "app/main.py", """
from fastapi import FastAPI

from .routes import health

app = FastAPI(title="docsvc")
app.include_router(health.router, prefix="/api")
""")
    w(b, "app/routes/__init__.py", "")
    w(b, "app/routes/health.py", """
from fastapi import APIRouter

router = APIRouter()


@router.get("/health")
async def health() -> dict[str, str]:
    return {"status": "ok"}
""")
    w(b, "README.md", """
# docsvc

A small HTTP service. Python, FastAPI, uvicorn.
""")


# --------------------------------------------------------------- plain-react
def plain_react(b):
    w(b, "package.json", """
{
  "name": "accounts-web",
  "private": true,
  "type": "module",
  "scripts": { "dev": "vite", "build": "vite build" },
  "dependencies": { "react": "^18.3.1", "react-dom": "^18.3.1" },
  "devDependencies": { "vite": "^6.0.3", "@vitejs/plugin-react": "^4.3.4" }
}
""")
    w(b, "index.html", """
<!doctype html>
<html lang="en">
  <head><meta charset="utf-8" /><title>accounts</title></head>
  <body><div id="root"></div><script type="module" src="/src/main.jsx"></script></body>
</html>
""")
    w(b, "src/main.jsx", """
import { createRoot } from 'react-dom/client';
import App from './App';
import './index.css';

createRoot(document.getElementById('root')).render(<App />);
""")
    w(b, "src/App.jsx", """
export default function App() {
  return (
    <div className="app">
      <h1>Accounts</h1>
      <p className="muted">Nothing selected.</p>
    </div>
  );
}
""")
    w(b, "src/index.css", """
body { margin: 0; font-family: system-ui, sans-serif; color: #1a1a1a; background: #fff; }
.app { padding: 24px; }
.muted { color: #6b7280; }
""")
    w(b, "README.md", """
# accounts-web

Customer accounts front end. React and Vite.
""")


BUILDERS = {
    "forge-solid": forge_solid,
    "forge-ratatui": forge_ratatui,
    "forge-egui": forge_egui,
    "forge-tauri": forge_tauri,
    "plain-axum": plain_axum,
    "plain-fastapi": plain_fastapi,
    "plain-react": plain_react,
}

if __name__ == "__main__":
    if ROOT.exists():
        shutil.rmtree(ROOT)
    for name, fn in BUILDERS.items():
        b = ROOT / name
        b.mkdir(parents=True)
        fn(b)

    # Assert the plain-* family carries no Forge trace at all.
    bad = []
    for name in BUILDERS:
        if not name.startswith("plain-"):
            continue
        base = ROOT / name
        for f in base.rglob("*"):
            if not f.is_file():
                continue
            rel = str(f.relative_to(base))
            if re.search(r"forge", f.read_text() + rel, re.I):
                bad.append(rel)
    if bad:
        print("LEAK — 'forge' found in a plain substrate:", *bad, sep="\n  ")
        sys.exit(1)

    for name in BUILDERS:
        n = sum(1 for f in (ROOT / name).rglob("*") if f.is_file())
        print(f"{name:16} {n:3} files")
    print("\nplain-* substrates verified free of 'forge'.")
