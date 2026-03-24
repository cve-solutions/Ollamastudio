<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    showFileExplorer, showSessionSidebar, showSettings,
    showTemplates, showSkillSelector, toasts, activePanel,
    models, modelsLoading, sessions, ollamaConfig,
    selectedModel, skills, toast
  } from '$lib/stores';
  import { modelsApi, sessionsApi, skillsApi } from '$lib/api';

  import SessionSidebar   from '$lib/components/SessionSidebar/SessionSidebar.svelte';
  import FileExplorer     from '$lib/components/FileExplorer/FileExplorer.svelte';
  import TopBar           from '$lib/components/TopBar.svelte';
  import SettingsModal    from '$lib/components/Settings/SettingsModal.svelte';
  import TemplateDrawer   from '$lib/components/TemplatePanel/TemplateDrawer.svelte';
  import SkillSelector    from '$lib/components/SkillSelector/SkillSelector.svelte';
  import ToastContainer   from '$lib/components/ToastContainer.svelte';

  let { children } = $props();

  onMount(async () => {
    // Charge modèles, sessions et skills en parallèle
    modelsLoading.set(true);
    const cfg = get(ollamaConfig);
    try {
      const [modelList, sessionList, skillList] = await Promise.all([
        modelsApi.list(cfg.base_url),
        sessionsApi.list(),
        skillsApi.list(),
      ]);
      models.set(modelList);
      sessions.set(sessionList);
      skills.set(skillList);
      if (modelList.length > 0) {
        selectedModel.set(cfg.default_model || modelList[0].name);
      }
    } catch (e) {
      toast('error', `Connexion Ollama impossible: ${e instanceof Error ? e.message : e}`);
    } finally {
      modelsLoading.set(false);
    }
  });
</script>

<div class="app-shell">
  <!-- Barre supérieure -->
  <TopBar />

  <div class="main-area">
    <!-- Sidebar sessions (gauche) -->
    {#if $showSessionSidebar}
      <aside class="sidebar" style="width: var(--sidebar-w)">
        <SessionSidebar />
      </aside>
    {/if}

    <!-- Contenu principal -->
    <main class="content">
      {@render children()}
    </main>

    <!-- Explorateur de fichiers (droite) -->
    {#if $showFileExplorer}
      <aside class="file-panel" style="width: var(--explorer-w)">
        <FileExplorer />
      </aside>
    {/if}
  </div>

  <!-- Modals et drawers -->
  {#if $showSettings}
    <SettingsModal />
  {/if}

  {#if $showTemplates}
    <TemplateDrawer />
  {/if}

  {#if $showSkillSelector}
    <SkillSelector />
  {/if}

  <!-- Toasts -->
  <ToastContainer />
</div>

<style>
  .app-shell {
    display: flex;
    flex-direction: column;
    height: 100vh;
    overflow: hidden;
    background: var(--bg-primary);
  }

  .main-area {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .sidebar {
    flex-shrink: 0;
    background: var(--bg-panel);
    border-right: 1px solid var(--border);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .content {
    flex: 1;
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }

  .file-panel {
    flex-shrink: 0;
    background: var(--bg-panel);
    border-left: 1px solid var(--border);
    overflow: hidden;
    display: flex;
    flex-direction: column;
  }
</style>
