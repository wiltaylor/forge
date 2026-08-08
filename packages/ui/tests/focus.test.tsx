/* Focus behaviour of the overlay module: Tab stays inside a modal surface, and
   closing an overlay returns focus to whatever opened it. Asserted the way a
   keyboard user perceives it — where focus is after a real key event. */
import { describe, expect, it } from 'vitest';
import { createSignal } from 'solid-js';
import { fireEvent, render, screen } from '@solidjs/testing-library';
import { Command, Modal, Popover, Sheet } from '../src/overlays';

describe('modal dialog semantics', () => {
  it('marks the Sheet as a modal dialog like the Modal', () => {
    render(() => (
      <Sheet open title="Details">
        <p>Sheet body</p>
      </Sheet>
    ));

    expect(screen.getByRole('dialog').getAttribute('aria-modal')).toBe('true');
  });

  /* The Command traps like the Modal does, so it carries the same marking. */
  it('marks the Command palette as a modal dialog like the Modal', () => {
    render(() => <Command open items={[]} />);

    expect(screen.getByRole('dialog').getAttribute('aria-modal')).toBe('true');
  });
});

describe('focus restore', () => {
  it('returns focus to the trigger when a Modal closes on Escape', () => {
    const [open, setOpen] = createSignal(false);
    render(() => (
      <>
        <button type="button" onClick={() => setOpen(true)}>Open settings</button>
        <Modal open={open()} onClose={() => setOpen(false)} title="Settings">
          <input aria-label="Name" />
        </Modal>
      </>
    ));
    const trigger = screen.getByRole('button', { name: 'Open settings' });
    trigger.focus();
    fireEvent.click(trigger);
    screen.getByRole('textbox', { name: 'Name' }).focus();

    fireEvent.keyDown(document.body, { key: 'Escape' });

    expect(document.activeElement).toBe(trigger);
  });

  /* Not only modal surfaces: an anchored overlay restores the same way. */
  it('returns focus to the trigger when a Popover closes on Escape', () => {
    render(() => (
      <Popover label="Filters">
        <button type="button">Apply</button>
      </Popover>
    ));
    const trigger = screen.getByRole('button', { name: 'Filters' });
    trigger.focus();
    fireEvent.click(trigger);
    const apply = screen.getByRole('button', { name: 'Apply' });
    apply.focus();

    fireEvent.keyDown(apply, { key: 'Escape' });

    expect(document.activeElement).toBe(trigger);
  });

  it('leaves focus where a dismissing outside pointer put it', () => {
    const [open, setOpen] = createSignal(true);
    render(() => (
      <>
        <input aria-label="Note" />
        <Modal open={open()} onClose={() => setOpen(false)} title="Settings">
          <input aria-label="Name" />
        </Modal>
      </>
    ));
    const note = screen.getByRole('textbox', { name: 'Note' });
    note.focus();

    fireEvent.pointerDown(screen.getByRole('dialog').parentElement!);

    expect(screen.queryByRole('dialog')).toBeNull();
    expect(document.activeElement).toBe(note);
  });
});

describe('focus trap', () => {
  /* The Modal's focusables in document order: the head Close button, the body
     input, the footer Save button. */
  const mountModal = () =>
    render(() => (
      <Modal open title="Settings" footer={<button type="button">Save</button>}>
        <input aria-label="Name" />
      </Modal>
    ));

  it('wraps Tab from the last focusable to the first', () => {
    mountModal();
    const save = screen.getByRole('button', { name: 'Save' });
    save.focus();

    fireEvent.keyDown(save, { key: 'Tab' });

    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Close' }));
  });

  it('wraps Shift-Tab from the first focusable to the last', () => {
    mountModal();
    const close = screen.getByRole('button', { name: 'Close' });
    close.focus();

    fireEvent.keyDown(close, { key: 'Tab', shiftKey: true });

    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Save' }));
  });

  /* A hidden element after the last visible focusable must not become the edge
     of the cycle, or the wrap never fires from the element the user actually
     reaches last. */
  it('skips hidden elements when it finds the edges', () => {
    render(() => (
      <Modal
        open
        title="Settings"
        footer={
          <>
            <button type="button">Save</button>
            <div hidden>
              <button type="button">Buried</button>
            </div>
            <input type="hidden" name="token" />
          </>
        }
      >
        <input aria-label="Name" />
      </Modal>
    ));
    const save = screen.getByRole('button', { name: 'Save' });
    save.focus();

    fireEvent.keyDown(save, { key: 'Tab' });

    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Close' }));
  });

  it('brings Tab back in when focus is outside the modal', () => {
    render(() => (
      <>
        <button type="button">Elsewhere</button>
        <Modal open title="Settings">
          <input aria-label="Name" />
        </Modal>
      </>
    ));
    const elsewhere = screen.getByRole('button', { name: 'Elsewhere' });
    elsewhere.focus();

    fireEvent.keyDown(elsewhere, { key: 'Tab' });

    expect(document.activeElement).toBe(screen.getByRole('button', { name: 'Close' }));
  });
});
