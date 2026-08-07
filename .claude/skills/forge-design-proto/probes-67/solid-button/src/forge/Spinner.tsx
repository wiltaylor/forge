import type { JSX } from 'solid-js'

export interface SpinnerProps {
  /** Extra classes, appended after `.fspinner`. */
  class?: string
  /** Accessible name. Omit it inside a control that is already labelled. */
  label?: string
}

/**
 * An indeterminate progress mark at 1.5px stroke in `currentColor`.
 * `button` puts it in the leading icon slot while `loading` is set.
 */
export function Spinner(props: SpinnerProps): JSX.Element {
  return (
    <svg
      class={props.class ? `fspinner ${props.class}` : 'fspinner'}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linecap="round"
      role={props.label ? 'img' : undefined}
      aria-label={props.label}
      aria-hidden={props.label ? undefined : 'true'}
    >
      <path d="M12 3a9 9 0 1 0 9 9" />
    </svg>
  )
}
