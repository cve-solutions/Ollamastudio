<script lang="ts">
  import { onMount } from 'svelte';
  import { showTemplates, activePanel, toast } from '$lib/stores';
  import { templatesApi } from '$lib/api';
  import type { Template } from '$lib/api';

  let templates = $state<Template[]>([]);
  let loading = $state(true);
  let search = $state('');
  let selected = $state<Template | null>(null);
  let vars = $state<Record<string, string>>({});

  onMount(async () => {
    try {
      templates = await templatesApi.list();
    } catch (e) {
      toast('error', 'Erreur chargement templates');
    } finally {
      loading = false;
    }
  });

  const categories = $derived([...new Set(templates.map(t => t.category))].sort());

  const filtered = $derived(
    templates.filter(t =>
      !search || t.name.toLowerCase().includes(search.toLowerCase()) ||
      t.category.toLowerCase().includes(search.toLowerCase())
    )
  );

  function selectTemplate(t: Template) {
    selected = t;
    vars = Object.fromEntries(t.variables.map(v => [v, '']));
  }

  function buildPrompt(): string {
    if (!selected) return '';
    let prompt = selected.content;
    for (const [k, v] of Object.entries(vars)) {
      prompt = prompt.replaceAll(`{{${k}}}`, v);
    }
    return prompt;
  }

  function useTemplate() {
    const prompt = buildPrompt();
    if (!prompt.trim()) return;

    // Dispatch un événement custom pour injecter dans le chat
    window.dispatchEvent(new CustomEvent('inject-prompt', { detail: prompt }));
    showTemplates.set(false);
    activePanel.set('chat');
  }

  function close() { showTemplates.set(false); }
</script>

<!-- Overlay -->
<div class="overlay" onclick={close} role="presentation"></div>

<div class="drawer" role="dialog" aria-label="Templates de prompts">
  <div class="drawer-header">
    <h2>📋 Templates de prompts</h2>
    <button class="close-btn" onclick={close}>✕</button>
  </div>

  <div class="drawer-body">
    <!-- Liste à gauche -->
    <div class="template-list">
      <input
        class="search-input"
        type="search"
        bind:value={search}
        placeholder="Rechercher…"
      />

      {#if loading}
        <p class="hint">Chargement…</p>
      {:else}
        {#each categories as cat}
          {@const catTemplates = filtered.filter(t => t.category === cat)}
          {#if catTemplates.length}
            <div class="category-group">
              <p class="category-label">{cat}</p>
              {#each catTemplates as t}
                <button
                  class="tpl-item {selected?.id === t.id ? 'active' : ''}"
                  onclick={() => selectTemplate(t)}
                >
                  <span>{t.icon}</span>
                  <span>{t.name}</span>
                </button>
              {/each}
            </div>
          {/if}
        {/each}
      {/if}
    </div>

    <!-- Détail à droite -->
    <div class="template-detail">
      {#if selected}
        <div class="detail-header">
          <h3>{selected.icon} {selected.name}</h3>
          <span class="cat-badge">{selected.category}</span>
        </div>

        <!-- Variables -->
        {#if selected.variables.length > 0}
          <div class="vars-section">
            <p class="vars-title">Variables à remplir</p>
            {#each selected.variables as v}
              <label class="var-field">
                <span class="var-label">{'{{'}{v}{'}}'}</span>
                <textarea
                  class="var-input"
                  bind:value={vars[v]}
                  placeholder="Valeur de {v}…"
                  rows="3"
                ></textarea>
              </label>
            {/each}
          </div>
        {/if}

        <!-- Aperçu -->
        <div class="preview-section">
          <p class="vars-title">Aperçu du prompt</p>
          <pre class="preview">{buildPrompt()}</pre>
        </div>

        <button class="btn btn-primary use-btn" onclick={useTemplate}>
          ▶ Utiliser ce template
        </button>
      {:else}
        <div class="empty-detail">
          <p>← Sélectionnez un template</p>
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: 100;
  }

  .drawer {
    position: fixed;
    top: 46px;
    right: 0;
    bottom: 0;
    width: 720px;
    max-width: 95vw;
    background: var(--bg-secondary);
    border-left: 1px solid var(--border);
    z-index: 101;
    display: flex;
    flex-direction: column;
    animation: slide-in 0.18s ease-out;
  }

  @keyframes slide-in {
    from { transform: translateX(40px); opacity: 0; }
    to   { transform: translateX(0);    opacity: 1; }
  }

  .drawer-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .drawer-header h2 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 15px;
    cursor: pointer;
    padding: 3px 6px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); }

  .drawer-body {
    display: flex;
    flex: 1;
    overflow: hidden;
  }

  .template-list {
    width: 220px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    padding: 10px 8px;
    flex-shrink: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .search-input {
    width: 100%;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 6px 9px;
    border-radius: 6px;
    font-size: 12px;
    outline: none;
    margin-bottom: 8px;
  }
  .search-input:focus { border-color: var(--border-focus); }

  .category-group { display: flex; flex-direction: column; gap: 2px; margin-bottom: 8px; }

  .category-label {
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.07em;
    color: var(--text-muted);
    padding: 2px 6px;
  }

  .tpl-item {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 8px;
    border-radius: 5px;
    border: none;
    background: transparent;
    color: var(--text-secondary);
    font-size: 12.5px;
    cursor: pointer;
    text-align: left;
    transition: all 0.1s;
  }
  .tpl-item:hover { background: var(--bg-hover); color: var(--text-primary); }
  .tpl-item.active { background: var(--accent-dim); color: var(--accent-light); border: 1px solid var(--accent-border); }

  .hint { text-align: center; color: var(--text-muted); font-size: 12px; padding: 20px; }

  .template-detail {
    flex: 1;
    padding: 16px 18px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .detail-header {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .detail-header h3 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .cat-badge {
    font-size: 10.5px;
    padding: 2px 7px;
    border-radius: 8px;
    background: var(--accent-dim);
    border: 1px solid var(--accent-border);
    color: var(--accent-light);
  }

  .vars-section, .preview-section { display: flex; flex-direction: column; gap: 8px; }

  .vars-title {
    font-size: 11px;
    font-weight: 500;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-muted);
  }

  .var-field { display: flex; flex-direction: column; gap: 4px; }

  .var-label {
    font-size: 11.5px;
    font-family: var(--font-mono);
    color: var(--teal);
  }

  .var-input {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 7px 9px;
    border-radius: 5px;
    font-size: 12.5px;
    font-family: var(--font-mono);
    resize: vertical;
    outline: none;
    transition: border-color 0.12s;
  }
  .var-input:focus { border-color: var(--border-focus); }

  .preview {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 10px 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 220px;
    overflow-y: auto;
  }

  .use-btn {
    align-self: flex-start;
    margin-top: 4px;
  }

  .empty-detail {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    font-size: 13px;
  }
</style>
