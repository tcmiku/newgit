<script lang="ts">
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { ArrowClockwise, TerminalWindow, X } from 'phosphor-svelte';
  import type { Terminal as XtermTerminal, IDisposable } from '@xterm/xterm';
  import type { FitAddon } from '@xterm/addon-fit';
  import '@xterm/xterm/css/xterm.css';

  let {
    root,
    theme,
    onclose,
    onrestart,
  }: {
    root: string;
    theme: string;
    onclose: () => void;
    onrestart: () => void;
  } = $props();

  let surface: HTMLDivElement;
  let status = $state('正在启动…');
  let exited = $state(false);
  let term: XtermTerminal | undefined;
  let fit: FitAddon | undefined;
  const sessionId = globalThis.crypto?.randomUUID?.() ?? `${Date.now()}-${Math.random()}`;

  function applyTheme() {
    if (!term) return;
    const styles = getComputedStyle(document.documentElement);
    term.options.theme = {
      background: styles.getPropertyValue('--surface-inset').trim(),
      foreground: styles.getPropertyValue('--text').trim(),
      cursor: styles.getPropertyValue('--accent').trim(),
      selectionBackground: styles.getPropertyValue('--selection').trim(),
    };
  }

  $effect(() => {
    theme;
    applyTheme();
  });

  onMount(() => {
    let disposed = false;
    let started = false;
    let observer: ResizeObserver | undefined;
    let frame = 0;
    const cleanups: (() => void)[] = [];
    let writeQueue = Promise.resolve();

    function send(data: number[]) {
      if (exited || disposed) return;
      writeQueue = writeQueue
        .then(() => invoke<void>('terminal_write', { sessionId, data }))
        .catch((error) => {
          if (!disposed) status = `输入失败：${String(error)}`;
        });
    }

    async function start() {
      try {
        const unlistenOutput = await listen<{ sessionId: string; data: number[] }>(
          'terminal-output',
          (event) => {
            if (event.payload.sessionId === sessionId && !disposed)
              term?.write(Uint8Array.from(event.payload.data));
          },
          { target: { kind: 'Any' } },
        );
        if (disposed) return unlistenOutput();
        cleanups.push(unlistenOutput);

        const unlistenExit = await listen<{ sessionId: string; code: number | null }>(
          'terminal-exit',
          (event) => {
            if (event.payload.sessionId !== sessionId || disposed) return;
            exited = true;
            status = event.payload.code === null ? '已退出' : `已退出 · ${event.payload.code}`;
            term?.writeln('\r\n[进程已结束]');
          },
          { target: { kind: 'Any' } },
        );
        if (disposed) return unlistenExit();
        cleanups.push(unlistenExit);

        const [{ Terminal }, { FitAddon }] = await Promise.all([
          import('@xterm/xterm'),
          import('@xterm/addon-fit'),
        ]);
        if (disposed) return;
        term = new Terminal({
          cursorBlink: true,
          fontFamily: "'Cascadia Code', 'SFMono-Regular', Consolas, monospace",
          fontSize: 12,
          lineHeight: 1.25,
          scrollback: 2000,
        });
        fit = new FitAddon();
        term.loadAddon(fit);
        term.open(surface);
        applyTheme();
        fit.fit();
        const dataListener: IDisposable = term.onData((data) =>
          send(Array.from(new TextEncoder().encode(data))),
        );
        const binaryListener: IDisposable = term.onBinary((data) =>
          send(Array.from(data, (character) => character.charCodeAt(0) & 0xff)),
        );
        cleanups.push(
          () => dataListener.dispose(),
          () => binaryListener.dispose(),
        );

        observer = new ResizeObserver(() => {
          cancelAnimationFrame(frame);
          frame = requestAnimationFrame(() => {
            if (!term || !fit || disposed) return;
            fit.fit();
            if (started && !exited)
              void invoke('terminal_resize', {
                sessionId,
                cols: term.cols,
                rows: term.rows,
              }).catch(() => {});
          });
        });
        observer.observe(surface);

        await invoke('terminal_start', {
          sessionId,
          repository: root,
          cols: term.cols,
          rows: term.rows,
        });
        started = true;
        if (disposed) {
          void invoke('terminal_stop', { sessionId });
          return;
        }
        if (!exited) status = '运行中';
        term.focus();
      } catch (error) {
        if (!disposed) status = `启动失败：${String(error)}`;
      }
    }

    void start();
    return () => {
      disposed = true;
      cancelAnimationFrame(frame);
      observer?.disconnect();
      cleanups.forEach((cleanup) => cleanup());
      term?.dispose();
      term = undefined;
      if (started) void invoke('terminal_stop', { sessionId });
    };
  });
</script>

<section class="terminal-pane" aria-label="集成终端">
  <div class="terminal-heading">
    <div class="terminal-label">
      <TerminalWindow size={17} />
      <strong>终端</strong>
      <span class="terminal-path" title={root}>{root}</span>
    </div>
    <div class="terminal-controls">
      <span class:ended={exited} class="terminal-status" title={status}>{status}</span>
      <button onclick={onrestart} title="重新启动终端" aria-label="重新启动终端"
        ><ArrowClockwise size={16} /></button
      >
      <button onclick={onclose} title="关闭终端" aria-label="关闭终端"><X size={16} /></button>
    </div>
  </div>
  <div class="terminal-surface" bind:this={surface}></div>
</section>

<style>
  .terminal-pane {
    height: min(38vh, 360px);
    min-height: 170px;
    flex: none;
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border-strong);
    background: var(--surface-inset);
  }
  .terminal-heading {
    height: 37px;
    flex: none;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    padding: 0 12px 0 17px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-raised);
  }
  .terminal-label,
  .terminal-controls {
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .terminal-label :global(svg) {
    color: var(--branch);
  }
  .terminal-label strong {
    font-size: 11px;
    font-weight: 650;
  }
  .terminal-path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--faint);
    font-size: 10px;
  }
  .terminal-status {
    max-width: 180px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--branch);
    font-size: 10px;
  }
  .terminal-status.ended {
    color: var(--muted);
  }
  .terminal-controls button {
    width: 27px;
    height: 27px;
    padding: 0;
    border-radius: 8px;
    color: var(--faint);
  }
  .terminal-controls button:hover {
    background: var(--hover);
    color: var(--text);
  }
  .terminal-surface {
    flex: 1;
    min-height: 0;
    min-width: 0;
    padding: 9px 12px;
    overflow: hidden;
  }
  .terminal-surface :global(.xterm) {
    height: 100%;
  }
</style>
