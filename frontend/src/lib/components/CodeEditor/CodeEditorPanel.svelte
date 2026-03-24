<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { openFiles, activeFilePath, activePanel, toast } from '$lib/stores';
  import { filesApi } from '$lib/api';
  import { get } from 'svelte/store';

  let editorContainer: HTMLElement;
  let monacoEditor: any = null;
  let monacoInstance: any = null;
  let currentPath: string | null = null;
  let saving = $state(false);

  // Charge Monaco de façon lazy
  onMount(async () => {
    const loader = await import('@monaco-editor/loader');
    monacoInstance = await loader.default.init();
    monacoInstance.editor.defineTheme('ollamastudio', {
      base: 'vs-dark',
      inherit: true,
      rules: [
        { token: 'comment', foreground: '5a5a72', fontStyle: 'italic' },
        { token: 'keyword', foreground: '7c6ff7' },
        { token: 'string', foreground: '4ade80' },
        { token: 'number', foreground: 'fbbf24' },
        { token: 'type', foreground: '2dd4bf' },
        { token: 'function', foreground: '9b8eff' },
      ],
      colors: {
        'editor.background': '#0d0d0f',
        'editor.foreground': '#e8e8f0',
        'editor.lineHighlightBackground': '#16161f',
        'editorLineNumber.foreground': '#3a3a52',
        'editorLineNumber.activeForeground': '#7c6ff7',
        'editor.selectionBackground': '#7c6ff740',
        'editor.findMatchBackground': '#7c6ff730',
        'editorCursor.foreground': '#7c6ff7',
        'editorIndentGuide.background': '#1a1a24',
        'editorIndentGuide.activeBackground': '#2a2a3e',
        'scrollbar.shadow': '#00000000',
        'scrollbarSlider.background': '#ffffff12',
        'scrollbarSlider.hoverBackground': '#ffffff20',
        'editorSuggestWidget.background': '#13131a',
        'editorSuggestWidget.border': '#2a2a3e',
        'editorSuggestWidget.selectedBackground': '#1e1e2a',
        'minimap.background': '#0d0d0f',
      },
    });

    createEditor();
  });

  function createEditor() {
    if (!editorContainer || !monacoInstance) return;
    monacoEditor = monacoInstance.editor.create(editorContainer, {
      value: '',
      language: 'plaintext',
      theme: 'ollamastudio',
      fontSize: 13,
      fontFamily: "'Geist Mono', 'JetBrains Mono', monospace",
      fontLigatures: true,
      lineHeight: 20,
      minimap: { enabled: true, maxColumn: 80 },
      scrollBeyondLastLine: false,
      wordWrap: 'off',
      automaticLayout: true,
      tabSize: 2,
      insertSpaces: true,
      renderWhitespace: 'selection',
      bracketPairColorization: { enabled: true },
      guides: { indentation: true, bracketPairs: true },
      smoothScrolling: true,
      cursorBlinking: 'smooth',
      cursorSmoothCaretAnimation: 'on',
      formatOnPaste: true,
      formatOnType: false,
      suggest: { preview: true },
      inlineSuggest: { enabled: true },
      padding: { top: 12, bottom: 12 },
    });

    // Keyboard shortcut Ctrl+S → sauvegarde
    monacoEditor.addCommand(
      monacoInstance.KeyMod.CtrlCmd | monacoInstance.KeyCode.KeyS,
      saveCurrentFile
    );

    // Marque le fichier comme modifié à chaque changement
    monacoEditor.onDidChangeModelContent(() => {
      const path = get(activeFilePath);
      if (!path) return;
      openFiles.update(files =>
        files.map(f => f.path === path ? { ...f, modified: true } : f)
      );
    });

    // Charge le fichier actif si déjà présent
    const path = get(activeFilePath);
    if (path) loadFileInEditor(path);
  }

  // Réagit aux changements de fichier actif
  $effect(() => {
    const path = $activeFilePath;
    if (path && monacoEditor && path !== currentPath) {
      loadFileInEditor(path);
    }
  });

  function loadFileInEditor(path: string) {
    const file = get(openFiles).find(f => f.path === path);
    if (!file || !monacoEditor || !monacoInstance) return;

    currentPath = path;
    const model = monacoInstance.editor.createModel(
      file.content,
      file.language,
      monacoInstance.Uri.parse(`file:///${path}`)
    );

    // Réutilise le modèle existant si disponible
    const existing = monacoInstance.editor.getModels()
      .find((m: any) => m.uri.toString() === `file:///${path}`);

    monacoEditor.setModel(existing || model);
    if (!existing) existing?.dispose();
  }

  async function saveCurrentFile() {
    const path = get(activeFilePath);
    if (!path || !monacoEditor) return;
    saving = true;
    try {
      const content = monacoEditor.getValue();
      await filesApi.write(path, content);
      openFiles.update(files =>
        files.map(f => f.path === path ? { ...f, content, modified: false } : f)
      );
      toast('success', `${path.split('/').pop()} sauvegardé`);
    } catch (e) {
      toast('error', `Sauvegarde échouée: ${e instanceof Error ? e.message : e}`);
    } finally {
      saving = false;
    }
  }

  function closeFile(path: string) {
    const files = get(openFiles);
    const file = files.find(f => f.path === path);
    if (file?.modified && !confirm(`${path.split('/').pop()} non sauvegardé — fermer quand même ?`)) return;

    openFiles.update(list => list.filter(f => f.path !== path));
    const remaining = get(openFiles);
    if (get(activeFilePath) === path) {
      activeFilePath.set(remaining[remaining.length - 1]?.path ?? null);
      if (!remaining.length) activePanel.set('chat');
    }
    // Dispose le modèle Monaco
    monacoInstance?.editor.getModels()
      .find((m: any) => m.uri.toString() === `file:///${path}`)
      ?.dispose();
  }

  onDestroy(() => { monacoEditor?.dispose(); });
</script>

<div class="editor-panel">
  <!-- Onglets -->
  <div class="tabs-bar">
    {#each $openFiles as file (file.path)}
      {@const name = file.path.split('/').pop() ?? file.path}
      <div
        class="tab {$activeFilePath === file.path ? 'active' : ''}"
        onclick={() => activeFilePath.set(file.path)}
        onkeydown={(e) => e.key === 'Enter' && activeFilePath.set(file.path)}
        role="button"
        tabindex="0"
      >
        <span class="tab-name">{name}</span>
        {#if file.modified}
          <span class="modified-dot" title="Non sauvegardé">●</span>
        {/if}
        <button
          class="close-tab"
          onclick={(e) => { e.stopPropagation(); closeFile(file.path); }}
          title="Fermer"
        >✕</button>
      </div>
    {:else}
      <div class="no-file">Aucun fichier ouvert — naviguez dans l'explorateur →</div>
    {/each}

    {#if $openFiles.length > 0}
      <button
        class="save-btn {saving ? 'saving' : ''}"
        onclick={saveCurrentFile}
        title="Sauvegarder (Ctrl+S)"
        disabled={saving}
      >
        {saving ? '⏳' : '💾'} Sauvegarder
      </button>
    {/if}
  </div>

  <!-- Éditeur Monaco -->
  <div class="editor-wrap" bind:this={editorContainer}></div>

  <!-- Barre de statut -->
  {#if $activeFilePath}
    <div class="status-bar">
      <span class="status-path">{$activeFilePath}</span>
      <span class="status-lang">
        {$openFiles.find(f => f.path === $activeFilePath)?.language ?? ''}
      </span>
    </div>
  {/if}
</div>

<style>
  .editor-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0d0d0f;
  }

  .tabs-bar {
    display: flex;
    align-items: center;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    overflow-x: auto;
    flex-shrink: 0;
    min-height: 36px;
  }
  .tabs-bar::-webkit-scrollbar { height: 0; }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 12px;
    height: 36px;
    border: none;
    background: transparent;
    color: var(--text-muted);
    font-size: 12px;
    font-family: var(--font-mono);
    cursor: pointer;
    border-right: 1px solid var(--border);
    white-space: nowrap;
    transition: all 0.1s;
    position: relative;
  }
  .tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .tab.active {
    background: #0d0d0f;
    color: var(--text-primary);
    border-bottom: 2px solid var(--accent);
  }

  .tab-name { max-width: 120px; overflow: hidden; text-overflow: ellipsis; }

  .modified-dot { color: var(--amber); font-size: 10px; }

  .close-tab {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 11px;
    cursor: pointer;
    padding: 1px 3px;
    border-radius: 3px;
    line-height: 1;
    opacity: 0;
    transition: all 0.1s;
  }
  .tab:hover .close-tab { opacity: 1; }
  .close-tab:hover { background: var(--red-dim); color: var(--red); }

  .save-btn {
    margin-left: auto;
    margin-right: 8px;
    padding: 4px 10px;
    background: var(--accent-dim);
    border: 1px solid var(--accent-border);
    color: var(--accent-light);
    border-radius: 5px;
    font-size: 11.5px;
    cursor: pointer;
    white-space: nowrap;
    transition: all 0.12s;
    flex-shrink: 0;
  }
  .save-btn:hover:not(:disabled) { background: var(--accent); color: #fff; }
  .save-btn:disabled { opacity: 0.5; }

  .no-file {
    font-size: 12px;
    color: var(--text-muted);
    padding: 0 16px;
    flex: 1;
    display: flex;
    align-items: center;
  }

  .editor-wrap {
    flex: 1;
    overflow: hidden;
  }

  .status-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 3px 12px;
    background: var(--accent);
    font-size: 11px;
    font-family: var(--font-mono);
    flex-shrink: 0;
  }

  .status-path { color: rgba(255,255,255,0.85); }
  .status-lang { color: rgba(255,255,255,0.7); text-transform: uppercase; letter-spacing: 0.05em; }
</style>
