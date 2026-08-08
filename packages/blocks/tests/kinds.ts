/* The kind list, read from the generated registry contract.

   `contract/blocks-registry.json` is what the TypeScript side is generated
   from, so a kind added to the registry appears here before any hand-written
   file knows it. Both rendering suites map over this list; neither carries
   its own copy. */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/* Like the corpus loader, read relative to this file: the JSX transform
   rewrites `import.meta.url`, but `import.meta.dirname` survives it. */
const REGISTRY_PATH = resolve(import.meta.dirname, '../../../contract/blocks-registry.json');

export interface RegistryKind {
  type: string;
  is_data: boolean;
}

export function loadRegistryKinds(): RegistryKind[] {
  const registry = JSON.parse(readFileSync(REGISTRY_PATH, 'utf8')) as { kinds: RegistryKind[] };
  return registry.kinds;
}
