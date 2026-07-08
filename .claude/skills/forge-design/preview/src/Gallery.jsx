import { createSignal, onCleanup } from 'solid-js';
import { Hexagon, Moon } from 'lucide-solid';
import {
  AppShell, NavSection, NavLink, Crumbs, IconButton, Badge,
} from '@forge/ui.jsx';
import Primitives from './sections/Primitives';
import Layout from './sections/Layout';
import TableDemo from './sections/TableDemo';
import LogsDemo from './sections/LogsDemo';
import SettingsDemo from './sections/SettingsDemo';
import ModalDemo from './sections/ModalDemo';

const SECTIONS = [
  ['primitives', 'Primitives', Primitives],
  ['layout', 'Page & layout', Layout],
  ['tables', 'Tables', TableDemo],
  ['logs', 'Logs', LogsDemo],
  ['settings', 'Settings', SettingsDemo],
  ['modal', 'Modal', ModalDemo],
];

function useViewport() {
  const [w, setW] = createSignal(window.innerWidth);
  const onResize = () => setW(window.innerWidth);
  window.addEventListener('resize', onResize);
  onCleanup(() => window.removeEventListener('resize', onResize));
  return () => `${w()} px · ${w() > 1024 ? 'desktop' : w() > 768 ? 'compact' : 'mobile'}`;
}

export default function Gallery() {
  const [active, setActive] = createSignal('primitives');
  const viewport = useViewport();

  const toggleTheme = () => {
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const current = document.documentElement.dataset.theme ?? (dark ? 'dark' : 'light');
    document.documentElement.dataset.theme = current === 'dark' ? 'light' : 'dark';
  };

  const jump = (id) => {
    setActive(id);
    document.getElementById(id)?.scrollIntoView({ block: 'start' });
  };

  return (
    <AppShell
      topbar={
        <>
          <div class="ftopbar-brand" style={{ 'font-weight': '600' }}>
            <Hexagon size={18} strokeWidth={1.5} /> Forge preview
          </div>
          <Crumbs items={['design system', 'gallery']} />
          <div style={{ flex: 1 }} />
          <Badge tone="accent">{viewport()}</Badge>
          <IconButton icon={Moon} label="Toggle theme" onClick={toggleTheme} />
        </>
      }
      sidebar={
        <>
          <NavSection>Components</NavSection>
          {SECTIONS.map(([id, label]) => (
            <NavLink href={`#${id}`} active={active() === id}
                     onClick={(e) => { e.preventDefault(); jump(id); }}>
              {label}
            </NavLink>
          ))}
          <NavSection>Shell</NavSection>
          <div style={{ padding: '6px 10px', 'font-size': '12px', color: 'var(--fg-2)' }}>
            This gallery runs inside <code>AppShell</code> — resize below 1024px
            for the drawer, below 768px for mobile stacking.
          </div>
        </>
      }
    >
      {SECTIONS.map(([id, label, Section]) => (
        <section id={id} style={{ 'margin-bottom': '40px' }}>
          <Section />
        </section>
      ))}
    </AppShell>
  );
}
