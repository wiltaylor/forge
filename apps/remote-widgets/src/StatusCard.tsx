import { Card, Badge, Button, StatusDot } from '@forge/ui';
import type { StatusTone } from '@forge/ui';
import type { RemoteComponentProps } from '@forge/remote';

/* A remote widget: shows a service status, lets the host request a refresh
   via the `refresh` CustomEvent. All colours are var(--token) — the HOST's
   theme decides how this looks. */
export function StatusCard(props: RemoteComponentProps) {
  const status = () => (props.status as StatusTone | undefined) ?? 'neutral';
  const toneOf = (): 'success' | 'warning' | 'danger' | 'info' | 'neutral' => status();
  return (
    <Card
      title={(props.title as string) ?? 'Service'}
      action={
        <Button size="sm" onClick={() => props.emit('refresh', { at: Date.now() })}>
          Refresh
        </Button>
      }
    >
      <div style={{ display: 'flex', 'align-items': 'center', gap: 'var(--sp-2)' }}>
        <StatusDot tone={status()} />
        <Badge tone={toneOf()}>{status()}</Badge>
        <span style={{ color: 'var(--fg-1)', 'font-size': 'var(--fs-sm)' }}>
          {(props.message as string) ?? 'No message'}
        </span>
      </div>
    </Card>
  );
}
