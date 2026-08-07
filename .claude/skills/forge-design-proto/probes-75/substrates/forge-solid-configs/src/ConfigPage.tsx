import { For } from 'solid-js';
import { versions } from './config/versions';

export function ConfigPage() {
  return (
    <section>
      <h1 class="page-title">Deployment config</h1>
      <ul class="stack">
        <For each={versions}>
          {(v) => (
            <li class="row">
              <span>{v.version}</span>
              <span class="muted">{v.savedAt}</span>
            </li>
          )}
        </For>
      </ul>
    </section>
  );
}
