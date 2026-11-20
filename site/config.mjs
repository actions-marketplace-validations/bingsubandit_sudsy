// Shared by astro.config.mjs and scripts/generate.mjs.
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

export const site = 'https://bingsubandit.github.io';
export const base = '/sudsy';
export const rulesPath = process.env.SUDSY_RULES_JSON ?? fileURLToPath(new URL('../docs/ai/rules/rules.json', import.meta.url));
export const rules = () => JSON.parse(readFileSync(rulesPath, 'utf8')).rules;
// Languages (support table, speed rows, presets) and compared tools (parity catalogs): cargo xtask codegen.
export const siteDataPath = fileURLToPath(new URL('../docs/ai/parity/site.json', import.meta.url));
export const siteData = () => JSON.parse(readFileSync(siteDataPath, 'utf8'));
