// Language pages (/languages/<id>/), tool pages (/compare/<tool>/) and the check that every rule, language
// and tool has a page. Source: docs/ai/parity/site.json (cargo xtask codegen; the README support table,
// the speed table and the parity catalogs), next to rules.json.
import { existsSync } from 'node:fs';
import { base, rules, siteData } from '../config.mjs';

const pct = (v) => (v == null ? 'not measured' : `${Math.floor(v)}%`);
const link = (path, text) => `[${text}](${base}/${path}/)`;
const tick = (t) => '`' + t + '`';

// Names shown for a tool: the form rules.json uses (`StyleCop`), else the catalog id.
export function names() {
  const { languages, tools } = siteData();
  const toolName = new Map(tools.map((t) => [t.tool, t.tool]));
  for (const r of rules()) for (const u of r.upstream) toolName.set(u.tool.toLowerCase(), u.tool);
  const langName = new Map(languages.flatMap((l) => l.ids.map((i) => [i, l])));
  return { toolName, langName, tools: new Map(tools.map((t) => [t.tool, t])) };
}

// What Sudsy has for one catalog entry: `[id, coverage, status, gap]`.
function sudsy([, coverage, status, gap]) {
  const [kind, arg] = [status.split(': ')[0], status.split(': ').slice(1).join(': ')];
  const what = {
    rule: () => arg.split(', ').map((r) => link(`rules/${r}`, tick(r))).join(', '),
    option: () => `option ${tick(arg)}`,
    sdk: () => `.NET SDK ${tick(arg)}`,
    config: () => `configuration: ${arg}`,
    wontfix: () => '-',
    planned: () => '-',
    deferred: () => '-',
    formatter: () => 'formatter',
  }[kind]();
  const label = { covered: 'covered', partial: 'partial', missing: 'not yet', sdk: 'SDK', wontfix: 'not planned', deferred: 'deferred' }[coverage];
  return [what, label, kind === 'wontfix' ? arg : gap];
}

export function pages({ put, cell, table }) {
  const { languages, tools } = siteData();
  const { toolName, langName, tools: byTool } = names();
  const show = (t) => toolName.get(t) ?? t;
  const written = { languages: [], tools: [] };

  for (const t of tools) {
    const n = (c) => t.entries.filter((e) => e[1] === c).length;
    const langs = t.languages.map((l) => langName.get(l)).filter(Boolean);
    const measured = t.coverage == null
      ? `Sudsy has not been measured against ${show(t.tool)} on a corpus yet.`
      : `On real-world code, Sudsy with the preset that mirrors ${show(t.tool)} also reports ${pct(t.coverage)} of the findings of ${show(t.tool)}'s default set${t.target == null ? '' : ` (${pct(t.target)} of the part in scope)`}.`;
    const body = [
      `${show(t.tool)}${t.version ? ` ${t.version}` : ''} has ${t.entries.length} rules and settings in the parity catalog. Sudsy covers ${n('covered')} of them and ${n('partial')} in part; ${n('missing')} are planned and ${n('deferred')} deferred. ${measured}`,
      langs.length && 'Languages: ' + [...new Map(langs.map((l) => [l.slug, l])).values()].map((l) => link(`languages/${l.slug}`, l.name)).join(', ') + '.',
      table(['Id', 'Sudsy', 'Coverage', 'Note'], t.entries.map((e) => [tick(e[0]).replace(/\|/g, '\\|'), ...sudsy(e).map(cell)])),
    ].filter(Boolean).join('\n\n');
    put(`compare-${t.tool}.md`, {
      title: `${show(t.tool)} rules in Sudsy`,
      slug: `compare/${t.tool}`,
      description: `Every ${show(t.tool)} rule with the Sudsy rule that covers it and how much of ${show(t.tool)}'s output Sudsy reproduces.`,
      head: [{ tag: 'title', content: `${show(t.tool)} rules and their Sudsy equivalents | Sudsy` }],
    }, body);
    written.tools.push(t.tool);
  }
  put('compare-index.md', {
    title: 'Compared tools', slug: 'compare',
    description: 'The linters and formatters Sudsy is measured against, with the share of their output it reproduces.',
  }, 'Every tool Sudsy is measured against: its rules and settings, the Sudsy rule for each, and the measured coverage.\n\n' +
    table(['Tool', 'Entries', 'Coverage of its default set', 'Languages'], tools.map((t) =>
      [link(`compare/${t.tool}`, show(t.tool)), t.entries.length, pct(t.coverage), t.languages.join(', ')])));

  for (const l of languages) {
    const tool = (x) => (x.catalog && byTool.has(x.catalog) ? link(`compare/${x.catalog}`, x.name) : x.name);
    const parity = [
      ...l.format.map((x) => ['Format', tool(x), x.share == null ? 'not measured' : `${pct(x.share)} of the files come out identical`]),
      ...l.lint.map((x) => ['Lint', tool(x), x.share == null ? 'not measured' : `${pct(x.share)} of the findings are also reported by Sudsy`]),
    ];
    const body = [
      `Status: **${l.status}**. Each number is measured on real-world code against the reference tool with its default settings. \`Ready\` means every number is at least 85%.`,
      '## Parity',
      parity.length ? table(['Task', 'Reference tool', 'Match'], parity) : `Sudsy has ${l.rules} rules for this language; no reference tool was found to compare with.`,
      l.speed.length && '## Speed\n\n' + table(['Task', 'Compared with', 'Sudsy speed', 'Time (Sudsy / tool)', 'Peak memory (Sudsy / tool)'], l.speed.map((s) => s.slice(1).map(cell))),
      l.presets.length && '## Presets\n\nA preset is an ordinary `sudsy.yaml`; use one with `extends`.\n\n' + l.presets.map((p) => `- [${tick(p)}](${base}/reference/presets/#${p.replace('/', '')}): \`extends: [preset:${p}]\``).join('\n'),
      `## Rules\n\n${l.rules} lint rules apply to ${l.name}: [browse them](${base}/rules/?language=${l.ids[0]}).`,
    ].filter(Boolean).join('\n\n');
    const against = [...new Set([...l.format, ...l.lint].map((x) => x.name))];
    put(`language-${l.slug}.md`, {
      title: l.name,
      slug: `languages/${l.slug}`,
      description: `Sudsy formats and lints ${l.name}: ${l.status.toLowerCase()}${against.length ? `, measured against ${against.join(', ')}` : ''}.`,
      head: [{ tag: 'title', content: `${l.name} formatter and linter${against.length ? ` (${against.join(', ')} parity)` : ''} | Sudsy` }],
    }, body);
    written.languages.push(l.slug);
  }
  put('language-index.md', {
    title: 'Languages', slug: 'languages',
    description: 'Every language Sudsy formats and lints, with its status against the reference tools.',
  }, table(['Language', 'Format', 'Lint', 'Status'], languages.map((l) => [
    link(`languages/${l.slug}`, l.name),
    l.format.map((x) => `${pct(x.share)} ${x.name}`).join('<br>') || 'none',
    l.lint.map((x) => `${pct(x.share)} ${x.name}`).join('<br>') || `${l.rules} rules`,
    l.status,
  ])));
  return written;
}

// The pages exist: one per rule, per language of a rule, per tool.
export function check(out, written) {
  const { languages, tools } = siteData();
  const missing = [
    ...rules().filter((r) => !existsSync(`${out}rule-${r.id}.md`)).map((r) => `rule ${r.id}`),
    ...rules().flatMap((r) => r.languages.filter((l) => l !== 'all' && !languages.some((x) => x.ids.includes(l))).map((l) => `language ${l} (rule ${r.id})`)),
    ...languages.filter((l) => !written.languages.includes(l.slug)).map((l) => `language ${l.slug}`),
    ...tools.filter((t) => !written.tools.includes(t.tool)).map((t) => `tool ${t.tool}`),
  ];
  if (missing.length) throw new Error(`no page for: ${missing.join(', ')}`);
}
