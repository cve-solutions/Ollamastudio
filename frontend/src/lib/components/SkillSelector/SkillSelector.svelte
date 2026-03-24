<script lang="ts">
  import { skills, activeSkill, showSkillSelector, toast } from '$lib/stores';
  import { skillsApi } from '$lib/api';
  import type { Skill } from '$lib/api';

  let importing = $state(false);
  let fileInput: HTMLInputElement;

  function selectSkill(skill: Skill | null) {
    activeSkill.set(skill);
    showSkillSelector.set(false);
    toast('info', skill ? `Skill "${skill.name}" activée` : 'Skill désactivée');
  }

  function close() { showSkillSelector.set(false); }

  function triggerImport() {
    fileInput?.click();
  }

  async function handleFileImport(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;

    importing = true;
    try {
      const result = await skillsApi.importJson(file);
      // Recharge la liste des skills
      const updatedSkills = await skillsApi.list();
      skills.set(updatedSkills);

      if (result.imported > 0) {
        toast('success', `${result.imported} compétence(s) importée(s)`);
      }
      if (result.skipped > 0) {
        toast('warning', `${result.skipped} entrée(s) ignorée(s) (pas de prompt)`);
      }
      if (result.errors > 0) {
        toast('error', `${result.errors} erreur(s) lors de l'import`);
      }
    } catch (e) {
      toast('error', `Erreur d'import: ${e instanceof Error ? e.message : e}`);
    } finally {
      importing = false;
      input.value = '';
    }
  }
</script>

<div class="overlay" onclick={close} role="presentation"></div>

<div class="panel" role="dialog" aria-label="Sélection de skill">
  <div class="panel-header">
    <h2>Compétences (Skills)</h2>
    <div class="header-actions">
      <button
        class="import-btn"
        onclick={triggerImport}
        disabled={importing}
        title="Importer un fichier JSON de compétences"
      >
        {importing ? 'Import…' : 'Importer JSON'}
      </button>
      <button class="close-btn" onclick={close}>✕</button>
    </div>
    <input
      type="file"
      accept=".json"
      style="display:none"
      bind:this={fileInput}
      onchange={handleFileImport}
    />
  </div>

  <div class="skills-grid">
    <!-- Option "aucune skill" -->
    <button
      class="skill-card none-card {$activeSkill === null ? 'active' : ''}"
      onclick={() => selectSkill(null)}
    >
      <span class="skill-icon">🌐</span>
      <div class="skill-info">
        <span class="skill-name">Mode par défaut</span>
        <span class="skill-desc">Tous les outils activés, pas de system prompt spécifique</span>
      </div>
    </button>

    {#each $skills as skill (skill.id)}
      <button
        class="skill-card {$activeSkill?.id === skill.id ? 'active' : ''}"
        style="--skill-color: {skill.color || '#7c6ff7'}"
        onclick={() => selectSkill(skill)}
      >
        <span class="skill-icon">{skill.icon}</span>
        <div class="skill-info">
          <span class="skill-name">{skill.name}</span>
          <span class="skill-desc">{skill.description}</span>
          <div class="skill-meta">
            {#if skill.enabled_tools}
              <span class="meta-badge">{skill.enabled_tools.length} outils</span>
            {:else}
              <span class="meta-badge">Tous les outils</span>
            {/if}
            <span class="meta-badge">{skill.temperature}</span>
            <span class="meta-badge">{skill.max_tokens}</span>
          </div>
        </div>
        {#if $activeSkill?.id === skill.id}
          <span class="active-check">✓</span>
        {/if}
      </button>
    {/each}
  </div>

  <div class="panel-footer">
    <span class="hint">Formats acceptés : OllamaStudio, Claude, ou tableau JSON avec champ "name" + "system_prompt"/"content"/"instructions"</span>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: rgba(0,0,0,0.5);
    z-index: 100;
  }

  .panel {
    position: fixed;
    top: 46px;
    left: 50%;
    transform: translateX(-50%);
    width: 600px;
    max-width: 95vw;
    max-height: calc(100vh - 60px);
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    z-index: 101;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: fade-in 0.15s ease-out;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .panel-header h2 {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .import-btn {
    font-size: 11.5px;
    padding: 5px 12px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-secondary);
    cursor: pointer;
    font-weight: 500;
    transition: all 0.12s;
  }
  .import-btn:hover {
    background: var(--bg-hover);
    border-color: var(--accent, #7c5cff);
    color: var(--accent, #7c5cff);
  }
  .import-btn:disabled {
    opacity: 0.5;
    cursor: wait;
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

  .skills-grid {
    padding: 12px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .skill-card {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 14px;
    border-radius: 8px;
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    cursor: pointer;
    text-align: left;
    transition: all 0.12s;
    position: relative;
  }
  .skill-card:hover {
    background: var(--bg-hover);
    border-color: var(--skill-color, var(--accent-border));
  }
  .skill-card.active {
    background: color-mix(in srgb, var(--skill-color, var(--accent)) 10%, transparent);
    border-color: var(--skill-color, var(--accent));
  }
  .none-card { --skill-color: var(--text-muted); }

  .skill-icon { font-size: 22px; flex-shrink: 0; margin-top: 2px; }

  .skill-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
    flex: 1;
    min-width: 0;
  }

  .skill-name {
    font-size: 13.5px;
    font-weight: 500;
    color: var(--text-primary);
  }

  .skill-desc {
    font-size: 12px;
    color: var(--text-muted);
    line-height: 1.4;
  }

  .skill-meta {
    display: flex;
    gap: 5px;
    flex-wrap: wrap;
    margin-top: 3px;
  }

  .meta-badge {
    font-size: 10.5px;
    padding: 1px 6px;
    border-radius: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  .active-check {
    position: absolute;
    top: 10px;
    right: 12px;
    font-size: 14px;
    color: var(--skill-color, var(--accent));
    font-weight: 700;
  }

  .panel-footer {
    padding: 10px 18px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .hint {
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.4;
  }
</style>
