import {
  createEffect,
  createMemo,
  createSignal,
  createUniqueId,
  For,
  Show,
  type JSX,
} from 'solid-js'
import { Check, ChevronDown, Search } from 'lucide-solid'
import { createDismiss } from './dismiss'
import './combobox.css'

/** One row of the popup. `label` is what the filter reads and what the field shows. */
export type ComboboxOption = {
  readonly value: string
  readonly label: string
  /** A disabled option shows and can be moved to, but it cannot be committed. */
  readonly disabled?: boolean
}

export type ComboboxProps = {
  /** Every option, in the order they are shown. */
  readonly options: readonly ComboboxOption[]
  /** The committed selection, owned by the caller. `null` is nothing selected. */
  readonly value: string | null
  /** Fired when the user commits an option. The caller applies it to `value`. */
  readonly onChange: (value: string) => void
  /** The line shown in place of the list when the filter matches nothing. */
  readonly emptyText: string
  /** The field label. Omit it when something outside labels the field. */
  readonly label?: string
  /** The help line under the field. */
  readonly help?: string
  /** The placeholder, shown while nothing is selected and nothing is typed. */
  readonly placeholder?: string
  /** Marks the frame and the help line as errored. */
  readonly error?: boolean
  /** Disables the whole field. */
  readonly disabled?: boolean
  /**
   * Replaces the default filter, which is a case-insensitive substring match on
   * the option's label.
   */
  readonly filter?: (option: ComboboxOption, query: string) => boolean
}

const defaultFilter = (option: ComboboxOption, query: string): boolean =>
  option.label.toLowerCase().includes(query.toLowerCase())

export function Combobox(props: ComboboxProps): JSX.Element {
  // Four pieces of state. `query` is null when unset — unset shows the selected
  // label in the field, the empty string shows an empty field and every option.
  const [open, setOpen] = createSignal(false)
  const [query, setQuery] = createSignal<string | null>(null)
  const [activeIdx, setActiveIdx] = createSignal(-1)

  const id = createUniqueId()
  const labelId = `${id}-label`
  const helpId = `${id}-help`
  const popId = `${id}-pop`
  const optId = (index: number) => `${id}-opt-${index}`

  let root: HTMLDivElement | undefined
  let field: HTMLInputElement | undefined
  let pop: HTMLDivElement | undefined

  const selected = createMemo(
    () => props.options.find((option) => option.value === props.value) ?? null,
  )

  const filtered = createMemo(() => {
    const typed = query()
    if (typed === null || typed === '') return props.options
    const match = props.filter ?? defaultFilter
    return props.options.filter((option) => match(option, typed))
  })

  const text = () => query() ?? selected()?.label ?? ''

  // Closing always clears the query. Miss it and the field keeps a stale search
  // string in place of the selected label.
  const close = () => {
    setOpen(false)
    setQuery(null)
    setActiveIdx(-1)
  }

  const commit = (index: number) => {
    const option = filtered()[index]
    // Committing a disabled option is a no-op, and the popup stays open.
    if (!option || option.disabled) return
    props.onChange(option.value)
    close()
  }

  createDismiss(open, close, () => root)

  // The active option is kept in view. `nearest` matters — the default scrolls the
  // row to the top of the popup on every arrow press.
  createEffect(() => {
    if (!open()) return
    const index = activeIdx()
    if (index < 0) return
    const row = pop?.children[index]
    if (row instanceof HTMLElement) row.scrollIntoView({ block: 'nearest' })
  })

  const onInput: JSX.EventHandler<HTMLInputElement, InputEvent> = (event) => {
    setQuery(event.currentTarget.value)
    setOpen(true)
    setActiveIdx(0)
  }

  // Focusing opens the popup and selects the text, so the first keystroke replaces
  // the shown label.
  const enter = () => {
    if (props.disabled || open()) return
    setOpen(true)
    field?.select()
  }

  // A press on a field that already holds focus fires no focus event, so a click
  // after Enter would otherwise never reopen the popup.
  const onFieldClick = () => enter()

  const onKeyDown: JSX.EventHandler<HTMLInputElement, KeyboardEvent> = (event) => {
    // Every key handled here calls preventDefault — arrows otherwise move the
    // caret, and Enter otherwise submits the surrounding form.
    switch (event.key) {
      case 'ArrowDown': {
        event.preventDefault()
        setOpen(true)
        setActiveIdx((index) => Math.min(index + 1, filtered().length - 1))
        break
      }
      case 'ArrowUp': {
        event.preventDefault()
        if (!open()) return
        setActiveIdx((index) => (index <= 0 ? 0 : index - 1))
        break
      }
      case 'Enter': {
        if (!open()) return
        event.preventDefault()
        commit(activeIdx())
        break
      }
      case 'Escape': {
        // One layer per press. A closed field lets Escape through to whatever is
        // behind it.
        if (!open()) return
        event.preventDefault()
        event.stopPropagation()
        close()
        break
      }
    }
  }

  return (
    <div class="ffield">
      <Show when={props.label}>
        <span class="ffield-label" id={labelId}>
          {props.label}
        </span>
      </Show>

      <div class="fcombo" ref={root}>
        <span
          class="ffield-input"
          classList={{ 'is-error': props.error, 'is-disabled': props.disabled }}
          onPointerDown={(event) => {
            // A press on the frame, the glyph or the chevron lands in the input.
            if (event.target !== field) {
              event.preventDefault()
              field?.focus()
            }
          }}
          onClick={onFieldClick}
        >
          <Search stroke-width={1.5} aria-hidden="true" />
          <input
            ref={field}
            type="text"
            role="combobox"
            id={id}
            value={text()}
            placeholder={props.placeholder}
            disabled={props.disabled}
            autocomplete="off"
            aria-autocomplete="list"
            aria-expanded={open()}
            aria-controls={popId}
            aria-labelledby={props.label ? labelId : undefined}
            aria-describedby={props.help ? helpId : undefined}
            aria-activedescendant={
              open() && activeIdx() >= 0 ? optId(activeIdx()) : undefined
            }
            onInput={onInput}
            onFocus={enter}
            onKeyDown={onKeyDown}
          />
          <ChevronDown stroke-width={1.5} aria-hidden="true" />
        </span>

        <Show when={open()}>
          <div class="fselect-pop" id={popId} role="listbox" ref={pop}>
            <Show
              when={filtered().length > 0}
              fallback={<div class="fcmd-empty">{props.emptyText}</div>}
            >
              <For each={filtered()}>
                {(option, index) => (
                  <div
                    class="fselect-opt"
                    id={optId(index())}
                    role="option"
                    classList={{
                      'is-active': index() === activeIdx(),
                      'is-selected': option.value === props.value,
                      'is-disabled': !!option.disabled,
                    }}
                    aria-selected={option.value === props.value}
                    aria-disabled={option.disabled ? true : undefined}
                    title={option.label}
                    // Without this the input blurs before the click lands and the
                    // popup closes underneath the pointer.
                    onPointerDown={(event) => event.preventDefault()}
                    onPointerMove={() => setActiveIdx(index())}
                    onClick={() => commit(index())}
                  >
                    <span>{option.label}</span>
                    <Show when={option.value === props.value}>
                      <span class="fselect-check">
                        <Check stroke-width={1.5} aria-hidden="true" />
                      </span>
                    </Show>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </Show>
      </div>

      <Show when={props.help}>
        <span class="ffield-help" classList={{ 'is-error': props.error }} id={helpId}>
          {props.help}
        </span>
      </Show>
    </div>
  )
}
