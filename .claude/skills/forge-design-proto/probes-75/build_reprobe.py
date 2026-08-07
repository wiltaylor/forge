#!/usr/bin/env python3
"""Substrates for the P4 and P9 re-probes.

The first run voided both: the thin forge-solid skeleton held no alerts panel and
no second config version, so the agent correctly reported the referent missing and
never reached a routing decision. These two substrates supply the referent and
change nothing else.
"""
import pathlib, shutil, sys

sys.path.insert(0, str(pathlib.Path(__file__).parent))
from build_substrates import forge_solid, w  # noqa: E402

ROOT = pathlib.Path("/tmp/claude-1000/wf/substrates")


def forge_solid_alerts(b):
    """forge-solid plus an alerts panel that is visibly off-style."""
    forge_solid(b)
    w(b, "src/AlertsPanel.tsx", """
import { For } from 'solid-js';

const alerts = [
  { id: 1, level: 'critical', text: 'ingest lag above 90s', at: '12:41' },
  { id: 2, level: 'warning', text: 'indexer queue depth rising', at: '12:38' },
  { id: 3, level: 'info', text: 'nightly compaction finished', at: '03:02' },
];

export function AlertsPanel() {
  return (
    <div style={{ background: '#ffffff', border: '1px solid #ccc', padding: '20px', 'border-radius': '10px' }}>
      <h2 style={{ 'font-family': 'Georgia, serif', 'font-size': '22px', color: '#333' }}>Alerts</h2>
      <For each={alerts}>
        {(a) => (
          <div style={{ padding: '14px 0', 'border-bottom': '1px dotted #999' }}>
            <span style={{ color: a.level === 'critical' ? 'red' : a.level === 'warning' ? 'orange' : 'gray',
                           'font-weight': 'bold', 'text-transform': 'uppercase' }}>
              {a.level}
            </span>
            <span style={{ 'margin-left': '12px', color: '#555' }}>{a.text}</span>
            <span style={{ float: 'right', color: '#aaa' }}>{a.at}</span>
          </div>
        )}
      </For>
      <button style={{ 'margin-top': '16px', background: '#0066cc', color: 'white',
                       border: 'none', padding: '10px 18px', 'border-radius': '20px' }}>
        Acknowledge all
      </button>
    </div>
  );
}
""")
    app = b / "src/App.tsx"
    t = app.read_text()
    t = t.replace("import { Button } from './forge/Button';",
                  "import { Button } from './forge/Button';\nimport { AlertsPanel } from './AlertsPanel';")
    t = t.replace("      </main>", "        <AlertsPanel />\n      </main>")
    app.write_text(t)


def forge_solid_configs(b):
    """forge-solid plus two versions of a deployment config the UI can compare."""
    forge_solid(b)
    w(b, "src/config/versions.ts", """
export type ConfigVersion = { version: string; savedAt: string; body: string };

export const versions: ConfigVersion[] = [
  {
    version: 'v14',
    savedAt: '2026-08-03T09:12:00Z',
    body: `region = "us-east-1"
replicas = 3
timeout_ms = 2000
log_level = "info"
features = ["ingest", "index"]
`,
  },
  {
    version: 'v15',
    savedAt: '2026-08-06T17:40:00Z',
    body: `region = "us-east-1"
replicas = 6
timeout_ms = 3500
log_level = "debug"
features = ["ingest", "index", "replay"]
retention_days = 30
`,
  },
];
""")
    w(b, "src/ConfigPage.tsx", """
import { For } from 'solid-js';
import { versions } from './config/versions';

export function ConfigPage() {
  return (
    <section>
      <h1 class="page-title">Deployment config</h1>
      <ul class="stack">
        <For each={versions}>
          {(v) => (
            <li class="row">
              <span>{v.version}</span>
              <span class="muted">{v.savedAt}</span>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}
""")


if __name__ == "__main__":
    for name, fn in (("forge-solid-alerts", forge_solid_alerts),
                     ("forge-solid-configs", forge_solid_configs)):
        b = ROOT / name
        if b.exists():
            shutil.rmtree(b)
        b.mkdir(parents=True)
        fn(b)
        print(f"{name:22} {sum(1 for f in b.rglob('*') if f.is_file()):3} files")
