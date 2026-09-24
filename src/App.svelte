<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { LogicalSize } from '@tauri-apps/api/dpi';
  import MiniView from './components/MiniView.svelte';
  import { open as chooseDirectory } from '@tauri-apps/plugin-dialog';
  import {
    GitBranch,
    GitCommit,
    FolderOpen,
    Plus,
    Minus,
    ArrowDown,
    ArrowUp,
    ArrowsClockwise,
    CaretDown,
    CaretRight,
    Check,
    ClockCounterClockwise,
    MagnifyingGlass,
    Command,
    X,
    WarningCircle,
    Sun,
    Moon,
    Keyboard,
    ArrowRight,
    CheckCircle,
    Info,
    FileCode,
    GearSix,
    ArrowSquareOut,
    ArrowsInSimple,
    TrayArrowDown,
    Power,
  } from 'phosphor-svelte';
  import GraphView from './components/GraphView.svelte';
  import GitLogView from './components/GitLogView.svelte';
  import RemoteSettings from './components/RemoteSettings.svelte';
  import DiffView from './components/DiffView.svelte';
  import { request, native, readSetting, saveSetting } from './lib/api';
  import { basename, isStaged, isUnstaged } from './lib/types';
  import type { Snapshot, FileChange, Diff, Commit, GitLog, Branch, Mutation, RemoteInfo } from './lib/types';

  const mac = navigator.userAgent.includes('Mac');
  const mod = mac ? '⌘' : 'Ctrl';
  // Always start with a reachable window, including after an interrupted tray session.
  const initialMenuBar = false;
  let repo = $state<Snapshot | null>(null);
  let menuBar = $state(initialMenuBar);
  let mini = $state(readSetting<boolean>('mini-mode', false) || initialMenuBar);
  let switchingMode = $state(false);
  let fullSize = new LogicalSize(1320, 850);
  let fullMaximized = false;

  async function resizeMode(compact: boolean, remember = true) {
    if (!native) return;
    const win = getCurrentWebviewWindow();
    if (compact && remember) {
      fullMaximized = await win.isMaximized();
      if (!fullMaximized) fullSize = (await win.innerSize()).toLogical(await win.scaleFactor());
    }
    await win.unmaximize();
    await win.setMinSize(new LogicalSize(compact ? 380 : 860, compact ? 480 : 580));
    await win.setSize(compact ? new LogicalSize(460, 620) : fullSize);
    if (!compact && fullMaximized) await win.maximize();
  }

  async function toggleMini() {
    if (menuBar || switchingMode || busy || refreshing) return;
    switchingMode = true;
    try {
      await resizeMode(!mini);
      mini = !mini;
      saveSetting('mini-mode', mini);
      modal = null;
      view = 'changes';
      autoHistory = false;
      diffSequence++;
      historySequence++;
      gitLogSequence++;
      diff = null;
      selected = null;
      currentCommit = null;
      diffLoading = historyLoading = gitLogLoading = false;
      if (!mini && repo && !repo.files.length) {
        historyMode = 'graph';
        await changeView('history', true);
      } else if (!mini) chooseNextFile();
    } catch (e) {
      notify(`无法切换窗口模式：${String(e)}`, true);
      try {
        await resizeMode(mini, false);
      } catch {
        /* Keep the current interface usable. */
      }
    } finally {
      switchingMode = false;
    }
  }

  async function enterMenuBar() {
    if (!native || !mac || menuBar || switchingMode || busy || refreshing) return;
    const wasMini = mini;
    switchingMode = true;
    try {
      if (!mini) await resizeMode(true);
      await getCurrentWebviewWindow().setSize(new LogicalSize(420, 560));
      mini = true;
      menuBar = true;
      saveSetting('menu-bar-mode', true);
      modal = null;
      await tick();
      await invoke('set_menu_bar_mode', { enabled: true });
    } catch (e) {
      await invoke('set_menu_bar_mode', { enabled: false }).catch(() => {});
      menuBar = false;
      saveSetting('menu-bar-mode', false);
      mini = wasMini;
      if (!wasMini) await resizeMode(false).catch(() => {});
      await getCurrentWebviewWindow()
        .show()
        .catch(() => {});
      notify(`无法开启菜单栏模式：${String(e)}`, true);
    } finally {
      switchingMode = false;
    }
  }

  async function leaveMenuBar() {
    if (!menuBar || switchingMode || busy || refreshing) return;
    switchingMode = true;
    const win = getCurrentWebviewWindow();
    try {
      await invoke('set_menu_bar_mode', { enabled: false });
      menuBar = false;
      mini = false;
      saveSetting('menu-bar-mode', false);
      saveSetting('mini-mode', false);
      await resizeMode(false);
      await tick();
      if (repo && !repo.files.length) {
        historyMode = 'graph';
        await changeView('history', true);
      } else chooseNextFile();
    } catch (e) {
      notify(`无法返回完整模式：${String(e)}`, true);
    } finally {
      await win.show().catch(() => {});
      await win.setFocus().catch(() => {});
      switchingMode = false;
    }
  }

  function expandMini() {
    if (menuBar) void leaveMenuBar();
    else void toggleMini();
  }
  let selected = $state<{ path: string; staged: boolean } | null>(null);
  let diff = $state<Diff | null>(null);
  let diffLoading = $state(false);
  let busy = $state('');
  let refreshing = $state(false);
  let view = $state<'changes' | 'history'>('changes');
  let autoHistory = false;
  let mode = $state(readSetting<string>('diff-mode', 'split'));
  let theme = $state(readSetting<string>('theme', 'dark'));
  type CloseBehavior = 'tray' | 'quit';
  let closeBehavior = $state<CloseBehavior>(
    readSetting<string>('close-behavior', 'tray') === 'quit' ? 'quit' : 'tray',
  );
  let closeSettingBusy = $state(false);
  let filter = $state('');
  let commitMessage = $state('');
  let recents = $state<string[]>(readSetting<string[]>('recents', []));
  let history = $state<Commit[]>([]);
  let historyLoading = $state(false);
  let historyMode = $state<'graph' | 'log'>('graph');
  let gitLog = $state<GitLog>({ output: '', hasMore: false });
  let gitLogLoading = $state(false);
  let gitLogLimit = $state(100);
  let hasMore = $state(false);
  let currentCommit = $state<Commit | null>(null);
  let allHistory = $state(true);
  let remotes = $state<RemoteInfo[]>([]);
  let remotesLoading = $state(false);
  let remotesError = $state('');
  let branches = $state<Branch[]>([]);
  let branchesLoading = $state(false);
  let modal = $state<'repository' | 'branches' | 'commands' | 'help' | 'remotes' | 'settings' | null>(null);
  let query = $state('');
  let pathInput = $state('');
  let newBranch = $state('');
  type Notice = { id: number; text: string; kind: 'success' | 'error' | 'info' };
  type RepositoryChange = { root: string; paths: string[]; gitState: boolean };
  let notices = $state<Notice[]>([]);
  let stagedExpanded = $state(true);
  let unstagedExpanded = $state(true);
  let demo = $state(false);
  let dialogElement = $state<HTMLDivElement>();
  let diffSequence = 0;
  let historySequence = 0;
  let gitLogSequence = 0;
  let nextNoticeId = 0;
  const noticeTimers = new Map<number, ReturnType<typeof setTimeout>>();
  let refreshTimer: ReturnType<typeof setTimeout>;
  let staged = $derived(repo?.files.filter(isStaged) ?? []);
  let unstaged = $derived(repo?.files.filter(isUnstaged) ?? []);
  let conflicts = $derived(repo?.files.filter((f) => f.conflict).length ?? 0);
  let filteredStaged = $derived(staged.filter((f) => f.path.toLowerCase().includes(filter.toLowerCase())));
  let filteredUnstaged = $derived(
    unstaged.filter((f) => f.path.toLowerCase().includes(filter.toLowerCase())),
  );
  let activeFile = $derived(repo?.files.find((f) => f.path === selected?.path));

  $effect(() => {
    document.documentElement.dataset.theme = theme;
    saveSetting('theme', theme);
  });
  $effect(() => {
    saveSetting('diff-mode', mode);
  });
  $effect(() => {
    if (repo && !demo) saveSetting(`draft:${repo.root}`, commitMessage);
  });

  function dismissNotice(id: number) {
    clearTimeout(noticeTimers.get(id));
    noticeTimers.delete(id);
    notices = notices.filter((notice) => notice.id !== id);
  }

  function clearNotices() {
    for (const timer of noticeTimers.values()) clearTimeout(timer);
    noticeTimers.clear();
    notices = [];
  }

  function notify(text: string, error = false, info = false) {
    const id = ++nextNoticeId;
    notices = [{ id, text, kind: error ? 'error' : info ? 'info' : 'success' }, ...notices];
    if (!error)
      noticeTimers.set(
        id,
        setTimeout(() => dismissNotice(id), 4500),
      );
  }

  async function chooseCloseBehavior(next: CloseBehavior) {
    if (closeSettingBusy || closeBehavior === next) return;
    closeSettingBusy = true;
    try {
      if (native) await invoke('set_close_behavior', { exitOnClose: next === 'quit' });
      closeBehavior = next;
      saveSetting('close-behavior', next);
    } catch (e) {
      notify(`无法保存关闭行为：${String(e)}`, true);
    } finally {
      closeSettingBusy = false;
    }
  }

  async function showModal(kind: typeof modal) {
    modal = kind;
    query = '';
    newBranch = '';
    pathInput = '';
    await tick();
    (
      dialogElement?.querySelector<HTMLElement>('input:not(:disabled), .close-option:not(:disabled)') ??
      dialogElement
    )?.focus();
    if (kind === 'remotes') await loadRemotes();
    if (kind === 'branches' && repo) {
      branchesLoading = true;
      try {
        branches = demo
          ? [
              { name: 'feat/workspace', current: true },
              { name: 'main', current: false },
            ]
          : await request<Branch[]>({ command: 'branches' }, repo.root);
      } catch (e) {
        notify(String(e), true);
      } finally {
        branchesLoading = false;
      }
    }
  }

  async function loadRemotes() {
    if (!repo) return;
    const root = repo.root;
    remotesLoading = true;
    remotesError = '';
    try {
      const result = demo
        ? (await import('./lib/demo')).demoRemotes
        : await request<RemoteInfo[]>({ command: 'remotes' }, root);
      if (repo?.root === root) remotes = result;
    } catch (e) {
      if (repo?.root === root) remotesError = String(e);
    } finally {
      if (repo?.root === root) remotesLoading = false;
    }
  }

  async function openRepository(path: string) {
    if (busy) return;
    busy = '正在打开仓库';
    modal = null;
    clearNotices();
    try {
      const next = await request<Snapshot>({ command: 'open', path }, null);
      diffSequence++;
      historySequence++;
      gitLogSequence++;
      demo = false;
      repo = next;
      remotes = [];
      await loadRemotes();
      selected = null;
      diff = null;
      history = [];
      gitLog = { output: '', hasMore: false };
      gitLogLimit = 100;
      historyMode = 'graph';
      currentCommit = null;
      view = 'changes';
      autoHistory = false;
      filter = '';
      commitMessage = readSetting(`draft:${next.root}`, '');
      recents = [next.root, ...recents.filter((p) => p !== next.root)].slice(0, 8);
      saveSetting('recents', recents);
      saveSetting('last-repository', next.root);
      if (!mini && !next.files.length) await changeView('history', true);
      else chooseNextFile();
    } catch (e) {
      notify(String(e), true);
    } finally {
      busy = '';
    }
  }

  async function browse() {
    if (!native) {
      notify('浏览器仅支持界面演示。请运行 gitpane 桌面应用来打开本地仓库。', false, true);
      return;
    }
    try {
      const path = await chooseDirectory({ directory: true, multiple: false, title: '选择 Git 仓库' });
      if (path) await openRepository(path);
    } catch (e) {
      notify(String(e), true);
    }
  }

  async function loadDemo() {
    const { demoSnapshot } = await import('./lib/demo');
    diffSequence++;
    historySequence++;
    gitLogSequence++;
    demo = true;
    repo = structuredClone(demoSnapshot);
    await loadRemotes();
    selected = null;
    diff = null;
    view = 'changes';
    autoHistory = false;
    history = [];
    gitLog = { output: '', hasMore: false };
    gitLogLimit = 100;
    historyMode = 'graph';
    currentCommit = null;
    modal = null;
    commitMessage = '';
    clearNotices();
    if (!mini && !repo.files.length) await changeView('history', true);
    else chooseNextFile();
  }

  function chooseNextFile() {
    if (mini || !repo || view !== 'changes') return;
    if (selected) {
      const existing = repo.files.find((f) => f.path === selected!.path);
      if (existing && (selected.staged ? isStaged(existing) : isUnstaged(existing))) {
        void selectFile(existing, selected.staged);
        return;
      }
      if (existing) {
        void selectFile(existing, isStaged(existing));
        return;
      }
    }
    const file = repo.files.find(isUnstaged) ?? repo.files.find(isStaged);
    if (file) void selectFile(file, !isUnstaged(file));
    else {
      selected = null;
      diff = null;
      diffLoading = false;
      diffSequence++;
    }
  }

  async function selectFile(file: FileChange, staged: boolean) {
    if (!repo) return;
    selected = { path: file.path, staged };
    currentCommit = null;
    diff = null;
    diffLoading = true;
    const sequence = ++diffSequence,
      root = repo.root;
    try {
      const result = demo
        ? (await import('./lib/demo')).demoDiff(file.path)
        : await request<Diff>({ command: 'diff', path: file.path, staged }, root);
      if (sequence === diffSequence) diff = result;
    } catch (e) {
      if (sequence === diffSequence) notify(String(e), true);
    } finally {
      if (sequence === diffSequence) diffLoading = false;
    }
  }

  async function refreshSnapshot(reloadSelected: boolean) {
    if (!repo || demo || refreshing || busy) return;
    refreshing = true;
    const root = repo.root;
    try {
      const next = await request<Snapshot>({ command: 'snapshot' }, root);
      if (repo?.root !== root) return;
      const previousFile = repo.files.find((file) => file.path === selected?.path);
      const nextFile = next.files.find((file) => file.path === selected?.path);
      const selectedStatusChanged =
        previousFile?.index !== nextFile?.index ||
        previousFile?.worktree !== nextFile?.worktree ||
        previousFile?.conflict !== nextFile?.conflict;
      const becameClean = !mini && view === 'changes' && repo.files.length > 0 && !next.files.length;
      const returnToChanges = !mini && autoHistory && view === 'history' && next.files.length > 0;
      repo = next;
      await loadRemotes();
      if (repo?.root !== root) return;
      if (becameClean) {
        historyMode = 'graph';
        await changeView('history', true);
      } else if (returnToChanges) {
        await changeView('changes');
      } else {
        if (reloadSelected || selectedStatusChanged || !selected) chooseNextFile();
      }
      if (view === 'history' && !becameClean) {
        void loadHistory();
        if (historyMode === 'log') void loadGitLog();
      }
    } catch (e) {
      notify(String(e), true);
    } finally {
      refreshing = false;
    }
  }

  async function refresh() {
    await refreshSnapshot(true);
  }

  async function mutate(action: Mutation, label: string) {
    if (!repo || busy || refreshing) return false;
    if (demo) {
      notify('当前是只读演示，请打开本地仓库以执行 Git 操作。', false, true);
      return;
    }
    busy = label;
    clearNotices();
    try {
      const result = await request<string>({ command: 'mutate', action }, repo.root);
      if (action.kind === 'commit') commitMessage = '';
      notify(result);
      if (modal !== 'remotes') modal = null;
      return true;
    } catch (e) {
      notify(String(e), true);
      return false;
    } finally {
      busy = '';
      await refresh();
    }
  }

  function stageFile(file: FileChange, wasStaged: boolean) {
    void mutate(
      { kind: wasStaged ? 'unstage' : 'stage', paths: [file.path] },
      wasStaged ? '正在取消暂存' : '正在暂存更改',
    );
  }

  function stageHunk(hunk: number) {
    if (selected && diff)
      void mutate({ kind: 'stageHunk', path: selected.path, expected: diff.patch, hunk }, '正在暂存代码块');
  }

  async function commit() {
    if (!commitMessage.trim() || !staged.length || conflicts) return;
    await mutate({ kind: 'commit', message: commitMessage }, '正在提交');
  }

  async function changeView(next: typeof view, automatic = false) {
    if (mini) return;
    autoHistory = automatic && next === 'history';
    if (view === next) return;
    diffSequence++;
    diffLoading = false;
    diff = null;
    selected = null;
    currentCommit = null;
    view = next;
    filter = '';
    if (next === 'history') {
      await loadHistory();
      if (historyMode === 'log') await loadGitLog();
    } else chooseNextFile();
  }

  async function loadHistory(more = false) {
    if (!repo) return;
    const sequence = ++historySequence;
    historyLoading = true;
    try {
      const result = demo
        ? (await import('./lib/demo')).demoHistory
        : await request<Commit[]>(
            { command: 'history', offset: more ? history.length : 0, all: allHistory },
            repo.root,
          );
      if (sequence !== historySequence || view !== 'history') return;
      history = more
        ? [...history, ...result.filter((item) => !history.some((loaded) => loaded.oid === item.oid))]
        : result;
      hasMore = !demo && result.length === 60;
      if (!history.some((item) => item.oid === currentCommit?.oid)) {
        currentCommit = null;
        diff = null;
        diffSequence++;
        diffLoading = false;
        if (history.length) void selectCommit(history[0]);
      }
    } catch (e) {
      notify(String(e), true);
    } finally {
      if (sequence === historySequence) historyLoading = false;
    }
  }

  async function loadGitLog(limit = gitLogLimit) {
    if (!repo) return;
    const sequence = ++gitLogSequence;
    const root = repo.root;
    gitLogLoading = true;
    try {
      const result = demo
        ? { output: (await import('./lib/demo')).demoGitLog, hasMore: false }
        : await request<GitLog>({ command: 'gitLog', limit, all: allHistory }, root);
      if (sequence !== gitLogSequence || repo?.root !== root || view !== 'history' || historyMode !== 'log')
        return;
      gitLog = result;
      gitLogLimit = limit;
    } catch (e) {
      if (sequence === gitLogSequence) notify(String(e), true);
    } finally {
      if (sequence === gitLogSequence) gitLogLoading = false;
    }
  }

  async function openGitLog() {
    if (!repo) return;
    if (view !== 'history') await changeView('history');
    historyMode = 'log';
    await loadGitLog();
  }

  function changeHistoryScope() {
    void loadHistory();
    if (historyMode === 'log') void loadGitLog();
  }

  async function selectCommit(item: Commit) {
    if (!repo) return;
    currentCommit = item;
    diff = null;
    diffLoading = true;
    const sequence = ++diffSequence;
    try {
      const result = demo
        ? (await import('./lib/demo')).demoDiff('src/lib/repository.ts')
        : await request<Diff>({ command: 'commitPatch', oid: item.oid }, repo.root);
      if (sequence === diffSequence) diff = result;
    } catch (e) {
      if (sequence === diffSequence) notify(String(e), true);
    } finally {
      if (sequence === diffSequence) diffLoading = false;
    }
  }

  async function closeRepository() {
    if (busy) return;
    if (native && !demo && repo) {
      try {
        await request({ command: 'close' }, repo.root);
      } catch (e) {
        notify(String(e), true);
        return;
      }
    }
    repo = null;
    autoHistory = false;
    demo = false;
    diff = null;
    selected = null;
    currentCommit = null;
    diffSequence++;
    historySequence++;
    gitLogSequence++;
    modal = null;
    saveSetting('last-repository', null);
  }

  const actions = [
    { label: '切换 Mini / 完整模式', key: '', run: toggleMini },
    { label: '打开仓库', key: `${mod} O`, run: browse },
    { label: '刷新仓库状态', key: `${mod} R`, run: refresh },
    { label: '查看变更', key: `${mod} ⇧ G`, run: () => changeView('changes') },
    { label: '查看提交历史', key: '', run: () => changeView('history') },
    { label: '打开 Git 日志', key: '', run: openGitLog },
    { label: '远程仓库设置', key: '', run: () => showModal('remotes') },
    { label: '应用设置', key: '', run: () => showModal('settings') },
    { label: '切换分支', key: '', run: () => showModal('branches') },
    { label: '切换明暗主题', key: '', run: () => (theme = theme === 'dark' ? 'light' : 'dark') },
  ];

  function keydown(event: KeyboardEvent) {
    if (modal && event.key === 'Escape') {
      modal = null;
      return;
    }
    if (menuBar && event.key === 'Escape') {
      void invoke('hide_menu_bar_panel');
      return;
    }
    if (modal && event.key === 'Tab') {
      const focusable = dialogElement?.querySelectorAll<HTMLElement>(
        'button:not(:disabled), input:not(:disabled), select:not(:disabled), [tabindex="0"]',
      );
      if (focusable?.length) {
        const first = focusable[0],
          last = focusable[focusable.length - 1];
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first.focus();
        }
      }
    }
    if (!(event.metaKey || event.ctrlKey)) return;
    if (event.key.toLowerCase() === 'o') {
      event.preventDefault();
      void browse();
    }
    if (event.key.toLowerCase() === 'p') {
      event.preventDefault();
      if (!mini) void showModal('commands');
    }
    if (event.key.toLowerCase() === 'r') {
      event.preventDefault();
      void refresh();
    }
    if (event.key.toLowerCase() === 'g' && event.shiftKey) {
      event.preventDefault();
      void changeView('changes');
    }
    if (event.key === 'Enter' && !modal && view === 'changes') {
      event.preventDefault();
      void commit();
    }
  }

  onMount(() => {
    let disposed = false;
    const cleanups: (() => void)[] = [];
    let pendingDiffReload = false;
    const scheduleRefresh = (reloadSelected: boolean) => {
      pendingDiffReload ||= reloadSelected;
      clearTimeout(refreshTimer);
      refreshTimer = setTimeout(() => {
        const reload = pendingDiffReload;
        pendingDiffReload = false;
        void refreshSnapshot(reload);
      }, 200);
    };
    const focus = () => scheduleRefresh(true);
    window.addEventListener('focus', focus);
    if (native) {
      void invoke('set_close_behavior', { exitOnClose: closeBehavior === 'quit' }).catch((e) =>
        notify(`无法读取关闭行为：${String(e)}`, true),
      );
      void (async () => {
        saveSetting('menu-bar-mode', false);
        const restore = await listen('menu-bar-restored', () => {
          menuBar = false;
          mini = false;
          modal = null;
          saveSetting('mini-mode', false);
          saveSetting('menu-bar-mode', false);
          if (repo && !repo.files.length) {
            historyMode = 'graph';
            void changeView('history', true);
          } else chooseNextFile();
        });
        if (disposed) restore();
        else cleanups.push(restore);
        if (mini) {
          try {
            await resizeMode(true, false);
          } catch (e) {
            mini = false;
            saveSetting('mini-mode', false);
            notify(String(e), true);
          }
        }
        const stop = await listen<RepositoryChange>('repository-changed', (event) => {
          if (event.payload.root === repo?.root && !document.hidden) {
            const selectedPath = selected?.path;
            const reloadSelected =
              event.payload.gitState ||
              !selectedPath ||
              event.payload.paths.some(
                (path) => path === selectedPath || !path || selectedPath.startsWith(`${path}/`),
              );
            scheduleRefresh(reloadSelected);
          }
        });
        if (disposed) stop();
        else cleanups.push(stop);
        const unlisten = await getCurrentWebviewWindow().onDragDropEvent((event) => {
          if (event.payload.type === 'drop' && event.payload.paths[0])
            void openRepository(event.payload.paths[0]);
        });
        if (disposed) unlisten();
        else cleanups.push(unlisten);
        const path =
          (await invoke<string | null>('launch_path')) ?? readSetting<string | null>('last-repository', null);
        if (path && !disposed) await openRepository(path);
      })().catch((e) => notify(String(e), true));
    }
    return () => {
      disposed = true;
      cleanups.forEach((fn) => fn());
      window.removeEventListener('focus', focus);
      clearTimeout(refreshTimer);
      for (const timer of noticeTimers.values()) clearTimeout(timer);
      noticeTimers.clear();
    };
  });
</script>

<svelte:window onkeydown={keydown} />

{#snippet fileRow(file: FileChange, inIndex: boolean)}
  <div class="file-row" class:selected={selected?.path === file.path && selected.staged === inIndex}>
    <button
      class="file-select"
      onclick={() => selectFile(file, inIndex)}
      title={file.originalPath ? `${file.originalPath} → ${file.path}` : file.path}
    >
      <span
        class="file-type"
        class:typescript={/\.(ts|tsx)$/.test(file.path)}
        class:svelte={file.path.endsWith('.svelte')}
        >{file.path.endsWith('.svelte')
          ? 'S'
          : /\.(ts|tsx)$/.test(file.path)
            ? 'TS'
            : file.path.endsWith('.json')
              ? '{}'
              : file.path.endsWith('.css')
                ? '#'
                : '≡'}</span
      >
      <span class="file-name"
        >{basename(file.path)}<small
          >{file.path.includes('/') ? file.path.slice(0, file.path.lastIndexOf('/')) : ''}</small
        ></span
      >
      <span
        class="file-status"
        class:added={file.index === '?' || file.index === 'A'}
        class:conflicted={file.conflict}
        >{file.conflict ? '!' : inIndex ? file.index : file.worktree === '?' ? 'U' : file.worktree}</span
      >
    </button>
    <button
      class="file-action icon-button"
      disabled={!!busy || refreshing || demo}
      onclick={() => stageFile(file, inIndex)}
      title={inIndex ? `取消暂存 ${file.path}` : `暂存 ${file.path}`}
      aria-label={inIndex ? `取消暂存 ${file.path}` : `暂存 ${file.path}`}
      >{#if inIndex}<Minus size={15} />{:else}<Plus size={15} />{/if}</button
    >
  </div>
{/snippet}

{#if mini}
  <MiniView
    {repo}
    {busy}
    {refreshing}
    {demo}
    switching={switchingMode}
    {menuBar}
    bind:commitMessage
    onopen={() => showModal('repository')}
    onexpand={expandMini}
    onmenubar={mac && native ? enterMenuBar : null}
    onquit={() => invoke('quit_app')}
    onhide={() => invoke('hide_menu_bar_panel')}
    onbranches={() => showModal('branches')}
    onremotes={() => showModal('remotes')}
    onsettings={() => showModal('settings')}
    onpull={() => mutate({ kind: 'remote', operation: 'pull' }, '正在拉取')}
    onpush={() =>
      repo?.upstream ? mutate({ kind: 'remote', operation: 'push' }, '正在推送') : showModal('remotes')}
    oncommit={commit}
    onstage={stageFile}
    onstageall={() => mutate({ kind: 'stage', paths: unstaged.map((f) => f.path) }, '正在暂存全部更改')}
  />
{:else}
  <div class="app-shell">
    <aside class="activity-bar" aria-label="主导航">
      <button
        class="brand-mark"
        title="gitpane 首页"
        onclick={() => {
          if (!busy) void showModal('repository');
        }}><img src="/app-icon.png" alt="gitpane" /></button
      >
      <div class="activity-main">
        <button
          class:active={view === 'changes'}
          onclick={() => changeView('changes')}
          title="变更"
          aria-label="变更"
          ><GitBranch
            size={23}
            weight={view === 'changes' ? 'fill' : 'regular'}
          />{#if repo?.files.length}<span class="activity-dot"></span>{/if}</button
        >
        <button
          class:active={view === 'history'}
          disabled={!repo}
          onclick={() => changeView('history')}
          title="提交历史"
          aria-label="提交历史"
          ><ClockCounterClockwise size={23} weight={view === 'history' ? 'fill' : 'regular'} /></button
        >
        <button onclick={() => showModal('repository')} title="仓库" aria-label="仓库"
          ><FolderOpen size={23} weight="light" /></button
        >
      </div>
      <div class="activity-bottom">
        <button onclick={() => showModal('settings')} title="应用设置" aria-label="应用设置"
          ><GearSix size={21} weight="light" /></button
        >
        <button onclick={() => showModal('commands')} title="命令面板" aria-label="命令面板"
          ><Command size={21} /></button
        >
        <button
          onclick={() => (theme = theme === 'dark' ? 'light' : 'dark')}
          title="切换明暗主题"
          aria-label="切换明暗主题"
          >{#if theme === 'dark'}<Sun size={21} weight="light" />{:else}<Moon
              size={21}
              weight="light"
            />{/if}</button
        >
        <button onclick={() => showModal('help')} title="快捷键与关于" aria-label="快捷键与关于"
          ><Keyboard size={21} weight="light" /></button
        >
      </div>
    </aside>

    <div class="workspace">
      <header class="topbar">
        <button
          class="repo-switcher"
          onclick={() => showModal('repository')}
          disabled={!!busy}
          title="切换仓库"
        >
          <span class="repo-avatar">{repo ? repo.name.slice(0, 1).toUpperCase() : 'G'}</span>
          <span>{repo?.name ?? 'gitpane'}</span>
          <CaretDown size={13} />
        </button>
        {#if repo}<button
            class="topbar-branch"
            onclick={() => showModal('branches')}
            disabled={!!busy || refreshing}
            title={`切换分支：${repo.branch}`}
            aria-label={`切换分支，当前为 ${repo.branch}`}
            ><GitBranch size={15} weight="bold" /><span>{repo.branch}</span><CaretDown size={11} /></button
          >{/if}
        <div class="remote-actions">
          {#if mac && native}<button
              class="topbar-menu"
              onclick={enterMenuBar}
              disabled={switchingMode || !!busy || refreshing}
              title="移至菜单栏"
              aria-label="菜单栏模式"><ArrowSquareOut size={18} /></button
            >{/if}
          <button
            class="topbar-mini"
            onclick={toggleMini}
            disabled={switchingMode || !!busy || refreshing}
            title="Mini 模式"
            aria-label="Mini 模式"><ArrowsInSimple size={18} /></button
          >
          <button
            class="topbar-remotes"
            disabled={!repo || !!busy}
            onclick={() => showModal('remotes')}
            title="远程设置"
            aria-label="远程设置"><GearSix size={18} /></button
          >
          <button
            class="topbar-fetch"
            disabled={!repo || !!busy || refreshing || demo}
            onclick={() => mutate({ kind: 'remote', operation: 'fetch' }, '正在 Fetch')}
            title="获取远程更新"
            aria-label="Fetch"><ArrowsClockwise size={18} /></button
          >
          <button
            disabled={!repo || !!busy || refreshing || demo}
            onclick={() => mutate({ kind: 'remote', operation: 'pull' }, '正在 Pull')}
            title="拉取（仅快进）"
            aria-label="Pull"
            class="sync-action pull-action"
            ><ArrowDown size={18} />{#if repo?.behind}<span>{repo.behind}</span>{/if}</button
          >
          <button
            disabled={!repo || !!busy || refreshing || demo}
            onclick={() =>
              repo?.upstream
                ? mutate({ kind: 'remote', operation: 'push' }, '正在 Push')
                : showModal('remotes')}
            title={repo && !repo.upstream ? '发布分支' : '推送当前分支'}
            aria-label={repo && !repo.upstream ? '发布分支' : 'Push'}
            class="sync-action push-action"
            ><ArrowUp size={18} />{#if repo?.ahead}<span>{repo.ahead}</span>{/if}</button
          >
        </div>
      </header>

      {#if repo}
        <div class="workspace-content" class:history-workspace={view === 'history'}>
          {#if view === 'history'}
            {#key repo.root}
              {#if historyMode === 'graph'}
                <GraphView
                  commits={history}
                  selected={currentCommit?.oid ?? null}
                  loading={historyLoading}
                  {hasMore}
                  remoteNames={remotes.map((remote) => remote.name)}
                  bind:all={allHistory}
                  onselect={selectCommit}
                  onmore={() => loadHistory(true)}
                  onscope={changeHistoryScope}
                  onrefresh={() => loadHistory()}
                  onlog={openGitLog}
                />
              {:else}
                <GitLogView
                  output={gitLog.output}
                  loading={gitLogLoading}
                  hasMore={gitLog.hasMore}
                  limit={gitLogLimit}
                  bind:all={allHistory}
                  ongraph={() => (historyMode = 'graph')}
                  onscope={changeHistoryScope}
                  onrefresh={() => loadGitLog()}
                  onmore={() => loadGitLog(Math.min(gitLogLimit + 100, 5000))}
                />
              {/if}
            {/key}
          {:else}
            <aside class="source-panel">
              <div class="source-heading">
                <h1>变更</h1>
                <div>
                  <span class="subtle-label">{view === 'changes' ? repo.files.length : history.length}</span
                  ><button
                    class="icon-button"
                    onclick={refresh}
                    disabled={!!busy || refreshing || demo}
                    title="刷新仓库"
                    aria-label="刷新仓库"
                    ><ArrowsClockwise size={16} class={refreshing ? 'spinning' : ''} /></button
                  >
                </div>
              </div>
              <div class="source-tabs">
                <button
                  class:active={view === 'changes'}
                  onclick={() => changeView('changes')}
                  title="变更"
                  aria-label="变更"><FileCode size={18} /></button
                ><button onclick={() => changeView('history')} title="提交图" aria-label="提交图"
                  ><GitCommit size={18} /></button
                ><button onclick={openGitLog} title="Git 日志" aria-label="Git 日志"
                  ><Command size={18} /></button
                >
              </div>
              {#if view === 'changes'}
                <div class="commit-box">
                  <textarea
                    id="commit-message"
                    aria-label="提交说明"
                    bind:value={commitMessage}
                    placeholder="提交说明…"
                    rows="3"
                    disabled={!!busy || demo}></textarea>
                  <button
                    class="primary commit-button"
                    onclick={commit}
                    disabled={!commitMessage.trim() ||
                      !staged.length ||
                      !!busy ||
                      refreshing ||
                      !!conflicts ||
                      demo}
                    title="提交暂存更改"
                    ><Check size={16} weight="bold" />提交 <span>{staged.length}</span></button
                  >
                </div>
                <div class="file-filter">
                  <MagnifyingGlass size={14} /><input
                    aria-label="筛选更改文件"
                    bind:value={filter}
                    placeholder="筛选…"
                  />{#if filter}<button
                      class="icon-button"
                      onclick={() => (filter = '')}
                      aria-label="清除筛选"><X size={12} /></button
                    >{/if}
                </div>
                <div class="file-groups">
                  <div class="group-heading">
                    <button onclick={() => (stagedExpanded = !stagedExpanded)} aria-expanded={stagedExpanded}
                      >{#if stagedExpanded}<CaretDown size={12} />{:else}<CaretRight
                          size={12}
                        />{/if}<CheckCircle size={14} />暂存
                      <span>{staged.length}</span></button
                    ><button
                      class="icon-button"
                      disabled={!staged.length || !!busy || refreshing || demo}
                      onclick={() =>
                        mutate({ kind: 'unstage', paths: staged.map((f) => f.path) }, '正在取消全部暂存')}
                      title="取消全部暂存"
                      aria-label="取消全部暂存"><Minus size={14} /></button
                    >
                  </div>
                  {#if stagedExpanded}{#each filteredStaged as file (file.path)}{@render fileRow(
                        file,
                        true,
                      )}{/each}{/if}
                  <div class="group-heading unstaged-heading">
                    <button
                      onclick={() => (unstagedExpanded = !unstagedExpanded)}
                      aria-expanded={unstagedExpanded}
                      >{#if unstagedExpanded}<CaretDown size={12} />{:else}<CaretRight
                          size={12}
                        />{/if}<FileCode size={14} />工作区
                      <span>{unstaged.length}</span></button
                    ><button
                      class="icon-button"
                      disabled={!unstaged.length || !!busy || refreshing || demo || !!conflicts}
                      onclick={() =>
                        mutate({ kind: 'stage', paths: unstaged.map((f) => f.path) }, '正在暂存全部更改')}
                      title="暂存全部更改"
                      aria-label="暂存全部更改"><Plus size={14} /></button
                    >
                  </div>
                  {#if unstagedExpanded}{#each filteredUnstaged as file (file.path)}{@render fileRow(
                        file,
                        false,
                      )}{/each}{/if}
                  {#if filter && !filteredStaged.length && !filteredUnstaged.length}<p class="group-empty">
                      没有匹配的文件
                    </p>{/if}
                </div>
              {/if}
            </aside>
          {/if}

          <main class="main-panel">
            {#if repo.merging || conflicts}<div class="conflict-notice">
                <WarningCircle size={16} /><span
                  >{conflicts
                    ? `${conflicts} 个文件存在冲突。请在编辑器中解决，确认后逐个暂存。`
                    : '仓库正在合并或变基，请确认操作状态后继续。'}</span
                >
              </div>{/if}
            {#if view === 'changes' && selected}
              <div class="review-heading">
                <div class="review-file">
                  <FileCode size={19} />
                  <h2>{basename(selected.path)}</h2>
                  <span class="tab-status">{selected.staged ? '暂存' : '工作区'}</span>
                </div>
                <button
                  class="secondary review-stage-action"
                  disabled={!!busy || refreshing || demo}
                  onclick={() => activeFile && stageFile(activeFile, selected!.staged)}
                  title={selected.staged
                    ? '取消暂存'
                    : activeFile?.conflict
                      ? '标记已解决并暂存'
                      : '暂存文件'}
                  aria-label={selected.staged
                    ? '取消暂存'
                    : activeFile?.conflict
                      ? '标记已解决并暂存'
                      : '暂存文件'}
                  >{#if selected.staged}<Minus size={17} />{:else}<Plus size={17} />{/if}</button
                >
              </div>
              <DiffView
                {diff}
                path={selected.path}
                staged={selected.staged}
                loading={diffLoading}
                busy={!!busy || refreshing}
                onhunk={stageHunk}
                bind:mode
              />
            {:else if view === 'history' && currentCommit}
              <div class="review-heading commit-detail">
                <div>
                  <span class="eyebrow"
                    >{currentCommit.short} · {currentCommit.author} · {new Date(
                      currentCommit.date,
                    ).toLocaleString('zh-CN')}</span
                  >
                  <h2>{currentCommit.subject}</h2>
                </div>
              </div>
              <DiffView
                {diff}
                path={`commit ${currentCommit.short}`}
                historical
                staged
                loading={diffLoading}
                bind:mode
              />
            {:else}
              <div class="clean-workspace">
                <div class="clean-icon"><CheckCircle size={46} weight="light" /></div>
                <h2>{view === 'history' ? '暂无提交' : '工作区已清空'}</h2>
                <button
                  class="secondary"
                  onclick={refresh}
                  disabled={refreshing || !!busy || demo}
                  title="刷新仓库"
                  aria-label="刷新仓库"><ArrowsClockwise size={17} /></button
                >
              </div>
            {/if}
          </main>
        </div>
      {:else}
        <main class="welcome">
          <div class="welcome-copy">
            <div class="welcome-symbol"><img src="/app-icon.png" alt="gitpane 蓝发狐面角色" /></div>
            <h1>gitpane<span>.</span></h1>
            <p>打开仓库，开始工作。</p>
            <div class="welcome-actions">
              <button class="primary" onclick={browse} disabled={!!busy}
                ><FolderOpen size={18} />打开仓库</button
              ><button class="secondary" onclick={loadDemo} title="打开只读演示"
                >试用演示<ArrowRight size={15} /></button
              >
            </div>
            {#if recents.length}<div class="recent-welcome">
                <span class="eyebrow">最近仓库</span>{#each recents.slice(0, 4) as path}<button
                    onclick={() => openRepository(path)}
                    disabled={!!busy}
                    ><FolderOpen size={15} /><span>{basename(path)}<small>{path}</small></span><ArrowRight
                      size={14}
                    /></button
                  >{/each}
              </div>{/if}
          </div>
        </main>
      {/if}

      <footer class="statusbar">
        <div>
          {#if repo}<button
              onclick={() => showModal('branches')}
              disabled={!!busy}
              title={`分支：${repo.branch}`}
              aria-label={`切换分支，当前为 ${repo.branch}`}><GitBranch size={14} /></button
            ><span class="sync-count"
              ><ArrowDown size={11} />{repo.behind}<ArrowUp size={11} />{repo.ahead}</span
            >{:else}<span><GitBranch size={13} />gitpane</span>{/if}
        </div>
        <div class="status-message" aria-live="polite">
          {#if busy || refreshing}<ArrowsClockwise size={12} class="spinning" />{busy ||
              '正在刷新'}{:else}<span
              class="local-dot"
              role="img"
              aria-label={demo ? '只读演示' : '就绪'}
              title={demo ? '只读演示' : '就绪'}
            ></span>{#if demo}只读演示{/if}{/if}
        </div>
      </footer>
    </div>
  </div>
{/if}

{#if notices.length}
  <div class="notice-rail" aria-label="通知">
    {#each notices as notice (notice.id)}
      <div
        class="notice-strip"
        class:error={notice.kind === 'error'}
        class:info={notice.kind === 'info'}
        role={notice.kind === 'error' ? 'alert' : 'status'}
      >
        <span
          class="notice-tag"
          title={notice.kind === 'error' ? '错误' : notice.kind === 'info' ? '提示' : '成功'}
          aria-label={notice.kind === 'error' ? '错误' : notice.kind === 'info' ? '提示' : '成功'}
        >
          {#if notice.kind === 'error'}<WarningCircle
              size={16}
              weight="bold"
            />{:else if notice.kind === 'info'}<Info size={16} weight="bold" />{:else}<Check
              size={16}
              weight="bold"
            />{/if}
        </span>
        <p title={notice.text}>{notice.text}</p>
        <button
          class="icon-button notice-close"
          onclick={() => dismissNotice(notice.id)}
          aria-label="关闭通知"
          title="关闭通知"><X size={15} /></button
        >
      </div>
    {/each}
  </div>
{/if}

{#if modal}
  <div
    class="modal-backdrop"
    class:mini-backdrop={mini}
    role="presentation"
    onclick={(event) => {
      if (event.target === event.currentTarget) modal = null;
    }}
  >
    <div
      class="modal"
      class:remote-modal={modal === 'remotes'}
      role="dialog"
      aria-modal="true"
      aria-label={modal === 'remotes'
        ? '远程仓库设置'
        : modal === 'settings'
          ? '应用设置'
          : modal === 'branches'
            ? '切换分支'
            : modal === 'commands'
              ? '命令面板'
              : modal === 'help'
                ? '快捷键与关于'
                : '打开仓库'}
      tabindex="-1"
      bind:this={dialogElement}
    >
      <div class="modal-title">
        <span
          >{#if modal === 'remotes'}<GearSix size={19} />远程{:else if modal === 'settings'}<GearSix
              size={19}
            />设置{:else if modal === 'branches'}<GitBranch
              size={19}
            />分支{:else if modal === 'commands'}<Command size={19} />命令{:else if modal === 'help'}<Keyboard
              size={19}
            />关于{:else}<FolderOpen size={19} />仓库{/if}</span
        ><button class="icon-button" onclick={() => (modal = null)} aria-label="关闭对话框"
          ><X size={18} /></button
        >
      </div>
      {#if modal === 'remotes'}
        <RemoteSettings
          compact={mini}
          {remotes}
          loading={remotesLoading}
          error={remotesError}
          disabled={!!busy || refreshing || demo}
          branch={repo?.branch ?? ''}
          upstream={repo?.upstream ?? null}
          {demo}
          onaction={mutate}
          onrefresh={loadRemotes}
        />
      {:else if modal === 'settings'}
        <div class="modal-body settings-body">
          <div class="settings-caption">点击关闭按钮时</div>
          <div class="close-options" role="group" aria-label="关闭按钮行为">
            <button
              class="close-option"
              class:chosen={closeBehavior === 'tray'}
              aria-pressed={closeBehavior === 'tray'}
              disabled={closeSettingBusy}
              onclick={() => chooseCloseBehavior('tray')}
              ><span class="close-option-icon"><TrayArrowDown size={20} /></span><span
                class="close-option-copy"><strong>收起到托盘</strong><small>后台继续运行</small></span
              >{#if closeBehavior === 'tray'}<Check size={17} weight="bold" />{/if}</button
            >
            <button
              class="close-option"
              class:chosen={closeBehavior === 'quit'}
              aria-pressed={closeBehavior === 'quit'}
              disabled={closeSettingBusy}
              onclick={() => chooseCloseBehavior('quit')}
              ><span class="close-option-icon"><Power size={20} /></span><span class="close-option-copy"
                ><strong>直接退出</strong><small>关闭程序进程</small></span
              >{#if closeBehavior === 'quit'}<Check size={17} weight="bold" />{/if}</button
            >
          </div>
        </div>
      {:else if modal === 'repository'}
        <div class="modal-body">
          <label for="repo-path">仓库路径</label>
          <form
            onsubmit={(e) => {
              e.preventDefault();
              if (pathInput.trim()) void openRepository(pathInput.trim());
            }}
          >
            <div class="path-input">
              <input
                id="repo-path"
                bind:value={pathInput}
                placeholder={mac ? '/Users/you/projects/my-app' : 'D:\Projects\my-app'}
              /><button class="primary" type="submit" disabled={!pathInput.trim() || !!busy || !native}
                >打开</button
              >
            </div>
          </form>
          <button class="browse-button" onclick={browse} disabled={!!busy}
            ><FolderOpen size={16} />浏览<span>{mod} O</span></button
          >
          {#if recents.length}<div class="modal-section-title">最近</div>
            {#each recents as path}<button
                class="recent-item"
                onclick={() => openRepository(path)}
                disabled={!!busy}
                ><FolderOpen size={16} /><span>{basename(path)}<small>{path}</small></span><ArrowRight
                  size={14}
                /></button
              >{/each}{/if}{#if repo}<button
              class="text-button close-repo"
              onclick={closeRepository}
              disabled={!!busy}>关闭仓库</button
            >{/if}
        </div>
      {:else if modal === 'branches'}
        <div class="modal-search">
          <MagnifyingGlass size={17} /><input
            aria-label="搜索分支"
            bind:value={query}
            placeholder="搜索本地分支…"
          />
        </div>
        <div class="command-list">
          {#each branches.filter((b) => b.name.toLowerCase().includes(query.toLowerCase())) as branch}<button
              class="command-item"
              class:current={branch.current}
              disabled={branch.current || !!busy || demo}
              onclick={() =>
                mutate({ kind: 'switchBranch', name: branch.name, create: false }, '正在切换分支')}
              ><GitBranch size={16} /><span>{branch.name}</span>{#if branch.current}<Check
                  size={16}
                />{/if}</button
            >{/each}{#if branchesLoading}<p class="modal-note">正在读取分支…</p>{/if}
        </div>
        <form
          class="create-branch"
          onsubmit={(e) => {
            e.preventDefault();
            if (newBranch.trim())
              void mutate({ kind: 'switchBranch', name: newBranch.trim(), create: true }, '正在创建分支');
          }}
        >
          <label for="new-branch">新建分支</label>
          <div class="path-input">
            <input id="new-branch" bind:value={newBranch} placeholder="feat/my-next-idea" /><button
              class="primary"
              disabled={!newBranch.trim() || !!busy || demo}><Plus size={14} />创建</button
            >
          </div>
        </form>
      {:else if modal === 'commands'}
        <div class="modal-search">
          <MagnifyingGlass size={17} /><input
            aria-label="搜索命令"
            bind:value={query}
            placeholder="输入命令名称…"
          />
        </div>
        <div class="command-list">
          {#each actions.filter((a) => a.label.includes(query)) as action}<button
              class="command-item"
              disabled={!!busy}
              onclick={() => {
                modal = null;
                void action.run();
              }}><CaretRight size={14} /><span>{action.label}</span><kbd>{action.key}</kbd></button
            >{/each}
        </div>
      {:else}
        <div class="modal-body help-body">
          <div class="about-logo">
            <img src="/app-icon.png" alt="" /><span>gitpane<small>0.1.0 · 本地 Git 工作空间</small></span>
          </div>
          <p>查看差异、暂存、提交。把注意力留给代码。</p>
          {#each [[`${mod} O`, '打开仓库'], [`${mod} P`, '命令面板'], [`${mod} R`, '刷新仓库'], [`${mod} ⇧ G`, '查看变更'], [`${mod} ↵`, '提交暂存更改'], ['Esc', '关闭对话框']] as shortcut}<div
              class="shortcut-row"
            >
              <span>{shortcut[1]}</span><kbd>{shortcut[0]}</kbd>
            </div>{/each}
          <p class="modal-note">
            使用系统 Git。暂存与提交在本地完成；Fetch / Pull / Push 连接仓库配置的远程。Pull 仅执行快进更新。
          </p>
        </div>
      {/if}
      <div class="modal-footer">
        <span>{busy || 'gitpane'}</span><span><kbd>Esc</kbd> 关闭</span>
      </div>
    </div>
  </div>
{/if}
