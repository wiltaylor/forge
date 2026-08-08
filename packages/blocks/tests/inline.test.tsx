/* The safety invariant of InlineMd: raw HTML in the input stays literal text
   in the output. The parser promises it, but the promise holds only while the
   renderer never assigns HTML directly — so these tests mount the component
   and assert on the DOM that comes out. */
import { describe, expect, it } from 'vitest';
import { render } from '@solidjs/testing-library';
import { InlineMd } from '../src/inline';

describe('InlineMd raw-HTML safety', () => {
  it('renders a script tag as literal text', () => {
    const { container } = render(() => <InlineMd md={'<script>alert(1)</script>'} />);
    expect(container.querySelector('script')).toBeNull();
    expect(container.textContent).toContain('<script>alert(1)</script>');
  });

  it('renders an img tag with an event handler as literal text', () => {
    const { container } = render(() => <InlineMd md={'<img src=x onerror=alert(1)>'} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<img src=x onerror=alert(1)>');
  });

  it('keeps HTML literal while markdown around it still renders', () => {
    const { container } = render(() => <InlineMd md={'**bold** and <b>not bold</b>'} />);
    expect(container.querySelector('strong')?.textContent).toBe('bold');
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toContain('<b>not bold</b>');
  });

  it('keeps HTML literal inside emphasis children', () => {
    const { container } = render(() => <InlineMd md={'**<i>x</i>**'} />);
    expect(container.querySelector('i')).toBeNull();
    expect(container.querySelector('strong')?.textContent).toBe('<i>x</i>');
  });

  it('keeps HTML literal in inline code', () => {
    const { container } = render(() => <InlineMd md={'`<script>b</script>`'} />);
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('code')?.textContent).toBe('<script>b</script>');
  });

  it('keeps HTML literal through emoji resolution on text nodes', () => {
    const { container } = render(() => <InlineMd md={'<u>x</u> :smile:'} />);
    expect(container.querySelector('u')).toBeNull();
    expect(container.textContent).toContain('<u>x</u>');
  });
});
