import { createSignal, Show } from 'solid-js';
import { Tabs } from '@forge/ui';
import Overview from './sections/Overview';
import TermTab from './sections/TermTab';
import DesktopTab from './sections/DesktopTab';

/* Tab panels mount on first activation (xterm.js must not initialize inside
   a display:none container — zero dimensions break its renderer), then stay
   mounted with display toggling so live widget sessions survive switching. */
export default function App() {
  const [tab, setTab] = createSignal('overview');
  const [visited, setVisited] = createSignal({ overview: true });
  const activate = (id) => {
    setVisited((v) => ({ ...v, [id]: true }));
    setTab(id);
  };
  const panel = (id) => ({ display: tab() === id ? 'block' : 'none' });

  return (
    <div style={{ padding: 'var(--sp-6)', display: 'grid', gap: 'var(--sp-5)', 'align-content': 'start', 'min-height': '100vh' }}>
      <Tabs
        tabs={[
          { id: 'overview', label: 'Overview' },
          { id: 'terminal', label: 'Terminal' },
          { id: 'desktop', label: 'Desktop' },
        ]}
        active={tab()}
        onChange={activate}
      />
      <div style={panel('overview')}><Overview /></div>
      <Show when={visited().terminal}>
        <div style={panel('terminal')}><TermTab /></div>
      </Show>
      <Show when={visited().desktop}>
        <div style={panel('desktop')}><DesktopTab /></div>
      </Show>
    </div>
  );
}
