import { mkdtempSync, mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { execFileSync } from 'node:child_process';

const base = resolve('.test-repos');
mkdirSync(base, { recursive: true });
const root = mkdtempSync(join(base, 'playground-'));
const git = (...args) => execFileSync('git', args, { cwd: root, stdio: 'pipe', windowsHide: true });
git('init', '-b', 'main');
git('config', 'user.name', 'GitPane Playground');
git('config', 'user.email', 'playground@gitpane.invalid');
git('config', 'commit.gpgsign', 'false');
git('config', 'core.autocrlf', 'false');
mkdirSync(join(root, '.hooks'));
git('config', 'core.hooksPath', join(root, '.hooks'));
mkdirSync(join(root, 'src', 'lib'), { recursive: true });
const initial = `import { invoke } from '@tauri-apps/api/core';
import type { Repository, FileChange } from './types';

export async function openRepository(path: string) {
  const repository = await invoke('open_repository', { path });
  return repository;
}

export async function getChanges(root: string): Promise<FileChange[]> {
  return invoke('get_changes', { root });
}

// Keep the workspace in sync with external changes.
export function subscribe(root: string) {
  return invoke('watch_repository', { root });
}

export function isRepository(path: string) {
  return invoke('is_repository', { path });
}

export function closeRepository(path: string) {
  return invoke('close_repository', { path });
}
`;
writeFileSync(join(root, 'src/lib/repository.ts'), initial);
writeFileSync(join(root, 'README.md'), '# GitPane Playground\n\nA safe place to try GitPane.\n');
writeFileSync(join(root, 'package.json'), '{\n  "name": "playground",\n  "version": "0.0.1"\n}\n');
git('add', '.');
git('commit', '-m', 'chore: initialize workspace');
git('switch', '-c', 'feat/workspace');
writeFileSync(
  join(root, 'README.md'),
  '# GitPane Playground\n\nA safe place to try GitPane.\n\nReview. Stage. Commit.\n',
);
git('add', 'README.md');
git('commit', '-m', 'docs: describe the daily workflow');
writeFileSync(
  join(root, 'src/lib/repository.ts'),
  initial
    .replace(
      'export async function openRepository(path: string) {',
      'const cache = new Map<string, Repository>();\n\nexport async function openRepository(path: string): Promise<Repository> {\n  const cached = cache.get(path);\n  if (cached) return cached;\n',
    )
    .replace('  return repository;', '  cache.set(path, repository);\n  return repository;')
    .replace(
      "  return invoke('get_changes', { root });",
      "  return invoke('get_changes', {\n    root,\n    includeUntracked: true,\n  });",
    )
    .replace(
      "  return invoke('close_repository', { path });",
      "  cache.delete(path);\n  return invoke('close_repository', { path });",
    ),
);
writeFileSync(
  join(root, 'README.md'),
  '# GitPane Playground\n\nYour familiar Git workflow, in its own workspace.\n\nReview. Stage. Commit.\n',
);
git('add', 'README.md');
writeFileSync(join(root, 'package.json'), '{\n  "name": "playground",\n  "version": "0.1.0"\n}\n');
writeFileSync(
  join(root, 'src/lib/shortcuts.ts'),
  "export const shortcuts = {\n  open: 'Mod+O',\n  commit: 'Mod+Enter',\n};\n",
);
console.log(root);
