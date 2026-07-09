import { Card, Stat } from '@forge/ui';
import { Sparkline } from '@forge/charts';
import type { RemoteComponentProps } from '@forge/remote';

/* A remote widget with a chart: proves zero-dep charts render inside a
   shadow root and follow the host theme. */
export function MetricsPanel(props: RemoteComponentProps) {
  const series = () => (props.series as number[] | undefined) ?? [];
  const latest = () => series().at(-1) ?? 0;
  const unit = () => (props.unit as string) ?? '';
  return (
    <Card title={(props.title as string) ?? 'Metrics'}>
      <div
        style={{ display: 'flex', 'align-items': 'center', gap: 'var(--sp-4)', cursor: 'pointer' }}
        onClick={() => props.emit('select', { latest: latest() })}
      >
        <Stat label="Latest" value={`${latest()}${unit()}`} />
        <Sparkline points={series()} width={160} height={40} />
      </div>
    </Card>
  );
}
