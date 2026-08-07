import { createSignal, type JSX } from 'solid-js'
import { Button } from './forge/Button'

export function App(): JSX.Element {
  const [inFlight, setInFlight] = createSignal(false)

  return (
    <div class="app-shell">
      <main class="app-main">
        <div class="page-head">
          <h1>Deploy</h1>
        </div>

        <p class="deploy-status">
          <span>Status</span>
          <span
            class={
              inFlight()
                ? 'deploy-status-value is-active'
                : 'deploy-status-value'
            }
          >
            {inFlight() ? 'Deploy in flight' : 'Idle'}
          </span>
        </p>

        <div class="deploy-actions">
          <Button
            variant="primary"
            loading={inFlight()}
            onClick={() => setInFlight(true)}
          >
            Deploy
          </Button>
          <Button disabled={inFlight()}>Dry run</Button>
          <Button variant="danger" onClick={() => setInFlight(false)}>
            Cancel deployment
          </Button>
        </div>

        <div class="deploy-toggle">
          <p class="deploy-toggle-hint">
            Switch the in-flight state to see the loading and disabled buttons.
          </p>
          <Button size="sm" onClick={() => setInFlight(!inFlight())}>
            {inFlight() ? 'Stop the deploy' : 'Start a deploy'}
          </Button>
        </div>
      </main>
    </div>
  )
}
