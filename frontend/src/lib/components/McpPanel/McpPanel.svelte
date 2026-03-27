<script lang="ts">
  import { onMount } from 'svelte';
  import { mcpApi } from '$lib/api';
  import type { McpServer } from '$lib/api';
  import { toast } from '$lib/stores';

  let servers = $state<(McpServer & { pinging?: boolean; pingResult?: { ok: boolean; msg: string } })[]>([]);
  let loading = $state(true);

  type FormMode = 'closed' | 'add' | 'edit';
  let formMode: FormMode = $state('closed');
  let editingId = $state('');

  const emptyForm = (): McpServer => ({
    id: '', name: '', description: '', url: '', enabled: true,
    type: 'http', auth_token: '', organization: '', category: '',
  });
  let form = $state<McpServer>(emptyForm());

  const SERVER_TYPES = [
    { value: 'http', label: 'MCP JSON-RPC (HTTP)' },
    { value: 'github', label: 'GitHub API' },
    { value: 'gitlab', label: 'GitLab API' },
  ];

  onMount(async () => {
    try { servers = await mcpApi.list() as any[]; }
    catch { toast('error', 'Erreur chargement MCP'); }
    finally { loading = false; }
  });

  function openAdd() {
    form = emptyForm();
    editingId = '';
    formMode = 'add';
  }

  function openEdit(server: McpServer) {
    form = { ...server };
    editingId = server.id;
    formMode = 'edit';
  }

  function closeForm() {
    formMode = 'closed';
    editingId = '';
  }

  async function saveServer() {
    if (!form.url) { toast('error', 'URL requise'); return; }

    try {
      if (formMode === 'add') {
        if (!form.id) { toast('error', 'ID requis'); return; }
        const s = await mcpApi.add(form);
        servers = [...servers, s as any];
        toast('success', `Serveur "${form.name}" ajouté`);
      } else {
        await mcpApi.update(editingId, form);
        servers = servers.map(s => s.id === editingId ? { ...form } as any : s);
        toast('success', `Serveur "${form.name}" modifié`);
      }
      closeForm();
    } catch (e) {
      toast('error', `Erreur: ${e instanceof Error ? e.message : e}`);
    }
  }

  async function ping(server: typeof servers[0]) {
    server.pinging = true;
    servers = [...servers];
    try {
      const r = await mcpApi.ping(server.id);
      if (!r.reachable) {
        server.pingResult = { ok: false, msg: `✗ ${r.error}` };
      } else if (r.authenticated) {
        server.pingResult = { ok: true, msg: `✓ Connecté (HTTP ${r.status})` };
      } else {
        const detail = r.detail?.message || `HTTP ${r.status}`;
        server.pingResult = { ok: false, msg: `✗ Auth échouée : ${detail}` };
      }
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
    const srv = servers.find(s => s.id === id);
    if (!confirm(`Supprimer "${srv?.name || id}" ?`)) return;
    try {
      await mcpApi.delete(id);
      servers = servers.filter(s => s.id !== id);
      toast('success', 'Serveur supprimé');
    } catch { toast('error', 'Suppression échouée'); }
  }
</script>

<div class="mcp-panel">
  <div class="panel-header">
    <h2>Serveurs MCP</h2>
    <button class="btn btn-primary" onclick={() => formMode === 'closed' ? openAdd() : closeForm()}>
      {formMode !== 'closed' ? '← Retour' : '+ Ajouter'}
    </button>
  </div>

  <div class="panel-body">
    <!-- Formulaire ajout / modification -->
    {#if formMode !== 'closed'}
      <div class="form-card">
        <h3>{formMode === 'add' ? 'Nouveau serveur MCP' : `Modifier : ${form.name || form.id}`}</h3>
        <div class="form-grid">
          {#if formMode === 'add'}
            <label class="field">
              <span>ID unique</span>
              <input class="input" bind:value={form.id} placeholder="mon-mcp" />
            </label>
          {/if}
          <label class="field">
            <span>Nom</span>
            <input class="input" bind:value={form.name} placeholder="Mon Serveur" />
          </label>
          <label class="field">
            <span>Type</span>
            <select class="input" bind:value={form.type}>
              {#each SERVER_TYPES as t}
                <option value={t.value}>{t.label}</option>
              {/each}
            </select>
          </label>
          <label class="field full">
            <span>URL</span>
            <input class="input" bind:value={form.url} placeholder={
              form.type === 'github' ? 'https://api.github.com' :
              form.type === 'gitlab' ? 'https://gitlab.com' :
              'http://127.0.0.1:9100'
            } type="url" />
          </label>
          <label class="field full">
            <span>Description</span>
            <input class="input" bind:value={form.description} placeholder="Description du serveur" />
          </label>
          {#if form.type === 'github' || form.type === 'gitlab'}
            <label class="field full">
              <span>Token d'authentification ({form.type === 'github' ? 'Personal Access Token' : 'Private-Token'})</span>
              <input class="input" bind:value={form.auth_token} type="password" placeholder="ghp_... ou glpat-..." />
            </label>
            <label class="field">
              <span>Organisation / Groupe (optionnel)</span>
              <input class="input" bind:value={form.organization} placeholder="mon-org" />
            </label>
          {/if}
          <label class="field">
            <span>Catégorie (optionnel)</span>
            <input class="input" bind:value={form.category} placeholder="Système, Git, API..." />
          </label>
        </div>
        <div class="form-actions">
          <button class="btn" onclick={closeForm}>Annuler</button>
          <button class="btn btn-primary" onclick={saveServer}>
            {formMode === 'add' ? 'Ajouter' : 'Enregistrer'}
          </button>
        </div>
      </div>
    {/if}

    <!-- Liste des serveurs -->
    {#if loading}
      <p class="hint">Chargement…</p>
    {:else if servers.length === 0}
      <div class="empty-state">
        <p class="icon">🔌</p>
        <p>Aucun serveur MCP configuré</p>
        <p class="sub">Les serveurs MCP étendent les capacités de l'agent IA</p>
      </div>
    {:else}
      <div class="server-list">
        {#each servers as server (server.id)}
          <div class="server-card {server.enabled ? '' : 'disabled'}">
            <div class="server-top">
              <div class="server-info">
                <div class="server-name-row">
                  <span class="server-name">{server.name || server.id}</span>
                  <span class="server-type-badge">{server.type || 'http'}</span>
                  {#if server.category}
                    <span class="server-cat-badge">{server.category}</span>
                  {/if}
                </div>
                {#if server.description}
                  <span class="server-desc">{server.description}</span>
                {/if}
                <code class="server-url">{server.url}</code>
                {#if server.auth_token}
                  <span class="server-auth">Token configuré</span>
                {/if}
              </div>

              <div class="server-actions">
                <button
                  class="toggle-btn {server.enabled ? 'on' : 'off'}"
                  onclick={() => toggleEnabled(server)}
                  title={server.enabled ? 'Désactiver' : 'Activer'}
                >{server.enabled ? '●' : '○'}</button>

                <button class="action-btn" onclick={() => openEdit(server)} title="Modifier">✎</button>
                <button class="action-btn" onclick={() => ping(server)} disabled={server.pinging} title="Tester">{server.pinging ? '⏳' : '🔍'}</button>
                <button class="action-btn danger" onclick={() => deleteServer(server.id)} title="Supprimer">🗑</button>
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

    <div class="info-box">
      <p class="info-title">Serveurs MCP (Model Context Protocol)</p>
      <p>Les serveurs MCP exposent des outils au LLM. Les outils des serveurs <strong>activés</strong> sont automatiquement injectés dans chaque conversation.</p>
      <p><strong>Agent MCP</strong> : commandes système (root) — fichiers, packages, services, cron, réseau</p>
      <p><strong>GitHub / GitLab</strong> : repos, issues, PRs, pipelines via API REST</p>
    </div>
  </div>
</div>

<style>
  .mcp-panel { display: flex; flex-direction: column; height: 100%; }

  .panel-header {
    display: flex; align-items: center; justify-content: space-between;
    padding: 12px 18px; border-bottom: 1px solid var(--border);
    background: var(--bg-secondary); flex-shrink: 0;
  }
  .panel-header h2 { font-size: 14px; font-weight: 600; }

  .panel-body {
    flex: 1; overflow-y: auto; padding: 16px 18px;
    display: flex; flex-direction: column; gap: 16px;
  }

  .form-card {
    background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: 8px; padding: 14px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .form-card h3 { font-size: 13px; font-weight: 500; color: var(--text-secondary); }

  .form-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 8px; }
  .field { display: flex; flex-direction: column; gap: 4px; }
  .field.full { grid-column: 1 / -1; }
  .field span { font-size: 11.5px; color: var(--text-muted); }

  .form-actions { display: flex; gap: 8px; justify-content: flex-end; }

  .input {
    background: var(--bg-secondary); border: 1px solid var(--border);
    color: var(--text-primary); padding: 7px 9px; border-radius: 5px;
    font-size: 12.5px; font-family: var(--font-mono); outline: none;
  }
  .input:focus { border-color: var(--border-focus); }
  select.input { font-family: var(--font-sans); }

  .btn {
    font-size: 11.5px; padding: 5px 12px; border-radius: 6px;
    border: 1px solid var(--border); background: var(--bg-tertiary);
    color: var(--text-secondary); cursor: pointer; font-weight: 500;
  }
  .btn:hover { background: var(--bg-hover); }
  .btn-primary { background: var(--accent, #7c5cff); color: #fff; border-color: var(--accent, #7c5cff); }
  .btn-primary:hover { filter: brightness(1.15); }

  .hint { font-size: 12px; color: var(--text-muted); text-align: center; padding: 20px; }

  .empty-state {
    text-align: center; padding: 40px 20px; color: var(--text-muted);
    display: flex; flex-direction: column; gap: 6px;
  }
  .empty-state .icon { font-size: 32px; }
  .empty-state p { font-size: 13px; color: var(--text-secondary); }
  .empty-state .sub { font-size: 11.5px; color: var(--text-muted); }

  .server-list { display: flex; flex-direction: column; gap: 8px; }

  .server-card {
    background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: 8px; padding: 12px 14px;
    display: flex; flex-direction: column; gap: 8px; transition: opacity 0.15s;
  }
  .server-card.disabled { opacity: 0.5; }

  .server-top { display: flex; align-items: flex-start; justify-content: space-between; gap: 10px; }

  .server-info { display: flex; flex-direction: column; gap: 3px; flex: 1; }
  .server-name-row { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .server-name { font-size: 13px; font-weight: 500; color: var(--text-primary); }
  .server-type-badge {
    font-size: 9.5px; padding: 1px 6px; border-radius: 8px;
    background: var(--bg-secondary); border: 1px solid var(--border);
    color: var(--text-muted); font-family: var(--font-mono); text-transform: uppercase;
  }
  .server-cat-badge {
    font-size: 9.5px; padding: 1px 6px; border-radius: 8px;
    background: color-mix(in srgb, var(--accent, #7c5cff) 10%, transparent);
    border: 1px solid color-mix(in srgb, var(--accent, #7c5cff) 30%, transparent);
    color: var(--accent, #7c5cff);
  }
  .server-desc { font-size: 11.5px; color: var(--text-muted); }
  .server-url { font-size: 11px; color: var(--teal); }
  .server-auth { font-size: 10px; color: var(--green); }

  .server-actions { display: flex; gap: 4px; align-items: center; flex-shrink: 0; }

  .toggle-btn {
    background: transparent; border: 1px solid var(--border);
    border-radius: 20px; padding: 3px 10px; font-size: 16px;
    cursor: pointer; transition: all 0.12s;
  }
  .toggle-btn.on  { color: var(--green); border-color: rgba(74,222,128,0.3); }
  .toggle-btn.off { color: var(--text-muted); }

  .action-btn {
    background: transparent; border: 1px solid var(--border);
    border-radius: 5px; padding: 4px 7px; font-size: 13px;
    cursor: pointer; transition: all 0.1s; color: var(--text-muted);
  }
  .action-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }
  .action-btn.danger:hover { background: var(--red-dim); color: var(--red); }
  .action-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .ping-result { font-size: 11.5px; font-family: var(--font-mono); padding: 4px 8px; border-radius: 4px; }
  .ping-result.ok  { background: var(--green-dim); color: var(--green); }
  .ping-result.err { background: var(--red-dim); color: var(--red); }

  .info-box {
    background: var(--bg-tertiary); border: 1px solid var(--border);
    border-radius: 8px; padding: 12px 14px;
    display: flex; flex-direction: column; gap: 4px;
  }
  .info-title { font-size: 12px; font-weight: 500; color: var(--text-secondary); }
  .info-box p { font-size: 11.5px; color: var(--text-muted); line-height: 1.5; }
</style>
