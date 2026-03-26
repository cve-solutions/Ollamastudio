<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { get } from 'svelte/store';
  import {
    showFileExplorer, showSessionSidebar, showSettings,
    showTemplates, showSkillSelector, toasts, activePanel,
    models, modelsLoading, sessions, ollamaConfig,
    selectedModel, skills, toast, appSettings, debugMode, appVersion
  } from '$lib/stores';
  import { modelsApi, sessionsApi, skillsApi, settingsApi } from '$lib/api';

  import SessionSidebar   from '$lib/components/SessionSidebar/SessionSidebar.svelte';
  import FileExplorer     from '$lib/components/FileExplorer/FileExplorer.svelte';
  import TopBar           from '$lib/components/TopBar.svelte';
  import SettingsModal    from '$lib/components/Settings/SettingsModal.svelte';
  import TemplateDrawer   from '$lib/components/TemplatePanel/TemplateDrawer.svelte';
  import SkillSelector    from '$lib/components/SkillSelector/SkillSelector.svelte';
  import ToastContainer   from '$lib/components/ToastContainer.svelte';

  let { children } = $props();

  onMount(async () => {
    modelsLoading.set(true);
    try {
      // 1. Charge les paramètres depuis la BDD
      const allSettings = await settingsApi.list();
      appSettings.set(allSettings);

      // Extrait la config Ollama depuis les settings DB
      const settingsMap = Object.fromEntries(allSettings.map(s => [s.key, s.value]));
      const cfg = {
        base_url: (settingsMap['ollama_base_url'] as string) || 'http://localhost:11434',
        api_mode: (settingsMap['ollama_api_mode'] as string) || 'openai',
        default_model: (settingsMap['ollama_default_model'] as string) || 'qwen3-coder',
      };
      ollamaConfig.set(cfg);

      // Sync debug mode
      debugMode.set(!!settingsMap['debug_mode']);

      // 2. Charge version, modèles, sessions et skills en parallèle
      // Chaque appel est indépendant — un échec (ex: Ollama down) ne bloque pas les autres
      const [healthData, modelList, sessionList, skillList] = await Promise.all([
        fetch('/health').then(r => r.json()).catch(() => ({ version: '' })),
        modelsApi.list(cfg.base_url).catch(() => [] as any[]),
        sessionsApi.list().catch(() => []),
        skillsApi.list().catch(() => []),
      ]);
      if (healthData.version) appVersion.set(healthData.version);
      models.set(modelList);
      sessions.set(sessionList);
      skills.set(skillList);
      if (modelList.length > 0) {
        selectedModel.set(cfg.default_model || modelList[0].name);
      }
    } catch (e) {
      toast('error', `Erreur d'initialisation: ${e instanceof Error ? e.message : e}`);
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
