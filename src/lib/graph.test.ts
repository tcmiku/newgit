import { describe, expect, it } from 'vitest';
import { layoutGraph } from './graph';
import type { Commit } from './types';

const commit = (oid: string, parents: string[] = []): Commit => ({
  oid,
  short: oid,
  parents,
  subject: oid,
  author: 'Test',
  date: '2026-09-23T00:00:00Z',
  refs: '',
});

describe('commit graph layout', () => {
  it('keeps a linear chain in one lane', () => {
    const graph = layoutGraph([commit('c', ['b']), commit('b', ['a']), commit('a')]);
    expect(graph.rows.map((row) => row.lane)).toEqual([0, 0, 0]);
    expect(graph.rows.map((row) => row.incoming)).toEqual([false, true, true]);
    expect(graph.width).toBe(28);
  });

  it('opens two parent lanes at a merge and joins them at their ancestor', () => {
    const graph = layoutGraph([
      commit('merge', ['left', 'right']),
      commit('left', ['base']),
      commit('right', ['base']),
      commit('base'),
    ]);
    expect(graph.rows[0].parents).toEqual([0, 1]);
    expect(graph.rows[2].parents).toEqual([0]);
    expect(graph.rows[3].lane).toBe(0);
    expect(graph.width).toBe(46);
  });

  it('starts unrelated tips without drawing a false incoming line', () => {
    const graph = layoutGraph([commit('top', ['base']), commit('independent'), commit('base')]);
    expect(graph.rows[1]).toMatchObject({ lane: 1, incoming: false, parents: [] });
    expect(graph.rows[1].paths.every((path) => !path.d.includes('M 32 0 L 32 18'))).toBe(true);
  });

  it('does not change already loaded rows when another page arrives', () => {
    const first = [commit('merge', ['left', 'right']), commit('left', ['base'])];
    const later = [...first, commit('right', ['base']), commit('base')];
    expect(layoutGraph(first).rows.map((row) => row.paths)).toEqual(
      layoutGraph(later).rows.slice(0, first.length).map((row) => row.paths),
    );
  });
});
