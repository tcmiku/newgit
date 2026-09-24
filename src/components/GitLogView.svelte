<script lang="ts">
  import { ArrowsClockwise, GitBranch, GitCommit } from 'phosphor-svelte';

  let {
    output,
    loading,
    hasMore,
    limit,
    all = $bindable(true),
    ongraph,
    onscope,
    onrefresh,
    onmore,
  }: {
    output: string;
    loading: boolean;
    hasMore: boolean;
    limit: number;
    all?: boolean;
    ongraph: () => void;
    onscope: () => void;
    onrefresh: () => void;
    onmore: () => void;
  } = $props();
</script>

<section class="git-log" aria-label="Git 日志">
  <div class="git-log-header">
    <div>
      <h2>Git 日志</h2>
    </div>
    <div class="toolbar">
      <button class="secondary" onclick={ongraph} title="提交图" aria-label="提交图"
        ><GitCommit size={18} /></button
      >
      <select id="git-log-scope" aria-label="分支范围" bind:value={all} onchange={onscope}>
        <option value={true}>所有分支</option>
        <option value={false}>当前分支</option>
      </select>
      <button onclick={onrefresh} disabled={loading} aria-label="刷新 Git 日志" title="刷新 Git 日志"
        ><ArrowsClockwise size={16} class={loading ? 'spinning' : ''} /></button
      >
    </div>
  </div>
  <div class="log-content">
    {#if loading && !output}<p class="empty">正在读取 Git 日志…</p>
    {:else if !output}<div class="empty"><GitBranch size={27} /><span>还没有提交</span></div>
    {:else}<pre>{output}</pre>{/if}
    {#if hasMore}<button class="more" onclick={onmore} disabled={loading}
        >{loading ? '正在加载…' : '加载更早的提交'}</button
      >{/if}
  </div>
  <div class="status"><span>最多 {limit} 条</span></div>
</section>

<style>
  .git-log {
    display: flex;
    flex-direction: column;
    min-height: 0;
    min-width: 0;
    height: 100%;
    border-right: 1px solid var(--border);
    background: var(--editor);
  }
  .git-log-header {
    min-height: 58px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 17px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  h2 {
    margin: 0;
    font-size: 15px;
    font-weight: 650;
  }
  .toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    color: var(--muted);
  }
  .toolbar button {
    padding: 6px;
    border-radius: var(--radius-control);
  }
  .toolbar button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .toolbar select {
    width: 105px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    color: var(--text);
    background: var(--surface-raised);
    padding: 6px;
    font: inherit;
  }
  .log-content {
    min-height: 0;
    flex: 1;
    overflow: auto;
    padding: 18px 22px;
  }
  pre {
    margin: 0;
    min-width: max-content;
    color: var(--text);
    font:
      12px/1.8 'SFMono-Regular',
      Consolas,
      monospace;
    tab-size: 2;
  }
  .more {
    display: flex;
    margin: 20px auto;
    padding: 8px 16px;
    color: var(--accent);
  }
  .more:hover {
    background: var(--hover);
  }
  .empty {
    min-height: 200px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    gap: 12px;
    color: var(--faint);
    font-size: 12px;
  }
  .status {
    min-height: 29px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 18px;
    border-top: 1px solid var(--border);
    color: var(--faint);
    font-size: 9px;
  }
  .git-log :global(.spinning) {
    animation: turn 1s linear infinite;
  }
  @keyframes turn {
    to {
      transform: rotate(360deg);
    }
  }
  @media (max-width: 1100px) {
    .git-log-header {
      align-items: flex-start;
      flex-direction: column;
      justify-content: center;
      padding: 10px 18px;
    }
    .toolbar {
      flex-wrap: wrap;
    }
  }
</style>
