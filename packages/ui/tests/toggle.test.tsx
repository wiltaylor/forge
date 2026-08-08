/* The smoke test for the DOM environment: it mounts a real component, clicks
   it the way a user would and asserts on the document that came out. If this
   fails, suspect the environment (see docs/web-testing.md) before the Toggle. */
import { describe, expect, it, vi } from 'vitest';
import { createSignal } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Toggle } from '../src/forms';

/* Found the way a user or a screen reader finds it, not by CSS class. */
const toggle = () => screen.getByRole<HTMLInputElement>('switch', { name: 'Dark mode' });

describe('Toggle', () => {
  it('shows the state it is given', () => {
    render(() => <Toggle checked>Dark mode</Toggle>);

    expect(toggle().checked).toBe(true);
  });

  it('reports the new state when the user clicks it', () => {
    const onChange = vi.fn();
    render(() => (
      <Toggle checked={false} onChange={onChange}>
        Dark mode
      </Toggle>
    ));

    fireEvent.click(toggle());

    expect(onChange).toHaveBeenCalledWith(true);
  });

  it('is controlled — the owner of the state decides what is shown', () => {
    const [on, setOn] = createSignal(false);
    render(() => (
      <Toggle checked={on()} onChange={setOn}>
        Dark mode
      </Toggle>
    ));

    expect(toggle().checked).toBe(false);

    fireEvent.click(toggle());
    expect(toggle().checked).toBe(true);

    setOn(false);
    expect(toggle().checked).toBe(false);
  });
});
