<script lang="ts">
  import type { ToolEvent } from '$lib/stores';

  let { evt }: { evt: ToolEvent } = $props();

  let expanded = $state(false);

  const TOOL_ICONS: Record<string, string> = {
    read_file: '📄',
    write_file: '✍️',
    patch_file: '🩹',
    list_files: '📁',
    run_command: '⌨️',
    grep_files: '🔍',
    git_status: '🌿',
    search_documents: '📚',
  };

  function formatJson(obj: unknown): string {
    try { return JSON.stringify(obj, null, 2); }
    catch { return String(obj); }
  }

  const icon = TOOL_ICONS[evt.name] ?? '⚙️';
  const isResult = evt.type === 'result';
  const hasError = isResult && evt.result && 'error' in evt.result;
</script>

<div class="tool-card {isResult ? 'done' : 'running'} {hasError ? 'error' : ''}">
  <button class="tool-header" onclick={() => expanded = !expanded}>
    <span class="tool-icon">{icon}</span>
    <span class="tool-name">{evt.name}</span>
    {#if !isResult}
      <span class="spinner">↻</span>
    {:else if hasError}
      <span class="badge err">✗ Erreur</span>
    {:else}
      <span class="badge ok">✓</span>
    {/if}
    <span class="expand">{expanded ? '▲' : '▼'}</span>
  </button>

  {#if expanded}
    <div class="tool-body">
      {#if evt.args && Object.keys(evt.args).length > 0}
        <div class="section">
          <span class="label">Arguments</span>
          <pre>{formatJson(evt.args)}</pre>
        </div>
      {/if}
      {#if evt.result}
        <div class="section">
          <span class="label">{hasError ? 'Erreur' : 'Résultat'}</span>
          <pre class="{hasError ? 'err-text' : ''}">{formatJson(evt.result)}</pre>
        </div>
      {/if}
    </div>
  {/if}
</div>

<style>
  .tool-card {
    border: 1px solid var(--border);
    border-radius: var(--panel-radius);
    margin: 6px 0;
    background: var(--bg-tertiary);
    overflow: hidden;
    font-size: 12.5px;
  }
  .tool-card.running { border-color: var(--accent-border); }
  .tool-card.done    { border-color: var(--border); }
  .tool-card.error   { border-color: rgba(248,113,113,0.3); }

  .tool-header {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 10px;
    width: 100%;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-secondary);
    text-align: left;
  }
  .tool-header:hover { background: var(--bg-hover); }

  .tool-icon { font-size: 14px; }

  .tool-name {
    flex: 1;
    font-family: var(--font-mono);
    color: var(--text-primary);
    font-size: 12px;
  }

  .spinner {
    color: var(--accent);
    animation: spin 0.8s linear infinite;
    font-size: 14px;
  }

  .badge {
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 8px;
  }
  .badge.ok  { background: var(--green-dim); color: var(--green); }
  .badge.err { background: var(--red-dim); color: var(--red); }

  .expand { font-size: 10px; color: var(--text-muted); }

  .tool-body {
    padding: 8px 10px;
    border-top: 1px solid var(--border-light);
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .section { display: flex; flex-direction: column; gap: 3px; }

  .label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  pre {
    background: var(--bg-secondary);
    border: 1px solid var(--border-light);
    border-radius: 4px;
    padding: 6px 8px;
    font-family: var(--font-mono);
    font-size: 11.5px;
    color: var(--text-secondary);
    overflow-x: auto;
    max-height: 200px;
    white-space: pre-wrap;
    word-break: break-all;
  }
  pre.err-text { color: var(--red); }
</style>
