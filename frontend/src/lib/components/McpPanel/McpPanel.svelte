<script lang="ts">
  import { onMount } from 'svelte';
  import { mcpApi } from '$lib/api';
  import type { McpServer } from '$lib/api';
  import { toast } from '$lib/stores';

  let servers = $state<(McpServer & { pinging?: boolean; pingResult?: { ok: boolean; msg: string } })[]>([]);
  let loading = $state(true);
  let showForm = $state(false);
  let form = $state<McpServer>({ id: '', name: '', description: '', url: '', enabled: true, type: 'http' });

  onMount(async () => {
    try { servers = await mcpApi.list() as any[]; }
    catch { toast('error', 'Erreur chargement MCP'); }
    finally { loading = false; }
  });

  async function ping(server: typeof servers[0]) {
    server.pinging = true;
    servers = [...servers];
    try {
      const r = await mcpApi.ping(server.id);
      server.pingResult = r.reachable
        ? { ok: true, msg: `✓ HTTP ${r.status}` }
        : { ok: false, msg: `✗ ${r.error}` };
    } catch (e) {
      server.pingResult = { ok: false, msg: String(e) };
    } finally {
      server.pinging = false;
      servers = [...servers];
    }
  }

  async function toggleEnabled(server: typeof servers[0]) {
    try {
      await mcpApi.update(server.id, { ...server, enabled: !server.enabled });
      server.enabled = !server.enabled;
      servers = [...servers];
    } catch { toast('error', 'Mise à jour échouée'); }
  }

  async function deleteServer(id: string) {
    if (!confirm('Supprimer ce serveur MCP ?')) return;
    try {
      await mcpApi.delete(id);
      servers = servers.filter(s => s.id !== id);
      toast('success', 'Serveur supprimé');
    } catch { toast('error', 'Suppression échouée'); }
  }

  async function addServer() {
    if (!form.id || !form.url) return;
    try {
      const s = await mcpApi.add(form);
      servers = [...servers, s as any];
      showForm = false;
      form = { id: '', name: '', description: '', url: '', enabled: true, type: 'http' };
      toast('success', 'Serveur MCP ajouté');
    } catch (e) {
      toast('error', `Ajout échoué: ${e instanceof Error ? e.message : e}`);
    }
  }
</script>

<div class="mcp-panel">
  <div class="panel-header">
    <h2>🔌 Serveurs MCP</h2>
    <button class="btn btn-primary" onclick={() => showForm = !showForm}>
      {showForm ? '✕ Annuler' : '✚ Ajouter'}
    </button>
  </div>

  <div class="panel-body">
    <!-- Formulaire d'ajout -->
    {#if showForm}
      <div class="add-form">
        <h3>Nouveau serveur MCP</h3>
        <div class="form-grid">
          <label class="field">
            <span>ID unique</span>
            <input class="input" bind:value={form.id} placeholder="mon-mcp" />
          </label>
          <label class="field">
            <span>Nom</span>
            <input class="input" bind:value={form.name} placeholder="Mon Serveur MCP" />
          </label>
          <label class="field full">
            <span>URL</span>
            <input class="input" bind:value={form.url} placeholder="http://localhost:3001" type="url" />
          </label>
          <label class="field full">
            <span>Description</span>
            <input class="input" bind:value={form.description} placeholder="Description optionnelle" />
          </label>
        </div>
        <button class="btn btn-primary" onclick={addServer}>➕ Ajouter</button>
      </div>
    {/if}

    <!-- Liste des serveurs -->
    {#if loading}
      <p class="hint">Chargement…</p>
    {:else if servers.length === 0}
      <div class="empty-state">
        <p>🔌</p>
        <p>Aucun serveur MCP configuré</p>
        <p class="sub">Les serveurs MCP étendent les capacités de l'agent</p>
      </div>
    {:else}
      <div class="server-list">
        {#each servers as server (server.id)}
          <div class="server-card {server.enabled ? '' : 'disabled'}">
            <div class="server-top">
              <div class="server-info">
                <span class="server-name">{server.name || server.id}</span>
                {#if server.description}
                  <span class="server-desc">{server.description}</span>
                {/if}
                <code class="server-url">{server.url}</code>
              </div>

              <div class="server-actions">
                <!-- Toggle enabled -->
                <button
                  class="toggle-btn {server.enabled ? 'on' : 'off'}"
                  onclick={() => toggleEnabled(server)}
                  title={server.enabled ? 'Désactiver' : 'Activer'}
                >
                  {server.enabled ? '●' : '○'}
                </button>

                <!-- Ping -->
                <button
                  class="action-btn"
                  onclick={() => ping(server)}
                  disabled={server.pinging}
                  title="Tester la connexion"
                >
                  {server.pinging ? '⏳' : '🔍'}
                </button>

                <!-- Supprimer -->
                <button
                  class="action-btn danger"
                  onclick={() => deleteServer(server.id)}
                  title="Supprimer"
                >🗑</button>
              </div>
            </div>

            {#if server.pingResult}
              <div class="ping-result {server.pingResult.ok ? 'ok' : 'err'}">
                {server.pingResult.msg}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}

    <!-- Info MCP -->
    <div class="info-box">
      <p class="info-title">💡 À propos du MCP (Model Context Protocol)</p>
      <p>Les serveurs MCP exposent des outils supplémentaires à l'agent via JSON-RPC. Exemple : accès étendu au système de fichiers, bases de données, APIs externes.</p>
      <p>L'agent découvrira automatiquement les outils des serveurs activés.</p>
    </div>
  </div>
</div>

<style>
  .mcp-panel { display: flex; flex-direction: column; height: 100%; }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--bg-secondary);
    flex-shrink: 0;
  }
  .panel-header h2 { font-size: 14px; font-weight: 600; }

  .panel-body {
    flex: 1;
    overflow-y: auto;
    padding: 16px 18px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .add-form {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 14px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  .add-form h3 { font-size: 13px; font-weight: 500; color: var(--text-secondary); }

  .form-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field.full { grid-column: 1 / -1; }
  .field span { font-size: 11.5px; color: var(--text-muted); }

  .input {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 7px 9px;
    border-radius: 5px;
    font-size: 12.5px;
    font-family: var(--font-mono);
    outline: none;
  }
  .input:focus { border-color: var(--border-focus); }

  .hint { font-size: 12px; color: var(--text-muted); text-align: center; padding: 20px; }

  .empty-state {
    text-align: center;
    padding: 40px 20px;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .empty-state p:first-child { font-size: 32px; }
  .empty-state p { font-size: 13px; color: var(--text-secondary); }
  .empty-state .sub { font-size: 11.5px; color: var(--text-muted); }

  .server-list { display: flex; flex-direction: column; gap: 8px; }

  .server-card {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    transition: opacity 0.15s;
  }
  .server-card.disabled { opacity: 0.5; }

  .server-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
  }

  .server-info { display: flex; flex-direction: column; gap: 3px; flex: 1; }
  .server-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
  .server-desc { font-size: 11.5px; color: var(--text-muted); }
  .server-url { font-size: 11px; color: var(--teal); }

  .server-actions { display: flex; gap: 5px; align-items: center; flex-shrink: 0; }

  .toggle-btn {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 20px;
    padding: 3px 10px;
    font-size: 16px;
    cursor: pointer;
    transition: all 0.12s;
  }
  .toggle-btn.on  { color: var(--green); border-color: rgba(74,222,128,0.3); }
  .toggle-btn.off { color: var(--text-muted); }

  .action-btn {
    background: transparent;
    border: 1px solid var(--border);
    border-radius: 5px;
    padding: 4px 7px;
    font-size: 13px;
    cursor: pointer;
    transition: all 0.1s;
    color: var(--text-muted);
  }
  .action-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .action-btn.danger:hover { background: var(--red-dim); color: var(--red); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .ping-result {
    font-size: 11.5px;
    font-family: var(--font-mono);
    padding: 4px 8px;
    border-radius: 4px;
  }
  .ping-result.ok  { background: var(--green-dim); color: var(--green); }
  .ping-result.err { background: var(--red-dim); color: var(--red); }

  .info-box {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .info-title { font-size: 12px; font-weight: 500; color: var(--text-secondary); }
  .info-box p { font-size: 11.5px; color: var(--text-muted); line-height: 1.5; }
</style>
