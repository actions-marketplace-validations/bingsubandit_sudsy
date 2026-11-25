// Link check of the upstream rule pages: every `rule-url` of the parity catalogs, resolved in
// docs/ai/parity/site.json and docs/ai/rules/rules.json. A URL passes when it answers 200 after
// redirects and, for a `#anchor`, the page has that anchor. Run by hand and nightly (never in the gate):
// `scripts/site.sh check-links [tool...]`. Needs the network; pages are read as data only.
import { rules, siteData } from '../config.mjs';

const only = new Set(process.argv.slice(2));
const UA = 'Mozilla/5.0 (X11; Linux x86_64) sudsy-link-check';
const PER_HOST = 4;

// url -> tools that use it
const urls = new Map();
const add = (tool, url) => url && urls.set(url, (urls.get(url) ?? new Set()).add(tool));
for (const t of siteData().tools) if (!only.size || only.has(t.tool)) for (const e of t.entries) add(t.tool, e[4]);
if (!only.size) for (const r of rules()) for (const u of r.upstream) add(u.tool.toLowerCase(), u.url);

// GitHub renders Markdown anchors from headings; a blob URL is read from raw.githubusercontent.com.
const slug = (h) => h.trim().toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, '').replace(/\s/g, '-');
const anchors = (text, markdown) => {
  const set = new Set([...text.matchAll(/\b(?:id|name)="(?:user-content-)?([^"]+)"/g)].map((m) => m[1]));
  if (markdown) for (const m of text.matchAll(/^#{1,6}\s+(.+?)\s*#*\s*$/gm)) set.add(slug(m[1].replace(/\[([^\]]*)\]\([^)]*\)|\[([^\]]*)\]\[\]|[`*]/g, '$1$2')));
  return set;
};
const source = (page) => {
  const m = page.match(/^https:\/\/github\.com\/([^/]+\/[^/]+)\/blob\/(.+)$/);
  return m ? [`https://raw.githubusercontent.com/${m[1]}/${m[2]}`, true] : [page, false];
};

const cache = new Map(); // page -> Promise<{ status, anchors }>
async function load(page) {
  const [src, markdown] = source(page);
  for (let attempt = 0; ; attempt++) {
    try {
      const res = await fetch(src, { headers: { 'user-agent': UA }, signal: AbortSignal.timeout(30000) });
      if ((res.status === 429 || res.status >= 500) && attempt < 2) { await new Promise((r) => setTimeout(r, 3000 * (attempt + 1))); continue; }
      return { status: res.status, text: res.status === 200 ? await res.text() : '', markdown };
    } catch (e) {
      if (attempt >= 2) return { status: `error ${e.cause?.code ?? e.message}`, text: '', markdown };
    }
  }
}
const page = (p) => cache.get(p) ?? cache.set(p, load(p)).get(p);

const failures = [];
const queue = [...urls.keys()];
const perHost = new Map();
async function worker() {
  while (queue.length) {
    const url = queue.shift();
    const host = new URL(url).host;
    while ((perHost.get(host) ?? 0) >= PER_HOST) await new Promise((r) => setTimeout(r, 50));
    perHost.set(host, (perHost.get(host) ?? 0) + 1);
    const [p, hash] = url.split('#');
    const res = await page(p);
    perHost.set(host, perHost.get(host) - 1);
    const fail = res.status !== 200 ? `status ${res.status}` : hash && !anchors(res.text, res.markdown).has(decodeURIComponent(hash)) ? 'anchor missing' : '';
    if (fail) failures.push({ url, fail, tools: [...urls.get(url)] });
    // Free the page when no later URL needs it.
    if (!queue.some((u) => u.split('#')[0] === p)) cache.delete(p);
  }
}
await Promise.all(Array.from({ length: 24 }, worker));

const byTool = new Map();
for (const f of failures) for (const t of f.tools) byTool.set(t, [...(byTool.get(t) ?? []), f]);
for (const [tool, list] of [...byTool].sort()) {
  console.log(`${tool}: ${list.length} failed`);
  for (const f of list) console.log(`  ${f.fail}: ${f.url}`);
}
console.log(`${urls.size} URLs, ${failures.length} failed`);
process.exit(failures.length ? 1 : 0);
