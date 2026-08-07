import { createSignal, type JSX } from 'solid-js'
import './forge/tokens.css'
import './forge/shell.css'
import { Combobox } from './forge/Combobox'
import { REGIONS } from './regions'

export function App(): JSX.Element {
  // Nothing is selected when the screen opens.
  const [region, setRegion] = createSignal<string | null>(null)

  return (
    <div class="app-shell">
      <main class="app-main">
        <header class="page-head">
          <div>
            <h1>Settings</h1>
            <p>Deployment defaults for this account.</p>
          </div>
        </header>

        <div class="settings-layout">
          <section class="settings-section">
            <h2>Deployment</h2>
            <div class="settings-row">
              <Combobox
                label="Region"
                help="Where new workloads run. Type to narrow the list. Two regions are unavailable on this account."
                placeholder="Search regions"
                emptyText="No region matches that search."
                options={REGIONS}
                value={region()}
                onChange={setRegion}
              />
            </div>
          </section>
        </div>
      </main>
    </div>
  )
}
