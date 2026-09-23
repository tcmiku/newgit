export interface FileChange {
  path: string;
  originalPath: string | null;
  index: string;
  worktree: string;
  conflict: boolean;
}

export interface Snapshot {
  root: string;
  name: string;
  branch: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: FileChange[];
  merging: boolean;
}

export interface Diff {
  patch: string;
  binary: boolean;
  truncated: boolean;
  canStageHunks: boolean;
}

export interface Commit {
  oid: string;
  short: string;
  subject: string;
  author: string;
  date: string;
  refs: string;
  parents: string[];
}

export interface RemoteInfo {
  name: string;
  fetchUrl: string;
  pushUrl: string | null;
}

export interface Branch {
  name: string;
  current: boolean;
}

export type Mutation =
  | { kind: 'stage' | 'unstage'; paths: string[] }
  | { kind: 'stageHunk'; path: string; expected: string; hunk: number }
  | { kind: 'commit'; message: string }
  | { kind: 'switchBranch'; name: string; create: boolean }
  | { kind: 'remote'; operation: 'fetch' | 'pull' | 'push'; remote?: string }
  | { kind: 'addRemote' | 'setRemote'; name: string; fetchUrl: string; pushUrl: string | null }
  | { kind: 'renameRemote'; oldName: string; newName: string }
  | { kind: 'removeRemote'; name: string }
  | { kind: 'publishBranch'; remote: string };

export type Request =
  | { command: 'open'; path: string }
  | { command: 'snapshot' | 'branches' | 'remotes' | 'close' }
  | { command: 'diff'; path: string; staged: boolean }
  | { command: 'history'; offset: number; all: boolean }
  | { command: 'commitPatch'; oid: string }
  | { command: 'mutate'; action: Mutation };

export const isStaged = (f: FileChange) => f.index !== ' ' && f.index !== '?' && !f.conflict;
export const isUnstaged = (f: FileChange) => f.worktree !== ' ' || f.conflict;
export const basename = (path: string) => path.split(/[\\/]/).pop() || path;
export const dirname = (path: string) => path.replace(/[\\/][^\\/]+$/, '');
