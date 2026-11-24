// Shared by the generators that turn a tool's element metadata into data files (the HTML ones are
// crates/lang-html/data/generate*.mjs): grouping of names by value, the row text, and line wrapping.

/** Map value -> items, in order; an item whose value is null or undefined is left out. */
export function groupBy(items, valueOf) {
  const groups = new Map();
  for (const item of items) {
    const value = valueOf(item);
    if (value === null || value === undefined) continue;
    if (!groups.has(value)) groups.set(value, []);
    groups.get(value).push(item);
  }
  return groups;
}

/** The lines `kind name,name value` of a grouping (a value of "" gives `kind name,name`). */
export function rows(kind, groups) {
  return [...groups].map(([value, names]) => `${kind} ${names.join(",")} ${value}`.trimEnd());
}

// Value text of the rows: items separated by spaces, an empty list is `-`.
export const list = (items) => items.join(" ") || "-";
// Alternatives inside one item are written `a|b` (the metadata has `a, b`).
export const alternatives = (items) => items.map((i) => i.replace(/, /g, "|")).join(" ");
// The names a list of `{exclude: x | [x]}` rules forbids, or null when there are none.
export const excluded = (rules) => {
  const names = rules.flatMap((r) => (r && r.exclude !== undefined ? [r.exclude].flat() : []));
  return names.length ? names.join(" ") : null;
};

/** Words wrapped into lines of at most `width` characters. */
export function wrap(words, width = 98) {
  const lines = [];
  let line = "";
  for (const word of words) {
    if (line.length + word.length > width) {
      lines.push(line.trim());
      line = "";
    }
    line += word + " ";
  }
  if (line) lines.push(line.trim());
  return lines;
}
