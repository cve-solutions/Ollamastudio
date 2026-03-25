<script lang="ts">
  import { onMount } from 'svelte';
  import { documentsApi } from '$lib/api';
  import type { Document } from '$lib/api';
  import { toast } from '$lib/stores';

  let docs = $state<Document[]>([]);
  let loading = $state(true);
  let importing = $state(false);
  let searching = $state(false);
  let searchQuery = $state('');
  let searchResults = $state<any[]>([]);
  let folderPath = $state('');
  let dragOver = $state(false);

  onMount(async () => {
    await loadDocs();
  });

  async function loadDocs() {
    loading = true;
    try {
      docs = await documentsApi.list();
    } catch (e) {
      toast('error', 'Erreur chargement documents');
    } finally {
      loading = false;
    }
  }

  async function importFolder() {
    if (!folderPath.trim()) return;
    importing = true;
    try {
      const result = await documentsApi.importFolder(folderPath.trim()) as any;
      toast('success', `${result.imported} fichier(s) importé(s)${result.skipped ? `, ${result.skipped} ignoré(s)` : ''}`);
      if (result.errors?.length) {
        const errorSummary = result.errors.slice(0, 3).map((e: any) => e.error || e.file).join('; ');
        toast('warning', `${result.errors.length} erreur(s): ${errorSummary}`);
        console.warn('[Import dossier] Erreurs détaillées:', result.errors);
      }
      folderPath = '';
      await loadDocs();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast('error', `Import échoué: ${msg}`);
      console.error('[Import dossier] Exception:', e);
    } finally {
      importing = false;
    }
  }

  async function uploadFiles(files: FileList | null) {
    if (!files || files.length === 0) return;
    importing = true;
    const fd = new FormData();
    for (const f of files) fd.append('files', f);
    try {
      const resp = await fetch('/api/documents/upload', { method: 'POST', body: fd });
      if (!resp.ok) {
        const err = await resp.json().catch(() => ({ detail: resp.statusText }));
        throw new Error(err.detail || `Erreur HTTP ${resp.status}`);
      }
      const result = await resp.json();
      const imported = result.imported?.length ?? 0;
      toast('success', `${imported} fichier(s) importé(s)`);
      if (result.errors?.length) {
        const errorSummary = result.errors.slice(0, 3).map((e: any) => `${e.file}: ${e.error}`).join('; ');
        toast('warning', `${result.errors.length} erreur(s): ${errorSummary}`);
        console.warn('[Upload] Erreurs détaillées:', result.errors);
      }
      await loadDocs();
    } catch (e) {
      const msg = e instanceof Error ? e.message : String(e);
      toast('error', `Upload échoué: ${msg}`);
      console.error('[Upload] Exception:', e);
    } finally {
      importing = false;
    }
  }

  async function search() {
    if (!searchQuery.trim()) { searchResults = []; return; }
    searching = true;
    try {
      const r = await documentsApi.search(searchQuery);
      searchResults = (r as any).results ?? [];
    } catch {
      toast('error', 'Recherche échouée');
    } finally {
      searching = false;
    }
  }

  async function deleteDoc(id: number) {
    if (!confirm('Supprimer ce document ?')) return;
    try {
      await documentsApi.delete(id);
      docs = docs.filter(d => d.id !== id);
      toast('success', 'Document supprimé');
    } catch { toast('error', 'Suppression échouée'); }
  }

  function formatSize(bytes: number): string {
    if (bytes < 1024) return bytes + ' B';
    if (bytes < 1024 * 1024) return (bytes / 1024).toFixed(1) + ' Ko';
    return (bytes / 1024 / 1024).toFixed(1) + ' Mo';
  }

  function handleDrop(e: DragEvent) {
    e.preventDefault();
    dragOver = false;
    uploadFiles(e.dataTransfer?.files ?? null);
  }
</script>

<div class="docs-panel">
  <div class="panel-header">
    <h2>📚 Documents importés</h2>
    <span class="doc-count">{docs.length} document(s) · {docs.reduce((a, d) => a + d.chunk_count, 0)} chunks</span>
  </div>

  <div class="panel-body">
    <!-- Import par chemin serveur -->
    <div class="section">
      <h3>📁 Importer un dossier (côté serveur)</h3>
      <div class="folder-row">
        <input
          class="path-input"
          bind:value={folderPath}
          placeholder="/host/chemin/vers/projet ou /app/workspace/src"
          onkeydown={(e) => e.key === 'Enter' && importFolder()}
        />
        <button
          class="btn btn-primary"
          onclick={importFolder}
          disabled={importing || !folderPath.trim()}
        >
          {importing ? '⏳ Import…' : '📥 Importer'}
        </button>
      </div>
      <p class="hint">Chemins serveur : /host/... (fichiers hôte), /app/workspace/... ou /app/documents/...</p>
    </div>

    <!-- Upload drag-and-drop -->
    <div class="section">
      <h3>⬆ Upload de fichiers</h3>
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="drop-zone {dragOver ? 'drag-over' : ''}"
        ondragover={(e) => { e.preventDefault(); dragOver = true; }}
        ondragleave={() => dragOver = false}
        ondrop={handleDrop}
      >
        <span>Glissez des fichiers ici ou</span>
        <label class="upload-label">
          parcourez
          <input type="file" multiple onchange={(e) => uploadFiles((e.target as HTMLInputElement).files)} hidden />
        </label>
        <p class="drop-hint">
          Formats : .py .rs .js .ts .md .txt .yaml .json .sh et autres fichiers texte
        </p>
      </div>
    </div>

    <!-- Recherche -->
    <div class="section">
      <h3>🔍 Recherche dans les documents</h3>
      <div class="search-row">
        <input
          class="path-input"
          bind:value={searchQuery}
          placeholder="Rechercher dans le contenu…"
          onkeydown={(e) => e.key === 'Enter' && search()}
        />
        <button class="btn btn-ghost" onclick={search} disabled={searching}>
          {searching ? '⏳' : '🔍'} Chercher
        </button>
      </div>
      {#if searchResults.length > 0}
        <div class="search-results">
          {#each searchResults as r}
            <div class="result-card">
              <div class="result-header">
                <span class="result-file">📄 {r.document}</span>
                <span class="result-chunk">chunk #{r.chunk_index}</span>
              </div>
              <p class="result-excerpt">{r.excerpt}</p>
            </div>
          {/each}
        </div>
      {/if}
    </div>

    <!-- Liste des documents -->
    <div class="section">
      <h3>📋 Documents indexés</h3>
      {#if loading}
        <p class="hint">Chargement…</p>
      {:else if docs.length === 0}
        <p class="hint">Aucun document importé</p>
      {:else}
        <div class="doc-list">
          {#each docs as doc (doc.id)}
            <div class="doc-row">
              <div class="doc-info">
                <span class="doc-name">📄 {doc.filename}</span>
                <span class="doc-meta">
                  {formatSize(doc.size_bytes)} ·
                  {doc.chunk_count} chunks ·
                  {new Date(doc.imported_at).toLocaleDateString('fr-FR')}
                </span>
              </div>
              <button
                class="del-btn"
                onclick={() => deleteDoc(doc.id)}
                title="Supprimer"
              >🗑</button>
            </div>
          {/each}
        </div>
      {/if}
    </div>
  </div>
</div>

<style>
  .docs-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    background: var(--bg-secondary);
  }

  .panel-header h2 { font-size: 14px; font-weight: 600; }

  .doc-count {
    font-size: 11.5px;
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .section { display: flex; flex-direction: column; gap: 10px; }

  .section h3 {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .folder-row, .search-row {
    display: flex;
    gap: 8px;
  }

  .path-input {
    flex: 1;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 8px 10px;
    border-radius: 6px;
    font-size: 12.5px;
    font-family: var(--font-mono);
    outline: none;
  }
  .path-input:focus { border-color: var(--border-focus); }

  .hint { font-size: 11px; color: var(--text-muted); }

  .drop-zone {
    border: 2px dashed var(--border);
    border-radius: 8px;
    padding: 24px;
    text-align: center;
    color: var(--text-muted);
    font-size: 13px;
    cursor: pointer;
    transition: all 0.15s;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 6px;
  }
  .drop-zone.drag-over { border-color: var(--accent); background: var(--accent-dim); }

  .upload-label {
    color: var(--accent-light);
    cursor: pointer;
    text-decoration: underline;
  }

  .drop-hint { font-size: 11px; }

  .search-results {
    display: flex;
    flex-direction: column;
    gap: 6px;
    max-height: 300px;
    overflow-y: auto;
  }

  .result-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 9px 12px;
  }

  .result-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 5px;
  }

  .result-file { font-size: 12px; color: var(--text-primary); font-family: var(--font-mono); }
  .result-chunk { font-size: 10.5px; color: var(--text-muted); }
  .result-excerpt { font-size: 11.5px; color: var(--text-secondary); line-height: 1.5; white-space: pre-wrap; }

  .doc-list { display: flex; flex-direction: column; gap: 4px; }

  .doc-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .doc-info { flex: 1; display: flex; flex-direction: column; gap: 2px; }

  .doc-name { font-size: 12.5px; color: var(--text-primary); font-family: var(--font-mono); }
  .doc-meta { font-size: 11px; color: var(--text-muted); }

  .del-btn {
    background: transparent;
    border: none;
    font-size: 14px;
    cursor: pointer;
    padding: 3px;
    border-radius: 4px;
    transition: all 0.1s;
    opacity: 0.5;
  }
  .del-btn:hover { opacity: 1; background: var(--red-dim); }
</style>
