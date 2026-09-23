<script lang="ts">
  import { ArrowsClockwise, GitBranch, MagnifyingGlass, X } from 'phosphor-svelte';
  import type { Commit } from '../lib/types';
  import { layoutGraph, GRAPH_ROW_HEIGHT } from '../lib/graph';

  let {
    commits,
    selected,
    loading,
    hasMore,
    remoteNames,
    all = $bindable(true),
    onselect,
    onmore,
    onscope,
    onrefresh,
  }: {
    commits: Commit[];
    selected: string | null;
    loading: boolean;
    hasMore: boolean;
    remoteNames: string[];
    all?: boolean;
    onselect: (commit: Commit) => void;
    onmore: () => void;
    onscope: () => void;
    onrefresh: () => void;
  } = $props();

  let search = $state('');
  let scrollTop = $state(0);
  let viewportHeight = $state(540);
  let viewport = $state<HTMLDivElement>();
  const rowHeight = GRAPH_ROW_HEIGHT;
  const colors = ['#83bc95', '#7c9fc9', '#cca871', '#b79bc8', '#73b9ae', '#c98f9b', '#acb579'];
  let graph = $derived(layoutGraph(commits));
  let start = $derived(Math.max(0, Math.min(graph.rows.length - 1, Math.floor(scrollTop / rowHeight)) - 8));
  let end = $derived(Math.min(graph.rows.length, start + Math.ceil(viewportHeight / rowHeight) + 16));
  let matchCount = $derived(commits.filter((item) => matches(item)).length);

  function matches(commit: Commit) {
    const value = search.trim().toLocaleLowerCase();
    return (
      !value ||
      `${commit.subject} ${commit.author} ${commit.oid} ${commit.refs}`.toLocaleLowerCase().includes(value)
    );
  }

  function refLabels(refs: string) {
    return refs
      .split(', ')
      .filter(Boolean)
      .map((raw) => {
        const tag = raw.startsWith('tag: ');
        const head = raw.startsWith('HEAD -> ');
        const label = raw.replace(/^tag: /, '').replace(/^HEAD -> /, '');
        const remote = remoteNames.some((name) => label.startsWith(`${name}/`));
        return { label, kind: tag ? 'tag' : head ? 'head' : remote ? 'remote' : 'branch' };
      });
  }

  function findNext() {
    if (!search.trim()) return;
    const next = commits.findIndex((commit, index) => index * rowHeight > scrollTop + 1 && matches(commit));
    const first = commits.findIndex(matches);
    const index = next < 0 ? first : next;
    if (index >= 0 && viewport) viewport.scrollTop = index * rowHeight;
  }
</script>

<section class="graph-view" aria-label="Git 提交图" style={`--graph-width:${graph.width}px`}>
  <div class="graph-titlebar">
    <div>
      <span class="graph-kicker">REPOSITORY HISTORY</span>
      <h2>提交图</h2>
    </div>
    <div class="graph-options">
      <label for="graph-scope">显示</label>
      <select id="graph-scope" bind:value={all} onchange={onscope}>
        <option value={true}>所有分支</option>
        <option value={false}>当前分支</option>
      </select>
      <button
        class="graph-refresh"
        onclick={onrefresh}
        disabled={loading}
        aria-label="刷新提交图"
        title="刷新提交图"><ArrowsClockwise size={16} class={loading ? 'spinning' : ''} /></button
      >
    </div>
  </div>
  <div class="graph-search">
    <MagnifyingGlass size={15} />
    <input
      id="graph-search"
      aria-label="查找已加载提交"
      bind:value={search}
      placeholder="查找提交、作者、分支或哈希…"
      onkeydown={(event) => {
        if (event.key === 'Enter') findNext();
      }}
    />
    {#if search}<span class="graph-match-count">{matchCount} 个匹配</span><button
        onclick={() => (search = '')}
        aria-label="清除查找"><X size={13} /></button
      >{/if}
  </div>
  <div class="graph-head">
    <span>GRAPH</span><span>提交说明</span><span>作者</span><span>日期</span><span>哈希</span>
  </div>
  <div
    class="graph-viewport"
    bind:this={viewport}
    bind:clientHeight={viewportHeight}
    onscroll={(event) => (scrollTop = event.currentTarget.scrollTop)}
  >
    {#if loading && !commits.length}
      <p class="graph-empty">正在读取提交历史…</p>
    {:else if !commits.length}
      <div class="graph-empty">
        <GitBranch size={28} /><strong>还没有提交</strong><span>第一次提交完成后，分支轨迹会出现在这里。</span
        >
      </div>
    {:else}
      <div class="graph-spacer" style:height={`${graph.rows.length * rowHeight + (hasMore ? 52 : 0)}px`}>
        <div style:transform={`translateY(${start * rowHeight}px)`}>
          {#each graph.rows.slice(start, end) as row (row.commit.oid)}
            <button
              class="graph-row"
              class:selected={selected === row.commit.oid}
              class:dimmed={!!search && !matches(row.commit)}
              onclick={() => onselect(row.commit)}
              title={`${row.commit.subject}\n${row.commit.oid}`}
            >
              <svg
                class="graph-lines"
                width={graph.width}
                height={rowHeight}
                viewBox={`0 0 ${graph.width} ${rowHeight}`}
                aria-hidden="true"
              >
                {#each row.paths as path}<path
                    d={path.d}
                    stroke={colors[path.color]}
                    stroke-width="2"
                    fill="none"
                    stroke-linecap="round"
                  />{/each}
                <circle
                  cx={14 + row.lane * 18}
                  cy={rowHeight / 2}
                  r="5.5"
                  fill={colors[row.color]}
                  stroke="var(--editor)"
                  stroke-width="2"
                />
              </svg>
              <span class="graph-subject"
                ><span>{row.commit.subject}</span>{#each refLabels(row.commit.refs) as ref}<span
                    class={`graph-ref ${ref.kind}`}>{ref.label}</span
                  >{/each}</span
              >
              <span class="graph-author">{row.commit.author}</span>
              <time>{new Date(row.commit.date).toLocaleDateString('zh-CN')}</time>
              <code>{row.commit.short}</code>
            </button>
          {/each}
        </div>
        {#if hasMore}<button
            class="graph-more"
            style:top={`${graph.rows.length * rowHeight}px`}
            disabled={loading}
            onclick={onmore}>{loading ? '正在加载…' : '加载更早的提交'}</button
          >{/if}
      </div>
    {/if}
  </div>
  <div class="graph-status"><span>{commits.length} 条提交已加载</span><span>点击提交查看差异</span></div>
</section>

<style>
  .graph-view {
    display: flex;
    flex-direction: column;
    min-width: 0;
    min-height: 0;
    height: 100%;
    background: var(--editor);
    border-right: 1px solid var(--border);
  }
  .graph-titlebar {
    height: 78px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 0 22px;
    border-bottom: 1px solid var(--border);
  }
  .graph-kicker {
    font:
      9px Consolas,
      monospace;
    letter-spacing: 1.4px;
    color: var(--faint);
  }
  h2 {
    margin: 5px 0 0;
    font-size: 19px;
    font-weight: 500;
  }
  .graph-options {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
  }
  .graph-options select {
    width: 105px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text);
    background: var(--panel);
    padding: 6px;
    font: inherit;
  }
  .graph-refresh {
    padding: 5px;
    border-radius: 4px;
    color: var(--muted);
  }
  .graph-refresh:hover {
    background: var(--hover);
    color: var(--text);
  }
  .graph-search {
    height: 44px;
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: 9px;
    padding: 0 21px;
    border-bottom: 1px solid var(--border);
    color: var(--faint);
  }
  .graph-search input {
    flex: 1;
    border: 0;
    min-width: 0;
    background: transparent;
    font-size: 11px;
    outline: 0;
  }
  .graph-search input:focus {
    border: 0;
  }
  .graph-search button {
    padding: 4px;
    color: var(--faint);
  }
  .graph-match-count {
    font:
      10px Consolas,
      monospace;
    white-space: nowrap;
  }
  .graph-head,
  .graph-row {
    display: grid;
    grid-template-columns: var(--graph-width) minmax(135px, 1fr) 94px 82px 66px;
    align-items: center;
    min-width: max(100%, calc(var(--graph-width) + 385px));
  }
  .graph-head {
    flex-shrink: 0;
    height: 30px;
    padding: 0 15px;
    border-bottom: 1px solid var(--border);
    font:
      9px Consolas,
      monospace;
    letter-spacing: 0.5px;
    color: var(--faint);
  }
  .graph-viewport {
    flex: 1;
    min-height: 0;
    overflow: auto;
    contain: strict;
  }
  .graph-spacer {
    position: relative;
    min-width: 100%;
    width: 100%;
  }
  .graph-row {
    height: 40px;
    width: 100%;
    padding: 0 15px;
    border: 0;
    box-shadow: inset 0 -1px var(--border);
    background: transparent;
    text-align: left;
    font-size: 11px;
    color: var(--muted);
    transition:
      background 0.12s,
      opacity 0.12s;
  }
  .graph-row:hover {
    background: var(--hover);
  }
  .graph-row.selected {
    background: var(--selected);
    box-shadow:
      inset 2px 0 var(--accent),
      inset 0 -1px var(--border);
    color: var(--text);
  }
  .graph-row.dimmed {
    opacity: 0.35;
  }
  .graph-lines {
    display: block;
    overflow: visible;
  }
  .graph-subject {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 6px;
    overflow: hidden;
    white-space: nowrap;
  }
  .graph-subject > span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    flex-shrink: 1;
    min-width: 55px;
    color: var(--text);
  }
  .graph-author,
  time {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
    color: var(--faint);
    font-size: 10px;
  }
  .graph-row code {
    color: var(--faint);
    font-size: 10px;
  }
  .graph-ref {
    flex-shrink: 0;
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    padding: 2px 6px;
    border-radius: 3px;
    font:
      9px Consolas,
      monospace;
    background: var(--accent-soft);
    border: 1px solid var(--accent-border);
    color: var(--accent);
  }
  .graph-ref.remote {
    background: #769bc01a;
    border-color: #769bc047;
    color: var(--type-color);
  }
  .graph-ref.tag {
    background: #c8a66e1c;
    border-color: #c8a66e4d;
    color: var(--warm);
  }
  .graph-ref.head {
    font-weight: 700;
  }
  .graph-more {
    position: absolute;
    left: 0;
    right: 0;
    height: 52px;
    width: 100%;
    justify-content: center;
    color: var(--accent);
    font-size: 11px;
  }
  .graph-more:hover {
    background: var(--hover);
  }
  .graph-empty {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 13px;
    color: var(--faint);
    text-align: center;
    font-size: 11px;
  }
  .graph-empty strong {
    color: var(--muted);
    font-size: 13px;
    font-weight: 500;
  }
  .graph-status {
    display: flex;
    align-items: center;
    justify-content: space-between;
    flex-shrink: 0;
    height: 29px;
    padding: 0 18px;
    border-top: 1px solid var(--border);
    font-size: 9px;
    color: var(--faint);
  }
  .spinning {
    animation: turn 1s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (max-width: 1180px) {
    .graph-head,
    .graph-row {
      grid-template-columns: var(--graph-width) minmax(110px, 1fr) 65px 0 0;
    }
    .graph-head > :nth-child(4),
    .graph-head > :nth-child(5),
    .graph-row > :nth-child(4),
    .graph-row > :nth-child(5) {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    *,
    *:before,
    *:after {
      animation: none !important;
      transition: none !important;
    }
  }
</style>
