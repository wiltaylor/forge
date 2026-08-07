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
