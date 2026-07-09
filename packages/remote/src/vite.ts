import type { Plugin, UserConfig } from 'vite';
import { createHash } from 'node:crypto';
import type { RemoteComponentMeta, RemoteManifest } from './types';

export interface ForgeRemoteConfigOptions {
  /** App name written into the manifest. */
  app: string;
  /** Entry module that calls defineRemoteElement for every exported component. */
  entry: string;
  /** Component metadata for the manifest (tags/props/events as registered by the entry). */
  components: Omit<RemoteComponentMeta, 'file' | 'hash'>[];
  /** Output directory (default 'dist-remote'). Serve it as the backend components dir. */
  outDir?: string;
  /** Bundle file name (default 'remote.js'). */
  fileName?: string;
  version?: string;
  /** Extra Vite plugins — pass vite-plugin-solid here. */
  plugins?: UserConfig['plugins'];
}

/**
 * Vite config factory for building a Forge remote-component bundle:
 * a single self-contained ES module (solid-js bundled in, CSS inlined via
 * `?inline` imports in your components) plus a `manifest.json`, laid out the
 * way `/api/components` serves them.
 */
export function forgeRemoteConfig(opts: ForgeRemoteConfigOptions): UserConfig {
  const outDir = opts.outDir ?? 'dist-remote';
  const fileName = opts.fileName ?? 'remote.js';
  return {
    plugins: [...(opts.plugins ?? []), manifestPlugin(opts, outDir, fileName)],
    resolve: {
      // one solid runtime inside the bundle
      dedupe: ['solid-js'],
    },
    build: {
      outDir,
      emptyOutDir: true,
      lib: {
        entry: opts.entry,
        formats: ['es'],
        fileName: () => fileName,
      },
      cssCodeSplit: false,
      rollupOptions: {
        output: { inlineDynamicImports: true },
      },
    },
  };
}

function manifestPlugin(opts: ForgeRemoteConfigOptions, _outDir: string, fileName: string): Plugin {
  return {
    name: 'forge-remote-manifest',
    apply: 'build',
    generateBundle(_outputOpts, bundle) {
      const chunk = bundle[fileName];
      const code = chunk && 'code' in chunk ? chunk.code : '';
      const hash = createHash('sha256').update(code).digest('hex').slice(0, 16);
      const manifest: RemoteManifest = {
        app: opts.app,
        components: opts.components.map((c) => ({
          ...c,
          file: fileName,
          hash,
          version: c.version ?? opts.version ?? '0.0.0',
        })),
      };
      this.emitFile({
        type: 'asset',
        fileName: 'manifest.json',
        source: JSON.stringify(manifest, null, 2),
      });
    },
  };
}
