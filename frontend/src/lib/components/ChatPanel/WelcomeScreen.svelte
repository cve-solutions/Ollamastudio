<script lang="ts">
  import { activeSkill, skills } from '$lib/stores';

  interface Props {
    onSend: (msg: string) => void;
  }
  let { onSend }: Props = $props();

  const suggestions = [
    { icon: '🦀', label: 'Créer un projet Rust', msg: 'Crée un projet Rust avec Cargo pour une CLI simple qui lit des fichiers JSON' },
    { icon: '🐍', label: 'Script Python FastAPI', msg: 'Écris un endpoint FastAPI en Python qui reçoit un fichier et retourne son hash SHA256' },
    { icon: '🐳', label: 'Optimiser un Dockerfile', msg: 'Optimise ce Dockerfile pour réduire la taille de l\'image et améliorer le cache des layers' },
    { icon: '🔍', label: 'Revue de code sécurité', msg: 'Analyse les fichiers du workspace et identifie les problèmes de sécurité potentiels' },
    { icon: '📝', label: 'Documenter le code', msg: 'Génère la documentation complète pour les fonctions Python du workspace' },
    { icon: '🌿', label: 'Statut Git', msg: 'Montre le statut git du workspace et explique les changements en attente' },
  ];
</script>

<div class="welcome">
  <div class="welcome-content">
    <div class="logo-big">◎</div>
    <h1>OllamaStudio</h1>
    <p class="subtitle">Interface Claude Code compatible — <strong>100% local</strong>, données sur votre infrastructure</p>

    {#if $activeSkill}
      <div class="active-skill-banner" style="border-color: {$activeSkill.color}; background: color-mix(in srgb, {$activeSkill.color} 8%, transparent)">
        <span>{$activeSkill.icon}</span>
        <div>
          <span class="skill-name">{$activeSkill.name}</span>
          <span class="skill-desc">{$activeSkill.description}</span>
        </div>
      </div>
    {/if}

    <div class="suggestions-grid">
      {#each suggestions as s}
        <button class="suggestion-card" onclick={() => onSend(s.msg)}>
          <span class="s-icon">{s.icon}</span>
          <span class="s-label">{s.label}</span>
        </button>
      {/each}
    </div>

    <div class="features">
      <span>📁 Éditeur Monaco</span>
      <span>⌨️ Terminal PTY</span>
      <span>⚙️ Tool calling</span>
      <span>📚 RAG Documents</span>
      <span>🔌 MCP Protocol</span>
      <span>🤖 Skills YAML</span>
    </div>
  </div>
</div>

<style>
  .welcome {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 20px;
    overflow-y: auto;
  }

  .welcome-content {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    max-width: 640px;
    width: 100%;
    text-align: center;
  }

  .logo-big {
    font-size: 48px;
    color: var(--accent);
    line-height: 1;
    animation: pulse-dot 3s ease-in-out infinite;
  }

  h1 {
    font-size: 26px;
    font-weight: 700;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .subtitle {
    font-size: 14px;
    color: var(--text-muted);
    line-height: 1.5;
  }
  .subtitle strong { color: var(--accent-light); }

  .active-skill-banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 16px;
    border: 1px solid;
    border-radius: 10px;
    font-size: 13px;
    width: 100%;
  }
  .active-skill-banner span:first-child { font-size: 20px; }
  .active-skill-banner > div { display: flex; flex-direction: column; gap: 2px; text-align: left; }
  .skill-name { font-weight: 500; color: var(--text-primary); }
  .skill-desc { font-size: 11.5px; color: var(--text-muted); }

  .suggestions-grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 8px;
    width: 100%;
  }

  .suggestion-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 12px;
    cursor: pointer;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    text-align: left;
    transition: all 0.12s;
  }
  .suggestion-card:hover {
    background: var(--bg-hover);
    border-color: var(--accent-border);
    transform: translateY(-1px);
  }

  .s-icon { font-size: 18px; }
  .s-label { font-size: 12px; color: var(--text-secondary); line-height: 1.3; }

  .features {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    justify-content: center;
  }

  .features span {
    font-size: 11px;
    padding: 3px 9px;
    border-radius: 12px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-muted);
  }

  @media (max-width: 500px) {
    .suggestions-grid { grid-template-columns: repeat(2, 1fr); }
  }
</style>
