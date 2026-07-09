import { defineConfig } from 'tsup';
import * as preset from 'tsup-preset-solid';

export default defineConfig((config) => {
  const parsed = preset.parsePresetOptions(
    {
      entries: [
        { entry: 'src/index.tsx' },
        { entry: 'src/vite.ts', name: 'vite' },
      ],
    },
    !!config.watch,
  );
  return preset.generateTsupOptions(parsed);
});
