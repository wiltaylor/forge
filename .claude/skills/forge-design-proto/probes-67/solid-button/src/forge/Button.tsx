import { Show, splitProps, type JSX } from 'solid-js'
import { Spinner } from './Spinner'

export type ButtonVariant = 'default' | 'primary' | 'ghost' | 'danger'
export type ButtonSize = 'sm' | 'md'

export interface ButtonProps
  extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  /** Fill and stroke only. Geometry never changes with the variant. */
  variant?: ButtonVariant
  /** `md` is the default height. There is no large. */
  size?: ButtonSize
  /** Swaps the leading icon for a spinner and disables the button. */
  loading?: boolean
  /** Icon before the label. */
  iconBefore?: JSX.Element
  /** Icon after the label. */
  iconAfter?: JSX.Element
}

/** One action, labelled. Use `nav-link` when it navigates rather than acts. */
export function Button(props: ButtonProps): JSX.Element {
  const [own, rest] = splitProps(props, [
    'variant',
    'size',
    'loading',
    'iconBefore',
    'iconAfter',
    'class',
    'children',
    'type',
    'disabled',
  ])

  const classes = (): string => {
    const parts = ['fbtn']
    if (own.variant && own.variant !== 'default') parts.push(`fbtn-${own.variant}`)
    if (own.size === 'sm') parts.push('fbtn-sm')
    if (own.class) parts.push(own.class)
    return parts.join(' ')
  }

  // `loading` takes the disabled path, so activation is a no-op either way.
  const isDisabled = (): boolean => own.disabled === true || own.loading === true

  return (
    <button
      {...rest}
      type={own.type ?? 'button'}
      class={classes()}
      disabled={isDisabled()}
      aria-busy={own.loading ? 'true' : undefined}
    >
      <Show when={own.loading} fallback={own.iconBefore}>
        <Spinner />
      </Show>
      {/* The label stays mounted while loading, so the width does not change. */}
      {own.children}
      {own.iconAfter}
    </button>
  )
}
