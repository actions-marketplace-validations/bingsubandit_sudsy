// The page "How languages are measured" (/measured/): docs/languages.md rendered as it is, the generated blocks of
// numbers included, so that the page and the repository file never differ. Every language of the support table gets an
// anchor (`#lang-<slug>`, the slug of its language page) on its section; the Languages index links to them.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { base, siteData } from '../config.mjs';

const doc = fileURLToPath(new URL('../../docs/languages.md', import.meta.url));
const REPO = 'https://github.com/bingsubandit/sudsy/blob/main';
export const SLUG = 'measured';
// The anchor is `lang-<slug>`: a heading of the page may already have the id `<slug>` (`## Python`).
export const measuredLink = (slug) => `${base}/${SLUG}/#lang-${slug}`;

// The links of the page: the pages the site has stay on the site, the others point at the file on GitHub.
const onSite = {
  'configuration.md#presets': `${base}/reference/presets/`,
  '../README.md#language-support': `${base}/languages/`,
};
function links(text) {
  return text.replace(/\]\((?!https?:|#|\/)([^)\s]*)\)/g, (_, target) => {
    if (onSite[target]) return `](${onSite[target]})`;
    const [path, hash] = target.split('#');
    return `](${REPO}/${new URL(path, 'file:///docs/languages.md').pathname.slice(1)}${hash ? `#${hash}` : ''})`;
  });
}

export function measuredPage(put) {
  const { languages } = siteData();
  let text = links(readFileSync(doc, 'utf8')).replace(/^# .*\n+/, '');
  for (const l of languages) {
    const heading = `\n## ${l.section}\n`;
    if (!text.includes(heading)) throw new Error(`docs/languages.md has no section "${l.section}" for the language ${l.slug}`);
    text = text.replace(heading, `\n<div id="lang-${l.slug}"></div>\n\n## ${l.section}\n`);
  }
  put('measured.md', {
    title: 'How languages are measured',
    slug: SLUG,
    description: 'The reference tool, corpus, tool version, options and differences behind the numbers of every language.',
  }, text.trim());
  return languages.map((l) => l.slug);
}
