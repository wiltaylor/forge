// Serialized `prepare` for the workspace packages: runs tsup under an atomic
// lock. pnpm's git-dep install runs a package's prepare TWICE — once as a
// workspace project during the fetched repo's install, once at pack time —
// and on slow machines the two overlap in the same directory; with tsup's
// `clean: true` the second run deletes the first's output mid-write and the
// packed dist/ ships truncated (seen as a 294-byte @forge/ui index.d.ts in
// vmlab's CI). The loser of the mkdir race waits for the winner and reuses
// its output instead of rebuilding.
import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, rmSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';

const lock = '.prepare-lock';
const WAIT_MS = 500;
const MAX_WAIT_MS = 5 * 60 * 1000; // stale-lock backstop: build anyway

function acquire() {
  try {
    mkdirSync(lock);
    return true;
  } catch {
    return false;
  }
}

if (!acquire()) {
  let waited = 0;
  while (existsSync(lock) && waited < MAX_WAIT_MS) {
    await sleep(WAIT_MS);
    waited += WAIT_MS;
  }
  if (existsSync(lock)) {
    // Stale lock (crashed holder): steal it and build.
    rmSync(lock, { recursive: true, force: true });
    if (!acquire()) process.exit(0);
  } else {
    // The concurrent prepare finished; its dist/ is the build.
    process.exit(0);
  }
}

// NOTE: process.exit() skips finally blocks — release the lock explicitly
// before exiting, and also on signals/unexpected exits.
process.on('exit', () => rmSync(lock, { recursive: true, force: true }));
for (const sig of ['SIGINT', 'SIGTERM']) process.on(sig, () => process.exit(1));

const r = spawnSync('tsup', [], { stdio: 'inherit', shell: true });
process.exit(r.status ?? 1);
