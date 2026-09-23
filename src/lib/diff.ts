export interface DiffLine {
  kind: 'context' | 'add' | 'remove' | 'hunk' | 'meta';
  content: string;
  old?: number;
  next?: number;
  hunk?: number;
}

export interface SplitLine {
  left?: DiffLine;
  right?: DiffLine;
  header?: DiffLine;
}

export function parseDiff(patch: string): DiffLine[] {
  let old = 0,
    next = 0,
    hunk = -1,
    inHunk = false;
  const result: DiffLine[] = [];
  const lines = patch.split('\n');
  if (lines.at(-1) === '') lines.pop();
  for (const line of lines) {
    const header = /^@@ -(\d+)(?:,\d+)? \+(\d+)(?:,\d+)? @@/.exec(line);
    if (header) {
      old = Number(header[1]);
      next = Number(header[2]);
      inHunk = true;
      hunk++;
      result.push({ kind: 'hunk', content: line, hunk });
    } else if (line.startsWith('diff --git ')) {
      inHunk = false;
      result.push({ kind: 'meta', content: line.replace('diff --git ', '') });
    } else if (inHunk && line.startsWith('+')) {
      result.push({ kind: 'add', content: line.slice(1), next: next++ });
    } else if (inHunk && line.startsWith('-')) {
      result.push({ kind: 'remove', content: line.slice(1), old: old++ });
    } else if (inHunk && line.startsWith(' ')) {
      result.push({ kind: 'context', content: line.slice(1), old: old++, next: next++ });
    } else if (line.startsWith('\\')) {
      result.push({ kind: 'meta', content: '文件末尾无换行符' });
    } else if (!/^(index |--- |\+\+\+ |new file mode |deleted file mode )/.test(line) && line.trim()) {
      result.push({ kind: 'meta', content: line });
    }
  }
  return result;
}

export function splitDiff(lines: DiffLine[]): SplitLine[] {
  const rows: SplitLine[] = [];
  let removals: DiffLine[] = [],
    additions: DiffLine[] = [];
  const flush = () => {
    for (let i = 0; i < Math.max(removals.length, additions.length); i++)
      rows.push({ left: removals[i], right: additions[i] });
    removals = [];
    additions = [];
  };
  for (const line of lines) {
    if (line.kind === 'remove') {
      if (additions.length) flush();
      removals.push(line);
    } else if (line.kind === 'add') additions.push(line);
    else {
      flush();
      if (line.kind === 'context') rows.push({ left: line, right: line });
      else rows.push({ header: line });
    }
  }
  flush();
  return rows;
}

export function tokens(code: string) {
  return code
    .split(
      /((?:"[^"\n]*"|'[^'\n]*'|`[^`\n]*`)|\/\/.*$|\b(?:const|let|function|return|import|from|export|default|if|else|async|await|true|false|null|interface|type|new)\b|\b\d+\b)/g,
    )
    .filter(Boolean)
    .map((text) => ({
      text,
      kind: /^["'`]/.test(text)
        ? 'string'
        : text.startsWith('//')
          ? 'comment'
          : /^(const|let|function|return|import|from|export|default|if|else|async|await|true|false|null|interface|type|new)$/.test(
                text,
              )
            ? 'keyword'
            : /^\d+$/.test(text)
              ? 'number'
              : '',
    }));
}
