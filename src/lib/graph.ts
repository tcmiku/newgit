import type { Commit } from './types';

export const GRAPH_ROW_HEIGHT = 40;

export interface GraphRow {
  commit: Commit;
  lane: number;
  color: number;
  incoming: boolean;
  parents: number[];
  paths: { d: string; color: number }[];
}

const laneX = (lane: number) => 14 + lane * 18;

export function layoutGraph(commits: Commit[]): { rows: GraphRow[]; width: number } {
  let lanes: { oid: string; color: number }[] = [];
  let nextColor = 0;
  let maxLanes = 1;
  const rows: GraphRow[] = [];

  for (const commit of commits) {
    let lane = lanes.findIndex((item) => item.oid === commit.oid);
    const incoming = lane >= 0;
    if (!incoming) {
      lane = lanes.length;
      lanes.push({ oid: commit.oid, color: nextColor++ % 7 });
    }
    const before = lanes.map((item) => ({ ...item }));
    const color = lanes[lane].color;
    lanes.splice(lane, 1);
    const parents: number[] = [];
    let insertAt = lane;
    for (const [index, oid] of [...new Set(commit.parents)].entries()) {
      let target = lanes.findIndex((item) => item.oid === oid);
      if (target < 0) {
        target = Math.min(insertAt, lanes.length);
        lanes.splice(target, 0, { oid, color: index === 0 ? color : nextColor++ % 7 });
      }

      insertAt = target + 1;
    }

    parents.push(...[...new Set(commit.parents)].map((oid) => lanes.findIndex((item) => item.oid === oid)));

    maxLanes = Math.max(maxLanes, before.length, lanes.length);
    const paths: GraphRow['paths'] = [];
    for (const [from, active] of before.entries()) {
      if (from === lane) continue;
      const to = lanes.findIndex((item) => item.oid === active.oid);
      if (to >= 0) {
        paths.push({
          d: `M ${laneX(from)} 0 C ${laneX(from)} 20, ${laneX(to)} 20, ${laneX(to)} 40`,
          color: active.color,
        });
      }
    }
    if (incoming) {
      paths.push({ d: `M ${laneX(lane)} 0 L ${laneX(lane)} 20`, color });
    }
    for (const target of parents) {
      paths.push({
        d: `M ${laneX(lane)} 20 C ${laneX(lane)} 30, ${laneX(target)} 30, ${laneX(target)} 40`,
        color: lanes[target].color,
      });
    }
    rows.push({ commit, lane, color, incoming, parents, paths });
  }
  return { rows, width: 28 + (maxLanes - 1) * 18 };
}
