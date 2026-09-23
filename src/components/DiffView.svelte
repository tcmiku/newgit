<script lang="ts">
  import { FileCode, Plus, Columns, Rows, FileX } from 'phosphor-svelte';
  import type { Diff } from '../lib/types';
  import { parseDiff, splitDiff, tokens } from '../lib/diff';
  import type { DiffLine, SplitLine } from '../lib/diff';
  let {
    diff,
    path,
    staged = false,
    loading = false,
    busy = false,
    onhunk,
    mode = $bindable('split'),
  }: {
    diff: Diff | null;
    path: string;
    staged?: boolean;
    loading?: boolean;
    busy?: boolean;
    onhunk?: (hunk: number) => void;
    mode?: string;
  } = $props();
  let scrollTop = $state(0);
  let viewportHeight = $state(500);
  let viewport = $state<HTMLDivElement>();
  const lineHeight = 25;
  let lines = $derived(parseDiff(diff?.patch ?? ''));
  let split = $derived(splitDiff(lines));
  let rows: SplitLine[] = $derived(
    mode === 'split'
      ? split
      : lines.map((line) => ({
          header: line.kind === 'hunk' || line.kind === 'meta' ? line : undefined,
          left: line,
        })),
  );
  let start = $derived(Math.max(0, Math.floor(scrollTop / lineHeight) - 12));
  let end = $derived(Math.min(rows.length, start + Math.ceil(viewportHeight / lineHeight) + 24));
  let additions = $derived(lines.filter((l) => l.kind === 'add').length);
  let deletions = $derived(lines.filter((l) => l.kind === 'remove').length);
  $effect(() => {
    path;
    mode;
    diff;
    scrollTop = 0;
    if (viewport) viewport.scrollTop = 0;
  });
</script>

{#snippet code(line: DiffLine | undefined, side: 'old' | 'next' | 'inline')}
  <div class="code-cell {line?.kind ?? 'blank'}">
    {#if side === 'inline'}<span class="line-number">{line?.old ?? ''}</span>{/if}
    <span class="line-number">{side === 'old' ? (line?.old ?? '') : (line?.next ?? '')}</span>
    <span class="line-sign">{line?.kind === 'add' ? '+' : line?.kind === 'remove' ? '−' : ''}</span>
    <code
      >{#if line}{#each tokens(line.content) as token}<span class="syntax-{token.kind}">{token.text}</span
          >{/each}{/if}</code
    >
  </div>
{/snippet}

<section class="diff-panel" aria-label="文件差异">
  <div class="diff-toolbar">
    <div class="diff-path"><FileCode size={17} /><span>{path || '文件差异'}</span></div>
    <div class="diff-tools">
      {#if diff && !loading}<span class="diff-stats"><b>+{additions}</b><i>−{deletions}</i></span>{/if}
      <div class="segmented" aria-label="差异显示方式">
        <button
          class:active={mode === 'split'}
          onclick={() => (mode = 'split')}
          title="并排差异"
          aria-label="并排差异"
          aria-pressed={mode === 'split'}><Columns size={16} /></button
        >
        <button
          class:active={mode === 'inline'}
          onclick={() => (mode = 'inline')}
          title="行内差异"
          aria-label="行内差异"
          aria-pressed={mode === 'inline'}><Rows size={16} /></button
        >
      </div>
    </div>
  </div>
  <div class="diff-labels" class:split={mode === 'split'}>
    <span>{staged ? 'HEAD · 上次提交' : 'INDEX · 暂存版本'}</span>
    {#if mode === 'split'}<span>{staged ? 'INDEX · 暂存版本' : 'WORKING TREE · 工作区'}</span>{/if}
  </div>
  {#if loading}
    <div class="diff-skeleton" aria-label="正在加载差异">
      {#each [78, 54, 65, 42, 80, 60, 72, 48] as width}<div style:width="{width}%"></div>{/each}
    </div>
  {:else if diff?.binary || diff?.truncated}
    <div class="pane-empty">
      <FileX size={36} weight="light" />
      <h3>{diff.binary ? '无法显示文本差异' : '文件较大，已暂停差异渲染'}</h3>
      <p>
        {diff.binary
          ? '二进制文件、符号链接或非 UTF-8 内容可按整个文件暂存。'
          : '超过 1 MB 的差异不会自动展开，你仍然可以暂存整个文件。'}
      </p>
      {#if diff.binary && diff.patch.startsWith('符号链接')}<code>{diff.patch}</code>{/if}
    </div>
  {:else if !lines.length}
    <div class="pane-empty">
      <FileCode size={36} weight="light" />
      <h3>没有文本差异</h3>
      <p>空文件、文件权限或重命名更改仍可暂存。</p>
    </div>
  {:else}
    <div
      class="diff-scroll"
      bind:this={viewport}
      bind:clientHeight={viewportHeight}
      onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
    >
      <div class="diff-lines" style:height="{rows.length * lineHeight}px">
        <div style:transform="translateY({start * lineHeight}px)">
          {#each rows.slice(start, end) as row, i (start + i)}
            {#if row.header}
              <div class="diff-line hunk-line" class:meta-line={row.header.kind === 'meta'}>
                <code>{row.header.content}</code>
                {#if row.header.kind === 'hunk' && diff?.canStageHunks && onhunk}
                  <button
                    class="hunk-action"
                    disabled={busy}
                    onclick={() => onhunk?.(row.header!.hunk!)}
                    title="暂存此代码块"><Plus size={12} />暂存块</button
                  >
                {/if}
              </div>
            {:else}
              <div class="diff-line" class:split-row={mode === 'split'}>
                {@render code(row.left, mode === 'split' ? 'old' : 'inline')}
                {#if mode === 'split'}{@render code(row.right, 'next')}{/if}
              </div>
            {/if}
          {/each}
        </div>
      </div>
    </div>
  {/if}
  <div class="diff-footer">
    <span>UTF-8</span><span>{mode === 'split' ? '并排视图' : '行内视图'}</span><span class="push-right"
      >{lines.length ? `${lines.length} 行差异` : '按需加载'}</span
    >
  </div>
</section>
