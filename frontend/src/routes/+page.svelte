<script lang="ts">
  import { activePanel } from '$lib/stores';
  import ChatPanel      from '$lib/components/ChatPanel/ChatPanel.svelte';
  import CodeEditorPanel from '$lib/components/CodeEditor/CodeEditorPanel.svelte';
  import TerminalPanel  from '$lib/components/Terminal/TerminalPanel.svelte';
  import DocumentsPanel from '$lib/components/DocumentPanel/DocumentsPanel.svelte';
  import McpPanel       from '$lib/components/McpPanel/McpPanel.svelte';
</script>

<div class="page">
  {#if $activePanel === 'chat'}
    <ChatPanel />
  {:else if $activePanel === 'editor'}
    <CodeEditorPanel />
  {:else if $activePanel === 'terminal'}
    <TerminalPanel />
  {:else if $activePanel === 'documents'}
    <DocumentsPanel />
  {:else if $activePanel === 'mcp'}
    <McpPanel />
  {/if}
</div>

<style>
  .page {
    height: 100%;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
