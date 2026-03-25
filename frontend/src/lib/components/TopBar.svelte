<script lang="ts">
  import {
    showFileExplorer, showSessionSidebar, showSettings,
    showTemplates, showSkillSelector, activePanel,
    selectedModel, models, activeSkill, isStreaming, skills,
    appVersion
  } from '$lib/stores';
  import type { PanelId } from '$lib/stores';

  const panels: { id: PanelId; label: string; icon: string }[] = [
    { id: 'chat',      label: 'Chat',       icon: '💬' },
    { id: 'editor',    label: 'Éditeur',    icon: '📝' },
    { id: 'terminal',  label: 'Terminal',   icon: '⌨️' },
    { id: 'documents', label: 'Documents',  icon: '📚' },
    { id: 'mcp',       label: 'MCP',        icon: '🔌' },
  ];

  function setPanel(id: PanelId) {
    activePanel.set(id);
  }
</script>

<header class="topbar">
  <!-- Logo -->
  <div class="logo">
    <span class="logo-icon">◎</span>
    <span class="logo-text">OllamaStudio</span>
    {#if $appVersion}
      <span class="logo-version">v{$appVersion}</span>
    {/if}
  </div>

  <!-- Navigation panels -->
  <nav class="panel-tabs">
    {#each panels as panel}
      <button
        class="tab {$activePanel === panel.id ? 'active' : ''}"
        onclick={() => setPanel(panel.id)}
      >
        <span>{panel.icon}</span>
        <span>{panel.label}</span>
      </button>
    {/each}
  </nav>

  <!-- Modèle + skill -->
  <div class="toolbar">
    <!-- Skill active -->
    <button
      class="skill-badge"
      style="border-color: {$activeSkill?.color || 'var(--border)'}; color: {$activeSkill?.color || 'var(--text-muted)'}"
      onclick={() => showSkillSelector.update(v => !v)}
      title="Sélectionner une skill"
    >
      {$activeSkill?.icon || '🤖'} {$activeSkill?.name || 'Skill'}
    </button>

    <!-- Sélecteur modèle -->
    <select
      class="model-select"
      bind:value={$selectedModel}
      disabled={$isStreaming}
    >
      {#each $models as m}
        <option value={m.name}>{m.name}</option>
      {/each}
      {#if $models.length === 0}
        <option value={$selectedModel}>{$selectedModel}</option>
      {/if}
    </select>

    <!-- Actions -->
    <div class="actions">
      <button
        class="icon-btn {$showTemplates ? 'active' : ''}"
        onclick={() => showTemplates.update(v => !v)}
        title="Templates"
      >📋</button>

      <button
        class="icon-btn {$showSessionSidebar ? 'active' : ''}"
        onclick={() => showSessionSidebar.update(v => !v)}
        title="Sessions"
      >🗂️</button>

      <button
        class="icon-btn {$showFileExplorer ? 'active' : ''}"
        onclick={() => showFileExplorer.update(v => !v)}
        title="Explorateur"
      >🗄️</button>

      <button
        class="icon-btn {$showSettings ? 'active' : ''}"
        onclick={() => showSettings.update(v => !v)}
        title="Paramètres"
      >⚙️</button>
    </div>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    gap: 0;
    height: 46px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    padding: 0 8px;
    flex-shrink: 0;
    user-select: none;
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 12px 0 4px;
    border-right: 1px solid var(--border);
    margin-right: 8px;
    flex-shrink: 0;
  }

  .logo-icon {
    font-size: 18px;
    color: var(--accent);
  }

  .logo-text {
    font-weight: 600;
    font-size: 13px;
    color: var(--text-primary);
    letter-spacing: 0.02em;
  }

  .logo-version {
    font-size: 10px;
    color: var(--text-muted);
    font-weight: 400;
    opacity: 0.7;
  }

  .panel-tabs {
    display: flex;
    gap: 2px;
    flex: 1;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 6px 10px;
    border-radius: 5px;
    font-size: 12.5px;
    color: var(--text-muted);
    background: transparent;
    border: none;
    cursor: pointer;
    transition: all 0.12s;
    white-space: nowrap;
  }

  .tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .tab.active { background: var(--bg-active); color: var(--text-primary); }

  .toolbar {
    display: flex;
    align-items: center;
    gap: 6px;
    flex-shrink: 0;
  }

  .skill-badge {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 9px;
    border-radius: 20px;
    border: 1px solid;
    background: transparent;
    font-size: 11.5px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.12s;
  }
  .skill-badge:hover { background: var(--bg-hover); }

  .model-select {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 4px 8px;
    border-radius: var(--input-radius);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    max-width: 180px;
  }
  .model-select:focus { outline: none; border-color: var(--border-focus); }
  .model-select:disabled { opacity: 0.5; cursor: not-allowed; }

  .actions {
    display: flex;
    gap: 2px;
  }

  .icon-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 30px;
    height: 30px;
    border-radius: 5px;
    border: none;
    background: transparent;
    font-size: 15px;
    cursor: pointer;
    color: var(--text-muted);
    transition: all 0.12s;
  }
  .icon-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .icon-btn.active { background: var(--bg-active); color: var(--accent-light); }
</style>
