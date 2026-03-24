<script lang="ts">
  import { onMount } from 'svelte';
  import { filesApi } from '$lib/api';
  import { toast, openFiles, activeFilePath, activePanel } from '$lib/stores';
  import TreeNode from './TreeNode.svelte';
  import type { FileEntry } from '$lib/api';

  export interface TreeEntry extends FileEntry {
    children?: TreeEntry[];
    open?: boolean;
    loading?: boolean;
  }

  let root = $state<TreeEntry[]>([]);
  let loading = $state(true);
  let contextMenu = $state<{ x: number; y: number; node: TreeEntry } | null>(null);
  let renaming = $state<{ path: string; name: string } | null>(null);

  onMount(async () => {
    await refresh();
  });

  async function refresh() {
    loading = true;
    try {
      const { entries } = await filesApi.tree('.');
      root = entries.map(e => ({ ...e, children: e.type === 'directory' ? [] : undefined, open: false }));
    } catch (e) {
      toast('error', `Erreur chargement workspace`);
    } finally {
      loading = false;
    }
  }

  function showCtx(e: MouseEvent, node: TreeEntry) {
    e.preventDefault();
    contextMenu = { x: e.clientX, y: e.clientY, node };
  }

  function closeCtx() { contextMenu = null; }

  async function deleteNode(node: TreeEntry) {
    closeCtx();
    if (!confirm(`Supprimer "${node.name}" ?`)) return;
    try {
      await filesApi.delete(node.path);
      toast('success', `${node.name} supprimé`);
      await refresh();
    } catch { toast('error', 'Suppression échouée'); }
  }

  async function startRename(node: TreeEntry) {
    closeCtx();
    renaming = { path: node.path, name: node.name };
  }

  async function confirmRename() {
    if (!renaming) return;
    const parts = renaming.path.split('/');
    parts[parts.length - 1] = renaming.name;
    try {
      await filesApi.rename(renaming.path, parts.join('/'));
      toast('success', 'Renommé');
      await refresh();
    } catch { toast('error', 'Renommage échoué'); }
    renaming = null;
  }

  async function newFile(dir: string) {
    closeCtx();
    const name = prompt('Nom du fichier :');
    if (!name) return;
    try {
      await filesApi.write(`${dir === '.' ? '' : dir + '/'}${name}`, '');
      toast('success', `${name} créé`);
      await refresh();
    } catch { toast('error', 'Création échouée'); }
  }

  async function newFolder(dir: string) {
    closeCtx();
    const name = prompt('Nom du dossier :');
    if (!name) return;
    try {
      await filesApi.mkdir(`${dir === '.' ? '' : dir + '/'}${name}`);
      toast('success', `Dossier créé`);
      await refresh();
    } catch { toast('error', 'Création échouée'); }
  }
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="explorer" onclick={closeCtx} role="tree">
  <div class="explorer-head">
    <span class="explorer-title">Workspace</span>
    <div class="head-actions">
      <button onclick={() => newFile('.')} title="Nouveau fichier" class="head-btn">📄+</button>
      <button onclick={() => newFolder('.')} title="Nouveau dossier" class="head-btn">📁+</button>
      <button onclick={refresh} title="Actualiser" class="head-btn">↻</button>
    </div>
  </div>

  <div class="tree-scroll">
    {#if loading}
      <div class="loading-hint">Chargement…</div>
    {:else if root.length === 0}
      <div class="loading-hint">Workspace vide</div>
    {:else}
      {#each root as node (node.path)}
        <TreeNode {node} depth={0} onContextMenu={showCtx} />
      {/each}
    {/if}
  </div>
</div>

<!-- Menu contextuel -->
{#if contextMenu}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <div
    class="ctx-menu"
    style="left:{contextMenu.x}px; top:{contextMenu.y}px"
    role="menu"
    onclick={closeCtx}
  >
    {#if contextMenu.node.type === 'directory'}
      <button onclick={() => newFile(contextMenu!.node.path)} class="ctx-item">📄 Nouveau fichier</button>
      <button onclick={() => newFolder(contextMenu!.node.path)} class="ctx-item">📁 Nouveau dossier</button>
      <hr class="ctx-sep" />
    {/if}
    <button onclick={() => startRename(contextMenu!.node)} class="ctx-item">✏️ Renommer</button>
    <button onclick={() => deleteNode(contextMenu!.node)} class="ctx-item danger">🗑 Supprimer</button>
  </div>
{/if}

<!-- Renommage -->
{#if renaming}
  <div class="rename-overlay" role="dialog">
    <div class="rename-box">
      <p>Renommer <code>{renaming.path.split('/').pop()}</code></p>
      <input
        class="rename-input"
        bind:value={renaming.name}
        onkeydown={(e) => e.key === 'Enter' && confirmRename()}
        autofocus
      />
      <div class="rename-actions">
        <button class="btn btn-ghost" onclick={() => renaming = null}>Annuler</button>
        <button class="btn btn-primary" onclick={confirmRename}>Renommer</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .explorer { display: flex; flex-direction: column; height: 100%; overflow: hidden; user-select: none; }
  .explorer-head { display: flex; align-items: center; justify-content: space-between; padding: 8px 10px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .explorer-title { font-size: 11px; font-weight: 500; text-transform: uppercase; letter-spacing: 0.07em; color: var(--text-muted); }
  .head-actions { display: flex; gap: 2px; }
  .head-btn { background: transparent; border: none; cursor: pointer; font-size: 13px; padding: 2px 5px; border-radius: 3px; color: var(--text-muted); transition: all 0.1s; }
  .head-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .tree-scroll { flex: 1; overflow-y: auto; padding: 4px 0; }
  .loading-hint { text-align: center; color: var(--text-muted); font-size: 12px; padding: 20px; }
  .ctx-menu { position: fixed; z-index: 200; background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 7px; padding: 4px; min-width: 160px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); }
  .ctx-item { display: flex; align-items: center; gap: 7px; width: 100%; padding: 6px 9px; border: none; background: transparent; color: var(--text-secondary); font-size: 12.5px; cursor: pointer; border-radius: 4px; text-align: left; transition: all 0.1s; }
  .ctx-item:hover { background: var(--bg-hover); color: var(--text-primary); }
  .ctx-item.danger:hover { background: var(--red-dim); color: var(--red); }
  .ctx-sep { border: none; border-top: 1px solid var(--border); margin: 3px 0; }
  .rename-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); z-index: 300; display: flex; align-items: center; justify-content: center; }
  .rename-box { background: var(--bg-secondary); border: 1px solid var(--border); border-radius: 10px; padding: 18px 20px; width: 340px; display: flex; flex-direction: column; gap: 10px; }
  .rename-box p { font-size: 13px; color: var(--text-secondary); }
  .rename-input { background: var(--bg-tertiary); border: 1px solid var(--border-focus); color: var(--text-primary); padding: 7px 10px; border-radius: 6px; font-size: 13px; font-family: var(--font-mono); outline: none; }
  .rename-actions { display: flex; justify-content: flex-end; gap: 8px; }
</style>
