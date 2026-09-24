import type { Snapshot, Diff, Commit, RemoteInfo } from './types';

export const demoSnapshot: Snapshot = {
  root: 'demo://gitpane',
  name: 'gitpane',
  branch: 'feat/workspace',
  upstream: 'origin/feat/workspace',
  ahead: 2,
  behind: 0,
  merging: false,
  files: [
    { path: 'src/lib/repository.ts', originalPath: null, index: ' ', worktree: 'M', conflict: false },
    {
      path: 'src/components/BranchPicker.svelte',
      originalPath: null,
      index: ' ',
      worktree: 'M',
      conflict: false,
    },
    { path: 'src/styles/theme.css', originalPath: null, index: ' ', worktree: 'M', conflict: false },
    { path: 'src/lib/shortcuts.ts', originalPath: null, index: '?', worktree: '?', conflict: false },
    { path: 'README.md', originalPath: null, index: 'M', worktree: ' ', conflict: false },
    { path: 'package.json', originalPath: null, index: 'M', worktree: ' ', conflict: false },
  ],
};

const patches: Record<string, string> = {
  'src/lib/repository.ts': `diff --git a/src/lib/repository.ts b/src/lib/repository.ts
index 4a7ce83..b1f693d 100644
--- a/src/lib/repository.ts
+++ b/src/lib/repository.ts
@@ -1,13 +1,22 @@
 import { invoke } from '@tauri-apps/api/core';
 import type { Repository, FileChange } from './types';
 
-export async function openRepository(path: string) {
-  const repository = await invoke('open_repository', { path });
-  return repository;
+const cache = new Map<string, Repository>();
+
+export async function openRepository(path: string): Promise<Repository> {
+  const cached = cache.get(path);
+  if (cached) return cached;
+
+  const repository = await invoke<Repository>('open_repository', { path });
+  cache.set(path, repository);
+  return repository;
 }
 
 export async function getChanges(root: string): Promise<FileChange[]> {
-  return invoke('get_changes', { root });
+  return invoke('get_changes', {
+    root,
+    includeUntracked: true,
+  });
 }
 
 // Keep the workspace in sync with external changes.
@@ -28,4 +37,9 @@
 export function closeRepository(path: string) {
-  return invoke('close_repository', { path });
+  cache.delete(path);
+  return invoke('close_repository', { path });
 }
+
+export function clearCache() {
+  cache.clear();
+}
`,
  'README.md':
    '--- a/README.md\n+++ b/README.md\n@@ -1,3 +1,5 @@\n # gitpane\n \n-A Git client.\n+A little less overhead. A lot more focus.\n+\n+Your familiar Git workflow, in its own workspace.\n',
  'package.json':
    '--- a/package.json\n+++ b/package.json\n@@ -1,4 +1,4 @@\n {\n   "name": "gitpane",\n-  "version": "0.0.1"\n+  "version": "0.1.0"\n }\n',
};

export function demoDiff(path: string): Diff {
  return {
    patch:
      patches[path] ??
      `--- a/${path}\n+++ b/${path}\n@@ -1,3 +1,4 @@\n // gitpane workspace\n-export const enabled = false;\n+export const enabled = true;\n+export const debounce = 250;\n \n`,
    binary: false,
    truncated: false,
    canStageHunks: false,
  };
}

export const demoHistory: Commit[] = [
  {
    oid: 'a7d23f10000000000000000000000000000000000',
    short: 'a7d23f1',
    subject: 'feat: remember the last opened workspace',
    author: 'Alex Chen',
    date: '2026-09-23T10:24:00+08:00',
    refs: 'HEAD -> feat/workspace',
    parents: ['f02b5810000000000000000000000000000000000'],
  },
  {
    oid: 'f02b5810000000000000000000000000000000000',
    short: 'f02b581',
    subject: 'merge: integrate history graph',
    author: 'Alex Chen',
    date: '2026-09-22T16:40:00+08:00',
    refs: '',
    parents: ['8be043d0000000000000000000000000000000000', 'd319a440000000000000000000000000000000000'],
  },
  {
    oid: '8be043d0000000000000000000000000000000000',
    short: '8be043d',
    subject: 'feat: add keyboard navigation',
    author: 'Alex Chen',
    date: '2026-09-22T09:10:00+08:00',
    refs: 'origin/feat/workspace',
    parents: ['391ad6e0000000000000000000000000000000000'],
  },
  {
    oid: 'd319a440000000000000000000000000000000000',
    short: 'd319a44',
    subject: 'feat: draw branch and merge lanes',
    author: 'Sam Lin',
    date: '2026-09-22T08:00:00+08:00',
    refs: 'feat/graph, origin/feat/graph, tag: v0.1.0',
    parents: ['391ad6e0000000000000000000000000000000000'],
  },
  {
    oid: '391ad6e0000000000000000000000000000000000',
    short: '391ad6e',
    subject: 'chore: initialize gitpane',
    author: 'Alex Chen',
    date: '2026-09-21T11:30:00+08:00',
    refs: 'main',
    parents: [],
  },
];

export const demoRemotes: RemoteInfo[] = [
  { name: 'origin', fetchUrl: 'git@github.com:example/gitpane.git', pushUrl: null },
];

export const demoGitLog = `* a7d23f1 (HEAD -> feat/workspace) feat: remember the last opened workspace  [Alex Chen · 2026-09-23]
*   f02b581 merge: integrate history graph  [Alex Chen · 2026-09-22]
|\\
| * d319a44 (tag: v0.1.0, origin/feat/graph, feat/graph) feat: draw branch and merge lanes  [Sam Lin · 2026-09-22]
* | 8be043d (origin/feat/workspace) feat: add keyboard navigation  [Alex Chen · 2026-09-22]
|/
* 391ad6e (main) chore: initialize gitpane  [Alex Chen · 2026-09-21]`;
