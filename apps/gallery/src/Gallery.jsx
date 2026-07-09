import { Show, createSignal, onCleanup } from 'solid-js';
import { Hexagon, LogOut, Moon, Palette } from 'lucide-solid';
import {
  AppShell, NavSection, NavLink, Crumbs, IconButton, Badge, Toaster,
} from '@forge/ui';
import { applyTheme, darkTheme, defineTheme } from '@forge/tokens';
import Primitives from './sections/Primitives';
import FormsDemo from './sections/FormsDemo';
import Forms2Demo from './sections/Forms2Demo';
import Layout from './sections/Layout';
import StructureDemo from './sections/StructureDemo';
import OverlaysDemo from './sections/OverlaysDemo';
import TableDemo from './sections/TableDemo';
import LogsDemo from './sections/LogsDemo';
import SettingsDemo from './sections/SettingsDemo';
import ModalDemo from './sections/ModalDemo';
import GraphDemo from './sections/GraphDemo';
import CodeDemo from './sections/CodeDemo';
import ChartsDemo from './sections/ChartsDemo';
import LiveDemo from './sections/LiveDemo';
import DataDemo from './sections/DataDemo';
import RemoteDemo from './sections/RemoteDemo';

const SECTIONS = [
  ['primitives', 'Primitives', Primitives],
  ['forms', 'Forms', FormsDemo],
  ['forms2', 'Forms 2', Forms2Demo],
  ['layout', 'Page & layout', Layout],
  ['structure', 'Navigation & structure', StructureDemo],
  ['tables', 'Tables', TableDemo],
  ['logs', 'Logs', LogsDemo],
  ['settings', 'Settings', SettingsDemo],
  ['modal', 'Modal', ModalDemo],
  ['overlays', 'Overlays & menus', OverlaysDemo],
  ['graph', 'Node graph', GraphDemo],
  ['code', 'Code', CodeDemo],
  ['charts', 'Charts', ChartsDemo],
];

const BACKEND_SECTIONS = [
  ['live', 'Live events', LiveDemo],
  ['data', 'Document store', DataDemo],
  ['remote', 'Remote components', RemoteDemo],
];

/* A custom brand theme — proves applyTheme() recolors everything, including
   shadow-DOM remotes, without touching any component. */
const emberTheme = defineTheme(darkTheme, {
  name: 'ember',
  accent: {
    base: 'oklch(0.65 0.17 45)',
    hover: 'oklch(0.70 0.18 45)',
    press: 'oklch(0.58 0.17 45)',
    bg: 'oklch(0.65 0.17 45 / 0.14)',
    fg: 'oklch(0.83 0.13 50)',
    contrast: '#FFFFFF',
  },
  bg: ['#0F0B09', '#171110', '#1F1715', '#281D1A', '#322420'],
  border: { subtle: '#241A17', default: '#33251F', strong: '#4A362D' },
});

function useViewport() {
  const [w, setW] = createSignal(window.innerWidth);
  const onResize = () => setW(window.innerWidth);
  window.addEventListener('resize', onResize);
  onCleanup(() => window.removeEventListener('resize', onResize));
  return () => `${w()} px · ${w() > 1024 ? 'desktop' : w() > 768 ? 'compact' : 'mobile'}`;
}

export default function Gallery(props) {
  const [active, setActive] = createSignal('primitives');
  const [ember, setEmber] = createSignal(false);
  const viewport = useViewport();

  const toggleTheme = () => {
    setEmber(false);
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const current = document.documentElement.dataset.theme ?? (dark ? 'dark' : 'light');
    applyTheme(current === 'dark' ? 'light' : 'dark');
  };
  const toggleEmber = () => {
    const on = !ember();
    setEmber(on);
    applyTheme(on ? emberTheme : 'dark');
  };

  const jump = (id) => {
    setActive(id);
    document.getElementById(id)?.scrollIntoView({ block: 'start' });
  };

  const navLinks = (sections) => sections.map(([id, label]) => (
    <NavLink href={`#${id}`} active={active() === id}
             onClick={(e) => { e.preventDefault(); jump(id); }}>
      {label}
    </NavLink>
  ));

  return (
    <AppShell
      topbar={
        <>
          <div class="ftopbar-brand" style={{ 'font-weight': '600' }}>
            <Hexagon size={18} strokeWidth={1.5} /> Forge gallery
          </div>
          <Crumbs items={['design system', 'gallery']} />
          <div style={{ flex: 1 }} />
          <Badge tone="accent">{viewport()}</Badge>
          <Show when={props.backend}>
            <Badge tone={props.backend.auth_enabled ? 'success' : 'neutral'}>
              {props.backend.app ?? 'backend'} · {props.backend.auth_enabled ? 'auth' : 'open'}
            </Badge>
          </Show>
          <IconButton icon={Palette} label="Toggle custom theme" onClick={toggleEmber} />
          <IconButton icon={Moon} label="Toggle dark/light" onClick={toggleTheme} />
          <Show when={props.backend?.auth_enabled}>
            <IconButton icon={LogOut} label="Sign out" onClick={() => props.onLogout?.()} />
          </Show>
        </>
      }
      sidebar={
        <>
          <NavSection>Components</NavSection>
          {navLinks(SECTIONS)}
          <NavSection>Backend</NavSection>
          {navLinks(BACKEND_SECTIONS)}
          <NavSection>Shell</NavSection>
          <div style={{ padding: '6px 10px', 'font-size': '12px', color: 'var(--fg-2)' }}>
            This gallery runs inside <code>AppShell</code> — resize below 1024px
            for the drawer, below 768px for mobile stacking.
          </div>
        </>
      }
    >
      <Toaster />
      {[...SECTIONS, ...BACKEND_SECTIONS].map(([id, label, Section]) => (
        <section id={id} style={{ 'margin-bottom': '40px' }}>
          <Section />
        </section>
      ))}
    </AppShell>
  );
}
