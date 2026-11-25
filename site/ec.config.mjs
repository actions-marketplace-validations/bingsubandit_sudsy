// Expressive Code plugin: `squiggles="line:column:length,..."` underlines the reported spans of a
// rule example with a red wavy line (site.css draws it); `tip="..."` is shown on hover.
import { definePlugin, ExpressiveCodeAnnotation } from 'astro-expressive-code';
import { h } from 'astro-expressive-code/hast';

class Squiggle extends ExpressiveCodeAnnotation {
  constructor(range, tip) {
    super({ inlineRange: range });
    this.tip = tip;
  }
  render({ nodesToTransform }) {
    return nodesToTransform.map((n) => h('span.squiggle', { title: this.tip }, [n]));
  }
}

const squiggles = () =>
  definePlugin({
    name: 'squiggles',
    baseStyles: '.squiggle{cursor:help}',
    hooks: {
      annotateCode: ({ codeBlock }) => {
        const spec = codeBlock.metaOptions.getString('squiggles');
        if (!spec) return;
        const tip = codeBlock.metaOptions.getString('tip') ?? '';
        // One underline per reported character: the spans of a line are merged where they overlap or touch.
        const byLine = new Map();
        for (const s of spec.split(',')) {
          const [line, column, length] = s.split(':').map(Number);
          const l = codeBlock.getLine(line - 1);
          if (!l) continue;
          const start = Math.min(column - 1, l.text.length);
          const end = Math.min(start + length, l.text.length);
          if (end > start) byLine.set(l, [...(byLine.get(l) ?? []), [start, end]]);
        }
        for (const [l, ranges] of byLine) {
          const merged = [];
          for (const [start, end] of ranges.sort((a, b) => a[0] - b[0] || a[1] - b[1])) {
            const last = merged[merged.length - 1];
            if (last && start <= last[1]) last[1] = Math.max(last[1], end);
            else merged.push([start, end]);
          }
          for (const [start, end] of merged) l.addAnnotation(new Squiggle({ columnStart: start, columnEnd: end }, tip));
        }
      },
    },
  });

// Code block languages are Sudsy's language ids; Shiki has no EditorConfig grammar, INI fits it.
export default { plugins: [squiggles()], shiki: { langAlias: { editorconfig: 'ini' } } };
