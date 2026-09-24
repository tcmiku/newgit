<script lang="ts">
  import type { Mutation, RemoteInfo } from '../lib/types';
  let {
    remotes,
    loading,
    error,
    disabled,
    branch,
    upstream,
    demo,
    compact = false,
    onaction,
    onrefresh,
  }: {
    remotes: RemoteInfo[];
    loading: boolean;
    error: string;
    disabled: boolean;
    branch: string;
    upstream: string | null;
    demo: boolean;
    compact?: boolean;
    onaction: (action: Mutation, label: string) => Promise<boolean | undefined>;
    onrefresh: () => void;
  } = $props();
  let editing = $state<string | null>(null);
  let name = $state('');
  let fetchUrl = $state('');
  let pushUrl = $state('');
  let renaming = $state<string | null>(null);
  let newName = $state('');
  let removing = $state<string | null>(null);
  function edit(remote?: RemoteInfo) {
    editing = remote?.name ?? null;
    name = remote?.name ?? 'origin';
    fetchUrl = remote?.fetchUrl ?? '';
    pushUrl = remote?.pushUrl ?? '';
    renaming = null;
    removing = null;
  }
  async function save() {
    const ok = await onaction(
      {
        kind: editing ? 'setRemote' : 'addRemote',
        name: name.trim(),
        fetchUrl: fetchUrl.trim(),
        pushUrl: pushUrl.trim() || null,
      },
      editing ? '正在更新远程地址' : '正在添加远程',
    );
    if (ok) {
      editing = null;
      name = '';
      fetchUrl = '';
      pushUrl = '';
    }
  }
</script>

<div class="remote-settings">
  <p class="remote-summary">{branch} · {upstream ? `跟踪 ${upstream}` : '尚未设置跟踪分支'}</p>
  {#if demo}<p class="modal-note">只读演示；打开本地仓库后可以修改设置。</p>{/if}
  {#if loading}<p class="modal-note">正在读取远程仓库…</p>{/if}
  {#if error}<p role="alert">{error}</p>
    <button class="secondary" onclick={onrefresh}>重新读取</button>{/if}
  <div class="remote-list">
    {#each remotes as remote (remote.name)}
      <article>
        <strong>{remote.name}</strong>
        <dl>
          <dt>Fetch</dt>
          <dd>{remote.fetchUrl}</dd>
          <dt>Push</dt>
          <dd>{remote.pushUrl || remote.fetchUrl}</dd>
        </dl>
        <div class="remote-buttons">
          <button class="secondary" disabled={disabled || loading} onclick={() => edit(remote)}
            >编辑地址</button
          >
          <button
            class="secondary"
            disabled={disabled || loading}
            onclick={() => {
              renaming = remote.name;
              newName = remote.name;
              removing = null;
            }}>重命名</button
          >
          {#if !compact}<button
              class="secondary"
              disabled={disabled || loading}
              onclick={() =>
                onaction({ kind: 'remote', operation: 'fetch', remote: remote.name }, '正在获取远程更新')}
              >Fetch</button
            >{/if}
          {#if !upstream}<button
              class="secondary"
              disabled={disabled || loading}
              onclick={() => onaction({ kind: 'publishBranch', remote: remote.name }, '正在发布分支')}
              >发布到 {remote.name}</button
            >{/if}
          <button
            class="text-button"
            disabled={disabled || loading}
            onclick={() => {
              removing = remote.name;
              renaming = null;
            }}>移除</button
          >
        </div>
        {#if renaming === remote.name}
          <form
            class="remote-inline"
            onsubmit={async (e) => {
              e.preventDefault();
              if (
                await onaction(
                  { kind: 'renameRemote', oldName: remote.name, newName: newName.trim() },
                  '正在重命名远程',
                )
              ) {
                renaming = null;
                if (editing === remote.name) edit();
              }
            }}
          >
            <input aria-label="新的远程名称" bind:value={newName} required {disabled} />
            <button class="primary" disabled={disabled || !newName.trim() || newName.trim() === remote.name}
              >重命名</button
            >
            <button type="button" class="secondary" onclick={() => (renaming = null)}>取消</button>
          </form>
        {/if}
        {#if removing === remote.name}
          <div class="remote-confirm">
            <p>移除 {remote.name} 的本地配置和远程跟踪引用。服务器上的仓库与本地分支会保留。</p>
            <button
              class="secondary"
              {disabled}
              onclick={async () => {
                if (await onaction({ kind: 'removeRemote', name: remote.name }, '正在移除远程')) {
                  removing = null;
                  if (editing === remote.name) edit();
                }
              }}>确认移除</button
            >
            <button class="text-button" onclick={() => (removing = null)}>取消</button>
          </div>
        {/if}
      </article>
    {:else}
      {#if !loading && !error}<p class="modal-note">尚未配置远程仓库，请先添加远程地址。</p>{/if}
    {/each}
  </div>
  <form
    class="remote-form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <strong>{editing ? `编辑 ${editing}` : '添加远程仓库'}</strong>
    <label for="remote-name">名称</label>
    <input
      id="remote-name"
      bind:value={name}
      placeholder="origin"
      required
      disabled={disabled || !!editing}
    />
    <label for="remote-fetch">Fetch 地址</label>
    <input
      id="remote-fetch"
      bind:value={fetchUrl}
      placeholder="https://github.com/owner/repo.git 或 git@host:owner/repo.git"
      required
      {disabled}
    />
    <label for="remote-push">Push 地址（选填）</label>
    <input id="remote-push" bind:value={pushUrl} placeholder="留空使用 Fetch 地址" {disabled} />
    <div class="remote-buttons">
      <button class="primary" disabled={disabled || loading || !!error || !name.trim() || !fetchUrl.trim()}
        >{editing ? '保存地址' : '添加远程'}</button
      >
      {#if editing}<button type="button" class="secondary" onclick={() => edit()}>取消编辑</button>{/if}
    </div>
  </form>
</div>

<style>
  .remote-settings {
    padding: 0 22px 22px;
    min-height: 0;
    flex: 1;
    overflow: auto;
    font-size: 12px;
  }
  .remote-summary {
    color: var(--muted);
    overflow-wrap: anywhere;
  }
  .remote-list article {
    border: 1px solid var(--border);
    padding: 14px;
    border-radius: var(--radius-card);
    margin: 12px 0;
    background: var(--accent-soft);
  }
  .remote-list article > strong {
    color: var(--branch);
  }
  dl {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 6px;
    color: var(--muted);
    font-size: 11px;
  }
  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  dt {
    color: var(--faint);
  }
  .remote-buttons,
  .remote-inline {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 12px;
  }
  .remote-form {
    display: grid;
    gap: 9px;
    padding-top: 18px;
    border-top: 1px solid var(--border);
  }
  input {
    width: 100%;
    padding: 9px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--editor);
    color: var(--text);
    font: inherit;
  }
  .remote-form label {
    color: var(--muted);
    margin-top: 6px;
  }
  .remote-confirm {
    color: var(--warm);
    margin-top: 14px;
    line-height: 1.7;
  }
  .remote-inline input {
    flex: 1;
    min-width: 100px;
  }
</style>
