// Rule list of a built site: page weight, time to the full list, first filter and search. `scripts/site.sh list-bench [PORT]`
// serves the build and runs this (SITE_OUT as for the build); PORT is the one `astro preview` listens on.
import { chromium } from 'playwright-core';
const B = `http://localhost:${process.argv[2]}/sudsy`;
let browser;
for (const options of [{ channel: 'msedge' }, { channel: 'chrome' }, {}]) {
  try { browser = await chromium.launch(options); break; } catch {}
}
const ctx = await browser.newContext();
const page = await ctx.newPage();
let bytes = 0;
page.on('response', async (r) => { try { const b = await r.body(); bytes += b.length; } catch {} });
const t0 = Date.now();
await page.goto(`${B}/rules/`, { waitUntil: 'networkidle' });
await page.waitForFunction(() => { const m = /^(\d+) of \d+ rules/.exec(document.querySelector('#rule-search .count')?.textContent ?? ''); return m && document.querySelector('#rule-search')?.dataset.rendered === m[1]; });
const load = Date.now() - t0;
const total = await page.locator('#rule-search .count').innerText();
const times = [];
for (const lang of ['yaml', 'c#', 'python']) {
  const m = page.locator('.rs-menu', { has: page.locator('button', { hasText: /^Language/ }) });
  await m.locator('button').first().click();
  await m.locator('input[type=search]').fill(lang);
  const t = Date.now();
  await m.locator('li:not([hidden]) input').first().check();
  await page.waitForFunction(() => { const m = /^(\d+) of \d+ rules/.exec(document.querySelector('#rule-search .count').textContent); return m && document.querySelector('#rule-search').dataset.rendered === m[1]; });
  times.push(Date.now() - t);
  await page.keyboard.press('Escape');
  await page.locator('.rs-clear').click();
}
const q = Date.now();
await page.fill('#rule-search .rs-q', 'trailing');
await page.waitForFunction(() => { const m = /^(\d+) of \d+ rules/.exec(document.querySelector('#rule-search .count').textContent); return m && document.querySelector('#rule-search').dataset.rendered === m[1]; });
console.log(JSON.stringify({ total, loadMs: load, firstFilterMs: times, searchMs: Date.now() - q, weightKB: Math.round(bytes / 1024), rows: await page.locator('#rule-search tbody tr').count() }));
await browser.close();
