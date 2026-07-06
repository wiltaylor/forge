/* Playpen starter app — a tiny working design-playground demo.
   Replace this with the real playground; keep the shape:
   controls (left) → live preview (right) → prompt output (bottom),
   one createStore state, presets, debounced persistence to /api/data/state. */

import { createMemo, createSignal, onMount, For } from 'solid-js';
import { createStore, reconcile, unwrap } from 'solid-js/store';
import { Copy, Check, RotateCcw } from 'lucide-solid';
import { Button, Card, Badge } from './forge/ui';
import { loadDoc, saveDocDebounced } from './api';

const DEFAULTS = { radius: 4, paddingX: 12, label: 'Deploy', variant: 'primary' };

const PRESETS = {
  default: { ...DEFAULTS },
  compact: { ...DEFAULTS, radius: 2, paddingX: 8 },
  airy: { ...DEFAULTS, radius: 8, paddingX: 20 },
};

export default function App() {
  const [state, setState] = createStore({ ...DEFAULTS });
  const [copied, setCopied] = createSignal(false);

  onMount(async () => {
    const saved = await loadDoc('state');
    if (saved) setState(reconcile({ ...DEFAULTS, ...saved }));
  });

  const set = (key, value) => {
    setState(key, value);
    saveDocDebounced('state', unwrap(state));
  };

  const applyPreset = (values) => {
    setState(reconcile({ ...values }));
    saveDocDebounced('state', unwrap(state));
  };

  const prompt = createMemo(() => {
    const parts = [];
    if (state.radius !== DEFAULTS.radius) parts.push(`a ${state.radius}px corner radius`);
    if (state.paddingX !== DEFAULTS.paddingX) parts.push(`${state.paddingX}px horizontal padding`);
    if (state.variant !== DEFAULTS.variant) parts.push(`the ${state.variant} variant`);
    if (state.label !== DEFAULTS.label) parts.push(`the label "${state.label}"`);
    return parts.length
      ? `Update the button to use ${parts.join(', ')}.`
      : 'The current defaults look right — no changes needed.';
  });

  const copy = async () => {
    await navigator.clipboard.writeText(prompt());
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  };

  return (
    <div class="pp-shell">
      <aside class="pp-controls">
        <Card title="Controls">
          <div class="pp-field-stack">
            <label class="ffield">
              <span class="ffield-label">Corner radius — {state.radius} px</span>
              <input
                type="range"
                min="0"
                max="24"
                value={state.radius}
                onInput={(e) => set('radius', +e.currentTarget.value)}
              />
            </label>
            <label class="ffield">
              <span class="ffield-label">Horizontal padding — {state.paddingX} px</span>
              <input
                type="range"
                min="4"
                max="32"
                value={state.paddingX}
                onInput={(e) => set('paddingX', +e.currentTarget.value)}
              />
            </label>
            <label class="ffield">
              <span class="ffield-label">Label</span>
              <span class="ffield-input">
                <input
                  value={state.label}
                  onInput={(e) => set('label', e.currentTarget.value)}
                />
              </span>
            </label>
            <label class="ffield">
              <span class="ffield-label">Variant</span>
              <span class="ffield-input">
                <select
                  value={state.variant}
                  onInput={(e) => set('variant', e.currentTarget.value)}
                  style={{ flex: 1, background: 'transparent', border: 0, color: 'var(--fg-0)', font: 'inherit' }}
                >
                  <For each={['primary', 'secondary', 'ghost', 'danger']}>
                    {(v) => <option value={v}>{v}</option>}
                  </For>
                </select>
              </span>
            </label>
          </div>
        </Card>
        <Card title="Presets">
          <div class="pp-preset-row">
            <For each={Object.entries(PRESETS)}>
              {([name, values]) => (
                <Button size="sm" onClick={() => applyPreset(values)}>
                  {name}
                </Button>
              )}
            </For>
            <Button size="sm" variant="ghost" icon={RotateCcw} onClick={() => applyPreset(DEFAULTS)}>
              Reset
            </Button>
          </div>
        </Card>
      </aside>

      <main class="pp-preview">
        <Card title="Preview" action={<Badge tone="accent" dot>live</Badge>}>
          <div class="pp-stage">
            <button
              class={`fbtn fbtn-${state.variant} fbtn-md`}
              style={{
                'border-radius': `${state.radius}px`,
                padding: `0 ${state.paddingX}px`,
              }}
            >
              {state.label}
            </button>
          </div>
        </Card>
      </main>

      <footer class="pp-prompt">
        <Card
          title="Prompt"
          action={
            <Button size="sm" icon={copied() ? Check : Copy} onClick={copy}>
              {copied() ? 'Copied' : 'Copy prompt'}
            </Button>
          }
        >
          <pre class="pp-prompt-text">{prompt()}</pre>
        </Card>
      </footer>
    </div>
  );
}
