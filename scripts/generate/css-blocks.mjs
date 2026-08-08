/**
 * Read a token stylesheet back as declarations per block.
 *
 * The generator's tests assert on what a block declares rather than on the text
 * that declares it, so that formatting is free to change and values are not.
 * The reader handles only the shape the token stylesheet has: nested blocks,
 * comments, and one `name: value;` declaration per line.
 */

/**
 * @param {string} css
 * @returns {Record<string, Record<string, string>>} block path (selectors joined by ' > ')
 *   to its declarations.
 */
export function parseBlocks(css) {
  const blocks = {};
  const stack = [];
  let inComment = false;

  for (const raw of css.split('\n')) {
    let line = raw;
    if (inComment) {
      const end = line.indexOf('*/');
      if (end === -1) continue;
      line = line.slice(end + 2);
      inComment = false;
    }
    for (;;) {
      const start = line.indexOf('/*');
      if (start === -1) break;
      const end = line.indexOf('*/', start + 2);
      if (end === -1) {
        line = line.slice(0, start);
        inComment = true;
        break;
      }
      // A space, so `1px/* note */solid` cannot become one token.
      line = `${line.slice(0, start)} ${line.slice(end + 2)}`;
    }

    const text = line.trim();
    if (!text) continue;
    if (text === '}') {
      if (!stack.length) throw new Error(`stray closing brace: ${raw}`);
      stack.pop();
      continue;
    }
    if (text.endsWith('{')) {
      stack.push(text.slice(0, -1).trim());
      blocks[stack.join(' > ')] ??= {};
      continue;
    }
    const decl = /^([\w-]+)\s*:\s*([^;]+);$/.exec(text);
    if (!decl) throw new Error(`unparsed line: ${raw}`);
    if (!stack.length) throw new Error(`declaration outside a block: ${raw}`);
    blocks[stack.join(' > ')][decl[1]] = decl[2].trim();
  }

  if (stack.length) throw new Error(`unclosed block: ${stack.join(' > ')}`);
  return blocks;
}
