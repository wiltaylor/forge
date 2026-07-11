import { createSignal } from 'solid-js';
import { Bomb, PartyPopper, RefreshCw, Sparkles } from 'lucide-solid';
import {
  PageHead, Card, Button, Badge, Stat, ToggleGroup, toast, fx, detectFxTier,
} from '@forge/ui';

/* Particle FX — fx.explode / fx.recreate / fx.materialize / fx.burst.
   The target element is never removed from the DOM; effects hide it with
   inline visibility and restore it. The remote-image card exercises the
   colored-particle fallback (SVG snapshots can't load cross-origin images). */

export default function FxDemo() {
  const [mode, setMode] = createSignal('auto');
  const [tier, setTier] = createSignal(detectFxTier());
  let card1;
  let card2;

  const setFxMode = (m) => {
    setMode(m);
    fx.config({ mode: m });
    setTier(detectFxTier());
  };

  const run = (name, el) => {
    fx[name](el).then(() => toast(`fx.${name}() finished`, { tone: 'success' }));
  };

  return (
    <>
      <PageHead title="Particle FX" sub="fx.explode, fx.recreate, fx.materialize, fx.burst — item create/destroy feedback"
                actions={<Badge tone="accent">tier: {tier()}</Badge>} />

      <Card title="Motion mode" action={<Badge tone="neutral">fx.config()</Badge>}>
        <div style={{ display: 'flex', gap: '12px', 'align-items': 'center', 'flex-wrap': 'wrap' }}>
          <ToggleGroup
            options={[
              { value: 'auto', label: 'Auto' },
              { value: 'full', label: 'Full' },
              { value: 'reduced', label: 'Reduced' },
              { value: 'off', label: 'Off' },
            ]}
            value={mode()} onChange={setFxMode} />
          <span style={{ 'font-size': '12px', color: 'var(--fg-2)' }}>
            Auto detects prefers-reduced-motion, low-core/low-memory devices, and downgrades
            for the session if frames run slow. Off degrades every effect to a short fade.
          </span>
        </div>
      </Card>

      <div style={{ height: '16px' }} />

      <Card title="Effects" action={<Badge tone="neutral">element stays in the DOM</Badge>}>
        <div style={{ display: 'grid', gap: '16px', 'grid-template-columns': 'repeat(auto-fit, minmax(260px, 1fr))' }}>
          <div>
            <div ref={card1} style={{ 'margin-bottom': '12px' }}>
              <Card title="prod-worker-04">
                <Stat label="GPU util" value="87 %" delta="-3 %" tone="success" />
                <div style={{ display: 'flex', gap: '8px', 'margin-top': '10px' }}>
                  <Badge tone="success" dot>running</Badge>
                  <Badge tone="accent">a100 × 8</Badge>
                </div>
              </Card>
            </div>
            <div style={{ display: 'flex', gap: '8px', 'flex-wrap': 'wrap' }}>
              <Button size="sm" icon={Bomb} onClick={() => run('explode', card1)}>Explode</Button>
              <Button size="sm" variant="primary" icon={RefreshCw} onClick={() => run('recreate', card1)}>Recreate</Button>
              <Button size="sm" icon={Sparkles} onClick={() => run('materialize', card1)}>Materialize</Button>
              <Button size="sm" icon={PartyPopper} onClick={() => run('burst', card1)}>Burst</Button>
            </div>
          </div>

          <div>
            <div ref={card2} style={{ 'margin-bottom': '12px' }}>
              <Card title="Remote image (fallback path)">
                <img src="https://picsum.photos/seed/forge/240/100" alt="remote"
                     width="240" height="100" style={{ 'border-radius': 'var(--r-md)', display: 'block' }} />
                <div style={{ 'font-size': '12px', color: 'var(--fg-2)', 'margin-top': '8px' }}>
                  Cross-origin images snapshot blank, so this card bursts into
                  theme-tinted particles instead of its own pixels.
                </div>
              </Card>
            </div>
            <div style={{ display: 'flex', gap: '8px', 'flex-wrap': 'wrap' }}>
              <Button size="sm" variant="primary" icon={RefreshCw} onClick={() => run('recreate', card2)}>Recreate</Button>
              <Button size="sm" icon={PartyPopper} onClick={() => run('burst', card2)}>Burst</Button>
            </div>
          </div>
        </div>
      </Card>
    </>
  );
}
