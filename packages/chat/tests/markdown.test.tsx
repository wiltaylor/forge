/* The safety invariant of the Markdown control: raw HTML in the input stays
   literal text in the output. The parser promises it, but the promise holds
   only while the renderer never assigns HTML directly — so these tests mount
   the component and assert on the DOM that comes out. */
import { describe, expect, it } from 'vitest';
import { render } from '@solidjs/testing-library';
import { Markdown } from '../src/markdown';

describe('Markdown raw-HTML safety', () => {
  it('renders a script tag as literal text', () => {
    const { container } = render(() => <Markdown text={'<script>alert(1)</script>'} />);
    expect(container.querySelector('script')).toBeNull();
    expect(container.textContent).toContain('<script>alert(1)</script>');
  });

  it('renders an img tag with an event handler as literal text', () => {
    const { container } = render(() => <Markdown text={'<img src=x onerror=alert(1)>'} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.textContent).toContain('<img src=x onerror=alert(1)>');
  });

  it('keeps HTML literal while markdown around it still renders', () => {
    const { container } = render(() => <Markdown text={'**bold** and <b>not bold</b>'} />);
    expect(container.querySelector('strong')?.textContent).toBe('bold');
    expect(container.querySelector('b')).toBeNull();
    expect(container.textContent).toContain('<b>not bold</b>');
  });

  it('keeps HTML literal inside emphasis children', () => {
    const { container } = render(() => <Markdown text={'**<i>x</i>**'} />);
    expect(container.querySelector('i')).toBeNull();
    expect(container.querySelector('strong')?.textContent).toBe('<i>x</i>');
  });

  it('keeps HTML literal in headings, list items and table cells', () => {
    const text = [
      '# Head <u>a</u>',
      '',
      '- item <u>b</u>',
      '',
      '| col <u>c</u> | x |',
      '| --- | --- |',
      '| cell <u>d</u> | y |',
    ].join('\n');
    const { container } = render(() => <Markdown text={text} />);
    expect(container.querySelector('u')).toBeNull();
    expect(container.querySelector('h1')?.textContent).toBe('Head <u>a</u>');
    expect(container.querySelector('li')?.textContent).toBe('item <u>b</u>');
    expect(container.querySelector('th')?.textContent).toBe('col <u>c</u>');
    expect(container.querySelector('td')?.textContent).toBe('cell <u>d</u>');
  });

  it('keeps HTML literal in fenced and inline code', () => {
    const { container } = render(() => (
      <Markdown text={'```\n<script>a</script>\n```\n\n`<script>b</script>`'} />
    ));
    expect(container.querySelector('script')).toBeNull();
    expect(container.querySelector('pre code')?.textContent).toBe('<script>a</script>');
    expect(container.querySelector('p code')?.textContent).toBe('<script>b</script>');
  });
});
