<script lang="ts">
  import { ollamaConfig, showSettings, models, modelsLoading, selectedModel, toast } from '$lib/stores';
  import { modelsApi } from '$lib/api';
  import { get } from 'svelte/store';

  let cfg = $state({ ...$ollamaConfig });
  let testing = $state(false);
  let testResult = $state<{ ok: boolean; message: string } | null>(null);

  async function testConnection() {
    testing = true;
    testResult = null;
    try {
      const modelList = await modelsApi.list(cfg.base_url);
      models.set(modelList);
      testResult = {
        ok: true,
        message: `✓ Connecté — ${modelList.length} modèle(s) disponible(s)`,
      };
    } catch (e) {
      testResult = {
        ok: false,
        message: `✗ ${e instanceof Error ? e.message : 'Connexion échouée'}`,
      };
    } finally {
      testing = false;
    }
  }

  function save() {
    ollamaConfig.set({ ...cfg });
    toast('success', 'Configuration sauvegardée');
    showSettings.set(false);
  }

  function close() { showSettings.set(false); }
</script>

<!-- Overlay -->
<div class="overlay" onclick={close} role="presentation"></div>

<div class="modal" role="dialog" aria-label="Paramètres Ollama">
  <div class="modal-header">
    <h2>⚙️ Paramètres Ollama</h2>
    <button class="close-btn" onclick={close}>✕</button>
  </div>

  <div class="modal-body">
    <!-- URL du serveur Ollama -->
    <label class="field">
      <span class="field-label">URL du serveur Ollama</span>
      <input
        type="url"
        bind:value={cfg.base_url}
        placeholder="http://localhost:11434"
        class="input"
      />
      <span class="field-hint">Serveur local ou distant (ex: http://192.168.1.100:11434)</span>
    </label>

    <!-- Mode API -->
    <label class="field">
      <span class="field-label">Mode API</span>
      <select bind:value={cfg.api_mode} class="input">
        <option value="openai">OpenAI (/v1/chat/completions) — recommandé</option>
        <option value="anthropic">Anthropic (/v1/messages) — v0.14.0+</option>
      </select>
      <span class="field-hint">Le mode Anthropic permet d'utiliser Claude Code avec Ollama</span>
    </label>

    <!-- Modèle par défaut -->
    <label class="field">
      <span class="field-label">Modèle par défaut</span>
      <select bind:value={cfg.default_model} class="input">
        {#each $models as m}
          <option value={m.name}>{m.name}</option>
        {/each}
        {#if $models.length === 0}
          <option value={cfg.default_model}>{cfg.default_model}</option>
        {/if}
      </select>
    </label>

    <!-- Test de connexion -->
    <div class="test-section">
      <button class="btn btn-ghost" onclick={testConnection} disabled={testing}>
        {testing ? '⏳ Test en cours…' : '🔍 Tester la connexion'}
      </button>
      {#if testResult}
        <p class="test-result {testResult.ok ? 'ok' : 'err'}">{testResult.message}</p>
      {/if}
    </div>

    <!-- Modèles recommandés -->
    <div class="info-box">
      <p class="info-title">🎯 Modèles recommandés pour le code</p>
      <ul>
        <li><code>qwen3-coder</code> — 30B, excellent en tool calling (24 Go VRAM)</li>
        <li><code>qwen2.5-coder:32b</code> — très bon équilibre qualité/vitesse</li>
        <li><code>glm-4.7</code> — performant, léger (16 Go VRAM)</li>
        <li><code>deepseek-coder-v2</code> — open source, très bon en Rust/Python</li>
      </ul>
    </div>
  </div>

  <div class="modal-footer">
    <button class="btn btn-ghost" onclick={close}>Annuler</button>
    <button class="btn btn-primary" onclick={save}>💾 Sauvegarder</button>
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
    width: 520px;
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

  .modal-body {
    padding: 20px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
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

  .modal-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 20px;
    border-top: 1px solid var(--border);
  }
</style>
