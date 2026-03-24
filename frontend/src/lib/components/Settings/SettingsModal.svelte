<script lang="ts">
  import { ollamaConfig, showSettings, models, modelsLoading, selectedModel, toast, appSettings, debugMode } from '$lib/stores';
  import { settingsApi, modelsApi, debugApi } from '$lib/api';
  import type { AppSetting, ModelInfo, DebugEntry } from '$lib/api';
  import { get } from 'svelte/store';

  type TabId = 'ollama' | 'security' | 'documents' | 'sessions' | 'mcp' | 'debug';

  const TABS: { id: TabId; label: string; icon: string }[] = [
    { id: 'ollama',    label: 'Ollama',     icon: '🤖' },
    { id: 'security',  label: 'Sécurité',   icon: '🔒' },
    { id: 'documents', label: 'Documents',  icon: '📚' },
    { id: 'sessions',  label: 'Sessions',   icon: '💬' },
    { id: 'mcp',       label: 'MCP',        icon: '🔌' },
    { id: 'debug',     label: 'Debug',      icon: '🐛' },
  ];

  let activeTab = $state<TabId>('ollama');
  let saving = $state(false);

  // Local state from DB settings
  let localSettings = $state<Record<string, unknown>>({});
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);
  let testModels = $state<ModelInfo[]>([]);
  let debugLogs = $state<DebugEntry[]>([]);
  let debugLoading = $state(false);
  let debugFilter = $state<string>('');
  let debugLevelFilter = $state<string>('');

  // Initialize from appSettings store
  const currentSettings = get(appSettings);
  for (const s of currentSettings) {
    localSettings[s.key] = s.value;
  }

  // Ensure defaults
  if (!localSettings['ollama_base_url']) localSettings['ollama_base_url'] = 'http://localhost:11434';
  if (!localSettings['ollama_api_mode']) localSettings['ollama_api_mode'] = 'openai';
  if (!localSettings['ollama_default_model']) localSettings['ollama_default_model'] = 'qwen3-coder';

  function getStr(key: string, fallback = ''): string {
    return (localSettings[key] as string) ?? fallback;
  }
  function getNum(key: string, fallback = 0): number {
    const v = localSettings[key];
    return typeof v === 'number' ? v : fallback;
  }

  async function testConnection() {
    testing = true;
    testResult = null;
    testModels = [];
    try {
      const result = await settingsApi.testConnection(getStr('ollama_base_url'));
      testModels = result.models;
      models.set(result.models);
      testResult = {
        ok: true,
        message: `Connecté — ${result.model_count} modèle(s) disponible(s)`,
      };
    } catch (e) {
      testResult = {
        ok: false,
        message: `${e instanceof Error ? e.message : 'Connexion échouée'}`,
      };
    } finally {
      testing = false;
    }
  }

  async function save() {
    saving = true;
    try {
      const updated = await settingsApi.bulkUpdate(localSettings);
      appSettings.set(updated.length > 0 ? updated : get(appSettings));

      // Sync ollamaConfig store
      ollamaConfig.set({
        base_url: getStr('ollama_base_url', 'http://localhost:11434'),
        api_mode: getStr('ollama_api_mode', 'openai'),
        default_model: getStr('ollama_default_model', 'qwen3-coder'),
      });
      selectedModel.set(getStr('ollama_default_model', 'qwen3-coder'));

      // Reload all settings from DB to ensure consistency
      const allSettings = await settingsApi.list();
      appSettings.set(allSettings);

      toast('success', 'Paramètres sauvegardés en base de données');
      showSettings.set(false);
    } catch (e) {
      toast('error', `Erreur: ${e instanceof Error ? e.message : e}`);
    } finally {
      saving = false;
    }
  }

  async function loadDebugLogs() {
    debugLoading = true;
    try {
      const result = await debugApi.getLogs(
        200,
        debugFilter || undefined,
        debugLevelFilter || undefined,
      );
      debugLogs = result.entries;
    } catch (e) {
      toast('error', `Erreur chargement logs: ${e instanceof Error ? e.message : e}`);
    } finally {
      debugLoading = false;
    }
  }

  async function clearDebugLogs() {
    try {
      await debugApi.clear();
      debugLogs = [];
      toast('success', 'Logs debug vidés');
    } catch (e) {
      toast('error', `Erreur: ${e instanceof Error ? e.message : e}`);
    }
  }

  function formatTs(ts: number): string {
    return new Date(ts * 1000).toLocaleTimeString('fr-FR', { hour12: false, fractionalSecondDigits: 1 });
  }

  function close() { showSettings.set(false); }
</script>

<!-- Overlay -->
<div class="overlay" onclick={close} role="presentation"></div>

<div class="modal" role="dialog" aria-label="Paramètres">
  <div class="modal-header">
    <h2>Paramètres</h2>
    <button class="close-btn" onclick={close}>✕</button>
  </div>

  <!-- Tabs -->
  <div class="tabs">
    {#each TABS as tab}
      <button
        class="tab"
        class:active={activeTab === tab.id}
        onclick={() => activeTab = tab.id}
      >
        <span class="tab-icon">{tab.icon}</span>
        <span class="tab-label">{tab.label}</span>
      </button>
    {/each}
  </div>

  <div class="modal-body">
    <!-- ═══════════════════════════ OLLAMA ═══════════════════════════ -->
    {#if activeTab === 'ollama'}
      <div class="section-title">Connexion au serveur Ollama</div>

      <label class="field">
        <span class="field-label">URL du serveur</span>
        <input
          type="url"
          value={getStr('ollama_base_url', 'http://localhost:11434')}
          oninput={(e) => localSettings['ollama_base_url'] = e.currentTarget.value}
          placeholder="http://localhost:11434"
          class="input"
        />
        <span class="field-hint">Adresse complète (ex: http://192.168.1.100:11434)</span>
      </label>

      <!-- Test de connexion -->
      <div class="test-section">
        <button class="btn btn-ghost" onclick={testConnection} disabled={testing}>
          {testing ? 'Test en cours…' : 'Tester la connexion'}
        </button>
        {#if testResult}
          <p class="test-result {testResult.ok ? 'ok' : 'err'}">
            {testResult.ok ? '✓' : '✗'} {testResult.message}
          </p>
        {/if}
      </div>

      <label class="field">
        <span class="field-label">Mode API</span>
        <select
          value={getStr('ollama_api_mode', 'openai')}
          onchange={(e) => localSettings['ollama_api_mode'] = e.currentTarget.value}
          class="input"
        >
          <option value="openai">OpenAI (/v1/chat/completions) — recommandé</option>
          <option value="anthropic">Anthropic (/v1/messages) — v0.14.0+</option>
        </select>
        <span class="field-hint">Le mode Anthropic permet d'utiliser Claude Code avec Ollama</span>
      </label>

      <label class="field">
        <span class="field-label">Modèle par défaut</span>
        <select
          value={getStr('ollama_default_model', 'qwen3-coder')}
          onchange={(e) => localSettings['ollama_default_model'] = e.currentTarget.value}
          class="input"
        >
          {#each $models as m}
            <option value={m.name}>{m.name}{m.size ? ` (${(m.size / 1e9).toFixed(1)} Go)` : ''}</option>
          {/each}
          {#if $models.length === 0}
            <option value={getStr('ollama_default_model', 'qwen3-coder')}>
              {getStr('ollama_default_model', 'qwen3-coder')}
            </option>
          {/if}
        </select>
        <span class="field-hint">Cliquez "Tester la connexion" pour charger les modèles disponibles</span>
      </label>

      <label class="field">
        <span class="field-label">Timeout (secondes)</span>
        <input
          type="number"
          value={getNum('ollama_timeout', 300)}
          oninput={(e) => localSettings['ollama_timeout'] = parseInt(e.currentTarget.value) || 300}
          min="10"
          max="3600"
          class="input"
        />
        <span class="field-hint">Temps maximum d'attente pour les requêtes Ollama</span>
      </label>

      <div class="info-box">
        <p class="info-title">Modèles recommandés pour le code</p>
        <ul>
          <li><code>qwen3-coder</code> — 30B, excellent en tool calling (24 Go VRAM)</li>
          <li><code>qwen2.5-coder:32b</code> — très bon équilibre qualité/vitesse</li>
          <li><code>glm-4.7</code> — performant, léger (16 Go VRAM)</li>
          <li><code>deepseek-coder-v2</code> — open source, très bon en Rust/Python</li>
        </ul>
      </div>

    <!-- ═══════════════════════════ SÉCURITÉ ═══════════════════════════ -->
    {:else if activeTab === 'security'}
      <div class="section-title">Sécurité et limites</div>

      <label class="field">
        <span class="field-label">Origines CORS autorisées</span>
        <input
          type="text"
          value={Array.isArray(localSettings['cors_origins']) ? (localSettings['cors_origins'] as string[]).join(', ') : ''}
          oninput={(e) => localSettings['cors_origins'] = e.currentTarget.value.split(',').map((s: string) => s.trim()).filter(Boolean)}
          class="input"
          placeholder="http://localhost:5173, http://localhost:3000"
        />
        <span class="field-hint">Séparées par des virgules (prise en compte au redémarrage)</span>
      </label>

      <label class="field">
        <span class="field-label">Timeout shell (secondes)</span>
        <input
          type="number"
          value={getNum('shell_timeout', 30)}
          oninput={(e) => localSettings['shell_timeout'] = parseInt(e.currentTarget.value) || 30}
          min="5"
          max="300"
          class="input"
        />
        <span class="field-hint">Durée maximale d'exécution d'une commande shell</span>
      </label>

      <label class="field">
        <span class="field-label">Taille max sortie shell (octets)</span>
        <input
          type="number"
          value={getNum('shell_max_output', 65536)}
          oninput={(e) => localSettings['shell_max_output'] = parseInt(e.currentTarget.value) || 65536}
          min="1024"
          class="input"
        />
        <span class="field-hint">64 Ko par défaut</span>
      </label>

      <label class="field">
        <span class="field-label">Taille max fichier (octets)</span>
        <input
          type="number"
          value={getNum('max_file_size', 10485760)}
          oninput={(e) => localSettings['max_file_size'] = parseInt(e.currentTarget.value) || 10485760}
          min="1024"
          class="input"
        />
        <span class="field-hint">10 Mo par défaut ({(getNum('max_file_size', 10485760) / 1024 / 1024).toFixed(1)} Mo)</span>
      </label>

    <!-- ═══════════════════════════ DOCUMENTS ═══════════════════════════ -->
    {:else if activeTab === 'documents'}
      <div class="section-title">Documents et RAG</div>

      <label class="field">
        <span class="field-label">Taille des chunks (tokens)</span>
        <input
          type="number"
          value={getNum('chunk_size', 1500)}
          oninput={(e) => localSettings['chunk_size'] = parseInt(e.currentTarget.value) || 1500}
          min="100"
          max="10000"
          class="input"
        />
        <span class="field-hint">Taille approximative en tokens de chaque chunk de document</span>
      </label>

      <label class="field">
        <span class="field-label">Chevauchement des chunks</span>
        <input
          type="number"
          value={getNum('chunk_overlap', 150)}
          oninput={(e) => localSettings['chunk_overlap'] = parseInt(e.currentTarget.value) || 150}
          min="0"
          max="1000"
          class="input"
        />
        <span class="field-hint">Nombre de tokens de chevauchement entre chunks consécutifs</span>
      </label>

      <label class="field">
        <span class="field-label">Chunks max en contexte</span>
        <input
          type="number"
          value={getNum('max_chunks_context', 8)}
          oninput={(e) => localSettings['max_chunks_context'] = parseInt(e.currentTarget.value) || 8}
          min="1"
          max="50"
          class="input"
        />
        <span class="field-hint">Nombre maximum de chunks injectés dans le contexte de chat</span>
      </label>

    <!-- ═══════════════════════════ SESSIONS ═══════════════════════════ -->
    {:else if activeTab === 'sessions'}
      <div class="section-title">Sessions de chat</div>

      <label class="field">
        <span class="field-label">Nombre max de sessions</span>
        <input
          type="number"
          value={getNum('max_sessions', 100)}
          oninput={(e) => localSettings['max_sessions'] = parseInt(e.currentTarget.value) || 100}
          min="1"
          max="10000"
          class="input"
        />
        <span class="field-hint">Nombre maximum de sessions de chat conservées</span>
      </label>

      <label class="field">
        <span class="field-label">Messages max par session</span>
        <input
          type="number"
          value={getNum('max_messages_per_session', 500)}
          oninput={(e) => localSettings['max_messages_per_session'] = parseInt(e.currentTarget.value) || 500}
          min="10"
          max="50000"
          class="input"
        />
        <span class="field-hint">Nombre maximum de messages conservés par session</span>
      </label>

    <!-- ═══════════════════════════ MCP ═══════════════════════════ -->
    {:else if activeTab === 'mcp'}
      <div class="section-title">Model Context Protocol</div>

      <label class="field">
        <span class="field-label">Timeout MCP (secondes)</span>
        <input
          type="number"
          value={getNum('mcp_timeout', 30)}
          oninput={(e) => localSettings['mcp_timeout'] = parseInt(e.currentTarget.value) || 30}
          min="5"
          max="300"
          class="input"
        />
        <span class="field-hint">Temps maximum d'attente pour les appels aux serveurs MCP</span>
      </label>

      <div class="info-box">
        <p class="info-title">Serveurs MCP</p>
        <p class="info-text">La gestion des serveurs MCP (ajout, suppression, activation) se fait depuis l'onglet MCP de l'interface principale.</p>
      </div>

    <!-- ═══════════════════════════ DEBUG ═══════════════════════════ -->
    {:else if activeTab === 'debug'}
      <div class="section-title">Mode debug</div>

      <label class="field debug-toggle-field">
        <span class="field-label">Activer le mode debug</span>
        <div class="toggle-row">
          <button
            class="toggle-btn {localSettings['debug_mode'] ? 'on' : 'off'}"
            onclick={() => localSettings['debug_mode'] = !localSettings['debug_mode']}
          >
            {localSettings['debug_mode'] ? 'ACTIF' : 'INACTIF'}
          </button>
          <span class="field-hint">
            {localSettings['debug_mode']
              ? 'Toutes les actions sont tracées (requêtes HTTP, WebSocket, imports, Ollama)'
              : 'Seules les erreurs sont tracées'}
          </span>
        </div>
      </label>

      <div class="debug-toolbar">
        <select class="input debug-select" bind:value={debugFilter}>
          <option value="">Toutes catégories</option>
          <option value="http">HTTP</option>
          <option value="ws">WebSocket</option>
          <option value="import">Import docs</option>
          <option value="skill">Skills</option>
          <option value="ollama">Ollama</option>
          <option value="chat">Chat</option>
          <option value="error">Erreurs</option>
          <option value="debug">Debug</option>
        </select>
        <select class="input debug-select" bind:value={debugLevelFilter}>
          <option value="">Tous niveaux</option>
          <option value="ERROR">ERROR</option>
          <option value="WARN">WARN</option>
          <option value="INFO">INFO</option>
          <option value="DEBUG">DEBUG</option>
        </select>
        <button class="btn btn-ghost" onclick={loadDebugLogs} disabled={debugLoading}>
          {debugLoading ? 'Chargement…' : 'Charger logs'}
        </button>
        <button class="btn btn-ghost" onclick={clearDebugLogs}>Vider</button>
      </div>

      <div class="debug-log-container">
        {#if debugLogs.length === 0}
          <p class="debug-empty">Aucun log.{localSettings['debug_mode'] ? ' Cliquez "Charger logs" pour rafraîchir.' : ' Activez le mode debug puis sauvegardez.'}</p>
        {:else}
          {#each debugLogs as entry}
            <div class="debug-entry level-{entry.level.toLowerCase()}">
              <span class="debug-ts">{formatTs(entry.timestamp)}</span>
              <span class="debug-level">{entry.level}</span>
              <span class="debug-cat">{entry.category}</span>
              <span class="debug-msg">{entry.message}</span>
              {#if Object.keys(entry.details).length > 0}
                <details class="debug-details">
                  <summary>détails</summary>
                  <pre>{JSON.stringify(entry.details, null, 2)}</pre>
                </details>
              {/if}
            </div>
          {/each}
        {/if}
      </div>
    {/if}
  </div>

  <div class="modal-footer">
    <span class="db-badge">SQLite</span>
    <div class="footer-actions">
      <button class="btn btn-ghost" onclick={close}>Annuler</button>
      <button class="btn btn-primary" onclick={save} disabled={saving}>
        {saving ? 'Sauvegarde…' : 'Sauvegarder'}
      </button>
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.6);
    z-index: 100;
    backdrop-filter: blur(2px);
  }

  .modal {
    position: fixed;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%);
    z-index: 101;
    width: 620px;
    max-width: 95vw;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    max-height: 90vh;
    overflow: hidden;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 16px 20px;
    border-bottom: 1px solid var(--border);
  }

  .modal-header h2 {
    font-size: 15px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .close-btn {
    background: transparent;
    border: none;
    color: var(--text-muted);
    font-size: 16px;
    cursor: pointer;
    padding: 2px 6px;
    border-radius: 4px;
  }
  .close-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }

  /* ── Tabs ─────────────────────────────────────────── */
  .tabs {
    display: flex;
    border-bottom: 1px solid var(--border);
    padding: 0 12px;
    gap: 2px;
    overflow-x: auto;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 10px 14px;
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-muted);
    background: transparent;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    white-space: nowrap;
    transition: color 0.15s, border-color 0.15s;
  }
  .tab:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }
  .tab.active {
    color: var(--accent, #7c5cff);
    border-bottom-color: var(--accent, #7c5cff);
  }

  .tab-icon { font-size: 14px; }
  .tab-label { font-size: 12px; }

  /* ── Body ─────────────────────────────────────────── */
  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 14px;
    min-height: 300px;
  }

  .section-title {
    font-size: 13px;
    font-weight: 600;
    color: var(--text-primary);
    padding-bottom: 4px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 2px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label {
    font-size: 12.5px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .input {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-primary);
    padding: 8px 10px;
    border-radius: var(--input-radius);
    font-size: 13px;
    font-family: var(--font-mono);
    outline: none;
    transition: border-color 0.15s;
  }
  .input:focus { border-color: var(--border-focus); }

  .field-hint {
    font-size: 11px;
    color: var(--text-muted);
  }

  .test-section {
    display: flex;
    align-items: center;
    gap: 10px;
    flex-wrap: wrap;
  }

  .test-result {
    font-size: 12.5px;
    font-family: var(--font-mono);
  }
  .test-result.ok  { color: var(--green); }
  .test-result.err { color: var(--red); }

  .info-box {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: var(--panel-radius);
    padding: 12px 14px;
  }

  .info-title {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
    margin-bottom: 8px;
  }

  .info-text {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.5;
  }

  .info-box ul {
    list-style: none;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .info-box li {
    font-size: 12px;
    color: var(--text-muted);
  }

  /* ── Footer ───────────────────────────────────────── */
  .modal-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }

  .footer-actions {
    display: flex;
    gap: 8px;
  }

  /* ── Debug ─────────────────────────────────────────── */
  .debug-toggle-field { gap: 6px; }

  .toggle-row {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .toggle-btn {
    padding: 4px 14px;
    border-radius: 20px;
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.5px;
    border: 1px solid var(--border);
    cursor: pointer;
    transition: all 0.15s;
  }
  .toggle-btn.on {
    background: rgba(74, 222, 128, 0.2);
    color: #4ade80;
    border-color: #4ade80;
  }
  .toggle-btn.off {
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }

  .debug-toolbar {
    display: flex;
    gap: 6px;
    align-items: center;
    flex-wrap: wrap;
  }

  .debug-select {
    flex: 0 0 auto;
    width: auto;
    min-width: 130px;
    font-size: 11px;
    padding: 5px 8px;
  }

  .debug-log-container {
    background: #0d0d0f;
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 8px;
    max-height: 280px;
    overflow-y: auto;
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
  }

  .debug-empty {
    color: var(--text-muted);
    text-align: center;
    padding: 20px;
    font-size: 12px;
  }

  .debug-entry {
    display: flex;
    gap: 6px;
    align-items: baseline;
    padding: 2px 0;
    border-bottom: 1px solid rgba(255,255,255,0.03);
    flex-wrap: wrap;
  }

  .debug-ts {
    color: #5a5a72;
    flex-shrink: 0;
  }

  .debug-level {
    font-weight: 700;
    flex-shrink: 0;
    min-width: 42px;
  }
  .level-error .debug-level { color: #f87171; }
  .level-warn .debug-level  { color: #fbbf24; }
  .level-info .debug-level  { color: #4ade80; }
  .level-debug .debug-level { color: #7c6ff7; }

  .debug-cat {
    color: #2dd4bf;
    flex-shrink: 0;
    min-width: 50px;
  }

  .debug-msg {
    color: #e8e8f0;
    word-break: break-word;
  }
  .level-error .debug-msg { color: #fca5a5; }
  .level-warn .debug-msg  { color: #fde68a; }

  .debug-details {
    width: 100%;
    margin: 2px 0 4px 0;
  }

  .debug-details summary {
    color: var(--text-muted);
    cursor: pointer;
    font-size: 10px;
  }

  .debug-details pre {
    color: #a5b4fc;
    font-size: 10px;
    white-space: pre-wrap;
    margin: 4px 0 0 0;
    padding: 4px 8px;
    background: rgba(124, 111, 247, 0.05);
    border-radius: 4px;
  }

  .db-badge {
    font-size: 10px;
    font-weight: 600;
    color: var(--text-muted);
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 4px;
    padding: 2px 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }
</style>
