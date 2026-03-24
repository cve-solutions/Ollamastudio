<script lang="ts">
  import type { FileEntry } from '$lib/api';
  import { openFiles, activeFilePath, activePanel, toast } from '$lib/stores';
  import { filesApi } from '$lib/api';
  import { get } from 'svelte/store';
  import TreeNode from './TreeNode.svelte';

  interface TreeEntry extends FileEntry {
    children?: TreeEntry[];
    open?: boolean;
    loading?: boolean;
  }

  interface Props {
    node: TreeEntry;
    depth?: number;
    onContextMenu: (e: MouseEvent, node: TreeEntry) => void;
  }

  let { node, depth = 0, onContextMenu }: Props = $props();

  const FILE_ICONS: Record<string, string> = {
    rs: '🦀', py: '🐍', js: '🟨', ts: '🔷', tsx: '🔷', jsx: '🟨',
    svelte: '🧡', go: '💠', java: '☕', md: '📝', json: '📋',
    yaml: '⚙️', yml: '⚙️', toml: '⚙️', sh: '📜', bash: '📜',
    dockerfile: '🐳', html: '🌐', css: '🎨', sql: '🗄️',
    txt: '📄', env: '🔒', gitignore: '🚫', lock: '🔒',
  };

  function fileIcon(name: string): string {
    const lower = name.toLowerCase();
    if (lower === 'dockerfile') return '🐳';
    if (lower === '.env' || lower.startsWith('.env.')) return '🔒';
    const ext = name.split('.').pop()?.toLowerCase() ?? '';
    return FILE_ICONS[ext] ?? '📄';
  }

  function detectLanguage(filename: string): string {
    const ext = filename.split('.').pop()?.toLowerCase() ?? '';
    const map: Record<string, string> = {
      rs: 'rust', py: 'python', js: 'javascript', ts: 'typescript',
      tsx: 'typescript', jsx: 'javascript', svelte: 'html',
      sh: 'shell', bash: 'shell', yaml: 'yaml', yml: 'yaml',
      toml: 'toml', json: 'json', md: 'markdown',
      dockerfile: 'dockerfile', html: 'html', css: 'css',
      sql: 'sql', go: 'go', c: 'c', cpp: 'cpp', java: 'java',
    };
    return map[ext] ?? 'plaintext';
  }

  async function handleClick() {
    if (node.type === 'directory') {
      await toggleDir();
    } else {
      await openFile();
    }
  }

  async function toggleDir() {
    if (!node.open) {
      node.loading = true;
      try {
        const { entries } = await filesApi.tree(node.path);
        node.children = entries.map(e => ({
          ...e,
          children: e.type === 'directory' ? [] : undefined,
          open: false,
          loading: false,
        }));
      } catch {
        toast('error', 'Erreur chargement dossier');
      }
      node.loading = false;
    }
    node.open = !node.open;
  }

  async function openFile() {
    const files = get(openFiles);
    if (files.some(f => f.path === node.path)) {
      activeFilePath.set(node.path);
      activePanel.set('editor');
      return;
    }
    try {
      const { content } = await filesApi.read(node.path);
      openFiles.update(list => [...list, {
        path: node.path,
        content,
        modified: false,
        language: detectLanguage(node.name),
      }]);
      activeFilePath.set(node.path);
      activePanel.set('editor');
    } catch (e) {
      toast('error', `Lecture impossible: ${node.name}`);
    }
  }

  const indent = depth * 14;
  const isActive = $derived($activeFilePath === node.path);
</script>

<div class="tree-node">
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="node-row {isActive ? 'active' : ''}"
    style="padding-left: {8 + indent}px"
    onclick={handleClick}
    oncontextmenu={(e) => onContextMenu(e, node)}
    role="treeitem"
    aria-expanded={node.type === 'directory' ? node.open : undefined}
    tabindex="0"
    onkeydown={(e) => e.key === 'Enter' && handleClick()}
  >
    {#if node.type === 'directory'}
      <span class="chevron">{node.loading ? '⏳' : node.open ? '▾' : '▸'}</span>
      <span class="icon">📁</span>
    {:else}
      <span class="chevron"></span>
      <span class="icon">{fileIcon(node.name)}</span>
    {/if}
    <span class="name">{node.name}</span>
    {#if node.type === 'file' && node.size != null}
      <span class="size">{node.size < 1024 ? node.size + 'B' : Math.round(node.size / 1024) + 'Ko'}</span>
    {/if}
  </div>

  <!-- Enfants récursifs -->
  {#if node.type === 'directory' && node.open && node.children}
    {#each node.children as child (child.path)}
      <TreeNode node={child} depth={depth + 1} {onContextMenu} />
    {/each}
    {#if node.children.length === 0}
      <div class="empty-dir" style="padding-left: {8 + indent + 28}px">vide</div>
    {/if}
  {/if}
</div>

<style>
  .tree-node { display: flex; flex-direction: column; }

  .node-row {
    display: flex;
    align-items: center;
    gap: 5px;
    padding-top: 3px;
    padding-bottom: 3px;
    padding-right: 8px;
    cursor: pointer;
    border-radius: 0;
    transition: background 0.08s;
    min-height: 26px;
  }
  .node-row:hover   { background: var(--bg-hover); }
  .node-row.active  { background: var(--bg-active); border-right: 2px solid var(--accent); }
  .node-row:focus   { outline: none; background: var(--bg-hover); }

  .chevron {
    width: 12px;
    flex-shrink: 0;
    font-size: 10px;
    color: var(--text-muted);
    text-align: center;
  }

  .icon    { font-size: 13px; flex-shrink: 0; }

  .name {
    flex: 1;
    font-size: 12.5px;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    font-family: var(--font-mono);
  }

  .size {
    font-size: 10px;
    color: var(--text-muted);
    flex-shrink: 0;
  }

  .empty-dir {
    font-size: 11px;
    color: var(--text-muted);
    padding-top: 3px;
    padding-bottom: 3px;
    font-style: italic;
  }
</style>
