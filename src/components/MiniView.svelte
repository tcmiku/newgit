<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    ArrowsOutSimple,
    GitBranch,
    GearSix,
    FolderOpen,
    Plus,
    Minus,
    Check,
    X,
    ArrowSquareOut,
    CheckCircle,
    FileCode,
  } from 'phosphor-svelte';
  import { basename, isStaged, isUnstaged } from '../lib/types';
  import type { Snapshot, FileChange } from '../lib/types';
  let {
    repo,
    busy,
    refreshing,
    demo,
    switching,
    menuBar,
    commitMessage = $bindable(''),
    onopen,
    onexpand,
    onbranches,
    onremotes,
    onsettings,
    onpull,
    onpush,
    oncommit,
    onstage,
    onstageall,
    onmenubar,
    onquit,
    onhide,
  }: {
    repo: Snapshot | null;
    busy: string;
    refreshing: boolean;
    demo: boolean;
    switching: boolean;
    menuBar: boolean;
    commitMessage: string;
    onopen: () => void;
    onexpand: () => void;
    onbranches: () => void;
    onremotes: () => void;
    onsettings: () => void;
    onpull: () => void;
    onpush: () => void;
    oncommit: () => void;
    onstage: (file: FileChange, staged: boolean) => void;
    onstageall: () => void;
    onmenubar: (() => void) | null;
    onquit: () => void;
    onhide: () => void;
  } = $props();
  let staged = $derived(repo?.files.filter(isStaged) ?? []);
  let unstaged = $derived(repo?.files.filter(isUnstaged) ?? []);
  let conflicts = $derived(repo?.files.some((f) => f.conflict) ?? false);
  let disabled = $derived(!!busy || refreshing || demo);
</script>

<main class="mini-shell">
  <header>
    <button class="repository" onclick={onopen} disabled={!!busy} title={repo?.root ?? '打开仓库'}>
      <img class="mini-brand" src="/app-icon.png" alt="" /><strong>{repo?.name ?? 'gitpane'}</strong><span
        >{menuBar ? 'BAR' : 'MINI'}</span
      >
    </button>
    <div class="header-actions">
      {#if onmenubar && !menuBar}<button
          class="menu-bar-button"
          onclick={onmenubar}
          disabled={switching || !!busy || refreshing}
          title="只在 macOS 菜单栏显示"
          aria-label="移至菜单栏"><ArrowSquareOut size={17} /></button
        >{/if}
      <button class="icon-button" onclick={onsettings} title="应用设置" aria-label="应用设置"
        ><GearSix size={18} /></button
      >
      <button
        class="icon-button"
        onclick={onexpand}
        disabled={switching || !!busy || refreshing}
        title="切换到完整模式"
        aria-label="切换到完整模式"><ArrowsOutSimple size={18} /></button
      >
    </div>
  </header>
  {#if menuBar}<div class="panel-hint">
      <span>Esc 收起</span>
      <button onclick={onhide} aria-label="收起到菜单栏" title="收起到菜单栏"><X size={16} /></button>
    </div>{/if}
  {#if repo}
    <section class="tools" aria-label="仓库操作">
      <button class="branch" onclick={onbranches} {disabled} title={`切换或创建分支：${repo.branch}`}
        ><GitBranch size={16} /><span>{repo.branch}</span></button
      >
      <button
        class="secondary"
        onclick={onremotes}
        disabled={!!busy}
        title="远程仓库设置"
        aria-label="远程仓库设置"><GearSix size={18} /></button
      >
      <button class="secondary" onclick={onpull} {disabled} title="拉取跟踪分支，仅快进更新" aria-label="拉取"
        ><ArrowDown size={18} />{#if repo.behind}<b>{repo.behind}</b>{/if}</button
      >
      <button
        class="secondary"
        onclick={onpush}
        {disabled}
        title={repo.upstream ? '推送当前分支' : '首次发布分支并设置上游'}
        aria-label={repo.upstream ? '推送' : '发布分支'}
        ><ArrowUp size={18} />{#if repo.ahead}<b>{repo.ahead}</b>{/if}</button
      >
    </section>
    {#if !repo.upstream}<p class="notice">未设置上游。点击 ↑ 发布分支。</p>{/if}
    {#if conflicts || repo.merging}<p class="notice warning">
        {conflicts
          ? '请在编辑器解决冲突，再逐个标记已解决并暂存。'
          : '仓库处于合并或变基中，请确认状态后继续。'}
      </p>{/if}
    <section class="changes" aria-label="提交文件">
      <div class="list-heading"><span><CheckCircle size={14} />暂存 <b>{staged.length}</b></span></div>
      {#each staged as file (file.path)}{@render row(file, true)}{/each}
      <div class="list-heading">
        <span><FileCode size={14} />工作区 <b>{unstaged.length}</b></span><button
          onclick={onstageall}
          disabled={disabled || !unstaged.length || conflicts}
          title="暂存全部更改"
          aria-label="暂存全部更改"><Plus size={16} /></button
        >
      </div>
      {#each unstaged as file (file.path)}{@render row(file, false)}{/each}
    </section>
    <form
      onsubmit={(event) => {
        event.preventDefault();
        oncommit();
      }}
    >
      <textarea
        id="mini-message"
        aria-label="提交说明"
        bind:value={commitMessage}
        placeholder="提交说明…"
        rows="3"
        disabled={!!busy || demo}></textarea>
      <button class="primary" disabled={disabled || conflicts || !staged.length || !commitMessage.trim()}
        ><Check size={16} />提交 <span>{staged.length}</span></button
      >
    </form>
  {:else}
    <section class="welcome-mini">
      <GitBranch size={36} weight="light" />
      <h1>gitpane Mini</h1>
      <button class="primary" onclick={onopen} disabled={!!busy}><FolderOpen size={16} />打开本地仓库</button>
    </section>
  {/if}
  <footer aria-live="polite">
    <span
      class="local-dot"
      role="img"
      aria-label={demo ? '只读演示' : '就绪'}
      title={demo ? '只读演示' : '就绪'}
    ></span>{busy || (refreshing ? '正在刷新…' : demo ? '只读演示' : '')}
    {#if menuBar}<button class="quit-button" onclick={onquit}>退出</button>{/if}
  </footer>
</main>

{#snippet row(file: FileChange, inIndex: boolean)}
  <div class="mini-file">
    <span class="status" class:warning={file.conflict}
      >{file.conflict ? '!' : inIndex ? file.index : file.worktree}</span
    >
    <span class="filename" title={file.originalPath ? `${file.originalPath} → ${file.path}` : file.path}
      >{basename(file.path)}<small>{file.path}</small></span
    >
    <button
      class="icon-button"
      onclick={() => onstage(file, inIndex)}
      {disabled}
      title={inIndex ? '取消暂存' : file.conflict ? '标记已解决并暂存' : '暂存文件'}
      aria-label={`${inIndex ? '取消暂存' : file.conflict ? '标记已解决并暂存' : '暂存'} ${file.path}`}
      >{#if inIndex}<Minus size={16} />{:else}<Plus size={16} />{/if}</button
    >
  </div>
{/snippet}

<style>
  .mini-shell {
    height: 100dvh;
    max-width: 640px;
    margin: auto;
    display: flex;
    flex-direction: column;
    background: var(--panel);
    overflow: hidden;
  }
  header {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 10px 14px;
    border-bottom: 1px solid var(--border);
    background: var(--topbar);
  }
  .repository {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    padding: 4px 7px 4px 3px;
    border-radius: var(--radius-control);
  }
  .header-actions {
    display: flex;
    align-items: center;
    flex: none;
    gap: 4px;
  }
  .header-actions .icon-button,
  .header-actions .menu-bar-button {
    width: 28px;
    height: 28px;
  }
  .repository:hover {
    background: var(--hover);
  }
  .mini-brand {
    width: 28px;
    height: 28px;
    flex-shrink: 0;
    object-fit: contain;
  }
  .repository strong {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .repository > span {
    font-size: 9px;
    letter-spacing: 1px;
    color: var(--accent-ink);
    background: var(--accent);
    padding: 3px 5px;
    border-radius: 7px;
  }
  .menu-bar-button,
  .quit-button {
    font-size: 11px;
    color: var(--muted);
  }
  .menu-bar-button {
    width: 28px;
    height: 28px;
    padding: 0;
    border-radius: 8px;
  }
  .panel-hint {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 14px;
    font-size: 11px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
  }
  .panel-hint span {
    flex: 1;
  }
  .panel-hint button {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 8px;
  }
  .menu-bar-button:hover,
  .quit-button:hover {
    color: var(--text);
  }
  .quit-button {
    margin-left: auto;
  }
  .tools {
    display: flex;
    gap: 6px;
    margin: 12px 13px;
    padding: 6px;
    border: 1px solid var(--border);
    border-radius: 13px;
    background: var(--surface-raised);
    box-shadow: 0 3px 12px var(--surface-shadow);
  }
  .branch {
    flex: 1;
    min-width: 0;
    justify-content: flex-start;
    color: var(--branch);
    padding: 0 8px;
    border-radius: 8px;
  }
  .branch:hover {
    background: var(--hover);
  }
  .branch span {
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tools .secondary {
    width: 35px;
    height: 35px;
    padding: 0;
    font-size: 12px;
    border-color: transparent;
    background: transparent;
    box-shadow: none;
  }
  .tools .secondary:nth-child(2) {
    color: var(--branch);
    background: #59dce719;
  }
  .tools .secondary:nth-child(3) {
    color: var(--positive);
    background: #7de8c319;
  }
  .tools .secondary:nth-child(4) {
    color: var(--red);
    background: #ff77aa19;
  }
  .notice {
    margin: 0 14px 10px;
    padding: 7px 9px;
    font-size: 11px;
    line-height: 1.4;
    color: var(--muted);
    overflow-wrap: anywhere;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--surface-raised);
  }
  .warning {
    color: var(--warm);
  }
  .changes {
    flex: 1;
    min-height: 60px;
    overflow: auto;
    border-top: 1px solid var(--border);
    background: var(--surface-inset);
  }
  .list-heading {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 11px 16px 6px;
    color: var(--muted);
    font-size: 11px;
  }
  .list-heading b {
    margin-left: 5px;
    color: var(--accent);
  }
  .list-heading span {
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .list-heading button {
    width: 27px;
    height: 25px;
    padding: 0;
    border-radius: 7px;
    color: var(--branch);
  }
  .list-heading button:not(:disabled):hover {
    background: var(--hover);
  }
  .mini-file {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 10px;
    margin: 2px 8px;
    border-radius: var(--radius-control);
    background: var(--panel);
    border: 1px solid var(--border);
  }
  .mini-file:hover {
    background: var(--hover);
  }
  .status {
    width: 14px;
    font-size: 11px;
    color: var(--branch);
  }
  .filename {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: 12px;
  }
  .filename small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--faint);
    font-size: 10px;
    margin-top: 2px;
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 9px;
    padding: 12px 14px;
    border-top: 1px solid var(--border);
    background: var(--panel);
  }
  textarea {
    padding: 10px;
    resize: none;
    min-height: 72px;
    background: var(--surface-inset);
  }
  form .primary {
    width: 100%;
    min-height: 35px;
  }
  form .primary span {
    opacity: 0.7;
  }
  footer {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 16px;
    font-size: 10px;
    color: var(--muted);
    border-top: 1px solid var(--border);
  }
  .welcome-mini {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 18px;
    padding: 24px;
  }
  h1 {
    font-size: 20px;
    font-weight: 500;
  }
</style>
