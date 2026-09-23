import { describe, expect, it } from 'vitest';
import { parseDiff, splitDiff } from './diff';

describe('unified Git patch rendering', () => {
  it('preserves line numbers across separated hunks', () => {
    const rows = parseDiff(
      '--- a/a\n+++ b/a\n@@ -2,2 +2,3 @@\n same\n-old\n+new\n+extra\n@@ -20 +21 @@\n-tail\n+end\n',
    );
    expect(rows.filter((r) => r.kind === 'add').map((r) => r.next)).toEqual([3, 4, 21]);
    expect(rows.filter((r) => r.kind === 'remove').map((r) => r.old)).toEqual([3, 20]);
    expect(rows.filter((r) => r.kind === 'hunk').map((r) => r.hunk)).toEqual([0, 1]);
  });
  it('aligns unequal replacement blocks without losing lines', () => {
    const rows = splitDiff(parseDiff('@@ -1,2 +1,3 @@\n-old\n+new\n+extra\n same\n'));
    expect(rows).toHaveLength(4);
    expect(rows[1].left?.content).toBe('old');
    expect(rows[1].right?.content).toBe('new');
    expect(rows[2].left).toBeUndefined();
    expect(rows[2].right?.content).toBe('extra');
    expect(rows[3].left?.old).toBe(2);
    expect(rows[3].right?.next).toBe(3);
  });
  it('does not mistake added code beginning with ++ for a file header', () => {
    const rows = parseDiff('@@ -0,0 +1 @@\n+++counter;\n');
    expect(rows[1]).toMatchObject({ kind: 'add', content: '++counter;', next: 1 });
  });
});
