<script lang="ts">
  import {
    GitBranch,
    GearSix,
    ArrowsClockwise,
    Plus,
    ArrowUp,
    X,
    Check,
    PencilSimple,
    TextT,
    Trash,
  } from 'phosphor-svelte';
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
  <div class="remote-context">
    <span><GitBranch size={15} />{branch}</span><span class:missing={!upstream}
      >{upstream ? `跟踪 ${upstream}` : '未设置上游'}</span
    >
  </div>
  {#if demo}<p class="modal-note">只读演示；打开本地仓库后可以修改设置。</p>{/if}
  {#if loading}<p class="modal-note">正在读取远程仓库…</p>{/if}
  {#if error}<p role="alert">{error}</p>
    <button class="secondary" onclick={onrefresh}>重新读取</button>{/if}
  <div class="remote-section-title"><span>远程</span><span>{remotes.length}</span></div>
  <div class="remote-list">
    {#each remotes as remote (remote.name)}
      <article>
        <div class="remote-name">
          <span class="remote-icon"><GearSix size={17} /></span><strong>{remote.name}</strong>
        </div>
        <dl>
          <dt>Fetch</dt>
          <dd>{remote.fetchUrl}</dd>
          <dt>Push</dt>
          <dd>{remote.pushUrl || remote.fetchUrl}</dd>
        </dl>
        <div class="remote-buttons">
          <button
            class="secondary remote-icon-action"
            disabled={disabled || loading}
            onclick={() => edit(remote)}
            title={`编辑 ${remote.name} 地址`}
            aria-label={`编辑 ${remote.name} 地址`}><PencilSimple size={16} /></button
          >
          <button
            class="secondary remote-icon-action"
            disabled={disabled || loading}
            title={`重命名 ${remote.name}`}
            aria-label={`重命名 ${remote.name}`}
            onclick={() => {
              renaming = remote.name;
              newName = remote.name;
              removing = null;
            }}><TextT size={16} /></button
          >
          {#if !compact}<button
              class="secondary remote-icon-action"
              disabled={disabled || loading}
              title={`获取 ${remote.name} 的更新`}
              aria-label={`获取 ${remote.name} 的更新`}
              onclick={() =>
                onaction({ kind: 'remote', operation: 'fetch', remote: remote.name }, '正在获取远程更新')}
              ><ArrowsClockwise size={16} /></button
            >{/if}
          {#if !upstream}<button
              class="secondary remote-icon-action"
              disabled={disabled || loading}
              title={`发布当前分支到 ${remote.name}`}
              aria-label={`发布当前分支到 ${remote.name}`}
              onclick={() => onaction({ kind: 'publishBranch', remote: remote.name }, '正在发布分支')}
              ><ArrowUp size={16} /></button
            >{/if}
          <button
            class="text-button remote-icon-action danger"
            disabled={disabled || loading}
            title={`移除 ${remote.name}`}
            aria-label={`移除 ${remote.name}`}
            onclick={() => {
              removing = remote.name;
              renaming = null;
            }}><Trash size={16} /></button
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
    <strong class="form-heading"
      ><span><Plus size={15} /></span>{editing ? `编辑 ${editing}` : '添加远程'}</strong
    >
    <label for="remote-name">名称</label>
    <input
      id="remote-name"
      bind:value={name}
      placeholder="origin"
      required
      disabled={disabled || !!editing}
    />
    <label for="remote-fetch">Fetch URL</label>
    <input
      id="remote-fetch"
      bind:value={fetchUrl}
      placeholder="https://github.com/owner/repo.git 或 git@host:owner/repo.git"
      required
      {disabled}
    />
    <label for="remote-push">Push URL · 选填</label>
    <input id="remote-push" bind:value={pushUrl} placeholder="留空使用 Fetch 地址" {disabled} />
    <div class="remote-buttons">
      <button class="primary" disabled={disabled || loading || !!error || !name.trim() || !fetchUrl.trim()}
        ><Check size={14} />{editing ? '保存地址' : '添加远程'}</button
      >
      {#if editing}<button type="button" class="secondary" onclick={() => edit()}>取消编辑</button>{/if}
    </div>
  </form>
</div>

<style>
  .remote-settings {
    padding: 16px 18px 18px;
    min-height: 0;
    flex: 1;
    overflow: auto;
    font-size: 12px;
  }
  .remote-context {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 7px;
    margin-bottom: 17px;
  }
  .remote-context span {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
    max-width: 100%;
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--branch);
    background: var(--surface-raised);
    overflow-wrap: anywhere;
  }
  .remote-context span + span {
    color: var(--muted);
  }
  .remote-context .missing {
    color: var(--warm);
  }
  .remote-section-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 2px;
    color: var(--muted);
    font-weight: 650;
  }
  .remote-section-title span:last-child {
    min-width: 20px;
    padding: 2px 6px;
    border-radius: 7px;
    background: var(--surface-inset);
    color: var(--faint);
    text-align: center;
    font-size: 10px;
  }
  .remote-list article {
    border: 1px solid var(--border);
    padding: 13px;
    border-radius: var(--radius-card);
    margin: 9px 0;
    background: var(--surface-raised);
    box-shadow: 0 3px 12px var(--surface-shadow);
  }
  .remote-name {
    display: flex;
    align-items: center;
    gap: 9px;
    color: var(--text);
  }
  .remote-icon,
  .form-heading span {
    width: 27px;
    height: 27px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    border-radius: 9px;
    color: var(--branch);
    background: #59dce720;
  }
  dl {
    display: grid;
    grid-template-columns: 40px minmax(0, 1fr);
    gap: 5px;
    color: var(--muted);
    font-size: 11px;
    margin: 11px 0 0 36px;
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
    margin-top: 11px;
  }
  .remote-buttons button {
    min-height: 29px;
    padding: 5px 9px;
    font-size: 11px;
  }
  .remote-buttons .text-button {
    margin-left: auto;
    color: var(--red);
  }
  .remote-buttons .remote-icon-action {
    width: 32px;
    height: 30px;
    min-height: 30px;
    padding: 0;
    border-radius: 9px;
  }
  .remote-buttons .remote-icon-action.danger:hover {
    background: var(--removed);
  }
  .remote-form {
    display: grid;
    gap: 7px;
    margin-top: 15px;
    padding: 14px;
    border: 1px solid var(--border);
    border-radius: var(--radius-card);
    background: var(--surface-raised);
  }
  .form-heading {
    display: flex;
    align-items: center;
    gap: 9px;
    margin-bottom: 3px;
  }
  input {
    width: 100%;
    padding: 9px;
    border: 1px solid var(--border);
    border-radius: var(--radius-control);
    background: var(--surface-inset);
    color: var(--text);
    font: inherit;
  }
  .remote-form label {
    color: var(--muted);
    margin-top: 5px;
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
