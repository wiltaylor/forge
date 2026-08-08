/**
 * Bundle the package stylesheets into the copy-in CSS assets the design-system
 * skill ships.
 *
 * A skill asset must be one self-contained file, while the monorepo splits its
 * styles per package. Each bundle is therefore a verbatim concatenation of the
 * package stylesheets, in the import order the stylesheets' own header
 * comments document — the same order an application imports them, so the
 * cascade in a copy-in project matches a package consumer. The bundles carry
 * no content of their own: the package stylesheets are the specification, and
 * a hand edit to a bundle belongs upstream.
 *
 * The skill's .jsx assets are the opposite: hand-maintained standalone ports,
 * not derivable from the package sources, and marked AUTHORED in their own
 * headers. Only the CSS is generated.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { bannerLines } from './banner.mjs';

const REPO = dirname(dirname(dirname(fileURLToPath(import.meta.url))));

/**
 * Every bundle: the skill file it writes, the package stylesheets that feed
 * it, in order, and where in the app entry the skill tells a consumer to
 * import it.
 *
 * The order restates what the source headers document — fonts.css says
 * "import FIRST", base.css says "Import after tokens.css", charts/graph/code
 * say "Extracted from console.css", chat.css says "Import after
 * @forge/ui/styles.css". skill-css.test.mjs checks this list against those
 * headers, so an order change upstream fails the test rather than silently
 * shipping a bundle with a different cascade. Sources no header orders
 * (charts/graph/code against each other — disjoint class families) keep the
 * gallery's import order.
 */
export const BUNDLES = [
  {
    path: '.claude/skills/forge-design/assets/colors_and_type.css',
    note: 'Import at the app entry before console.css.',
    sources: [
      'packages/tokens/css/fonts.css',
      'packages/tokens/css/tokens.css',
      'packages/tokens/css/base.css',
    ],
  },
  {
    path: '.claude/skills/forge-design/assets/console.css',
    note: 'Import at the app entry after colors_and_type.css.',
    sources: [
      'packages/ui/styles/console.css',
      'packages/charts/styles/charts.css',
      'packages/graph/styles/graph.css',
      'packages/code/styles/code.css',
    ],
  },
  {
    path: '.claude/skills/forge-design/assets/chat.css',
    note: 'Import at the app entry after console.css.',
    sources: ['packages/chat/styles/chat.css'],
  },
];

const RULE = '='.repeat(73);

/** The banner, with every source listed under the first `Source:` line. */
function banner(bundle) {
  const [first, ...rest] = bundle.sources;
  const lines = bannerLines(first);
  const sourceAt = lines.findIndex((l) => l.startsWith('Source:'));
  lines.splice(sourceAt + 1, 0, ...rest.map((s) => `            ${s}`));
  lines.push('Concatenated in the import order the source headers document.');
  lines.push(bundle.note);
  return [`/* ${RULE}`, ...lines.map((l) => `   ${l}`), `   ${RULE} */`].join('\n');
}

/** The ruled line that opens each source's section of a bundle. */
export function sectionMarker(source) {
  return `/* ==== ${source} `.padEnd(76, '=') + ' */';
}

/** Render one bundle: banner, then each source verbatim under its marker. */
function renderBundle(bundle) {
  const sections = bundle.sources.map(
    (source) => `${sectionMarker(source)}\n\n${readFileSync(join(REPO, source), 'utf8')}`,
  );
  return `${banner(bundle)}\n\n${sections.join('\n')}`;
}

export const renderSkillColorsAndType = () => renderBundle(BUNDLES[0]);
export const renderSkillConsole = () => renderBundle(BUNDLES[1]);
export const renderSkillChat = () => renderBundle(BUNDLES[2]);
