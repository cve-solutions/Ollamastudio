<script lang="ts">
  import { skills, activeSkill, showSkillSelector, toast } from '$lib/stores';
  import { skillsApi } from '$lib/api';
  import type { Skill } from '$lib/api';

  const AVAILABLE_TOOLS = [
    'read_file', 'write_file', 'patch_file', 'list_files',
    'run_command', 'grep_files', 'git_status', 'search_documents',
  ];

  const DEFAULT_IDS = ['default', 'code-review', 'devops', 'rust-expert', 'security-audit'];

  let importing = $state(false);
  let fileInput: HTMLInputElement;

  // ── Mode formulaire (create / edit) ─────────────────────────────
  type FormMode = 'list' | 'create' | 'edit';
  let mode: FormMode = $state('list');
  let saving = $state(false);

  let form = $state<{
    id: string; name: string; description: string; icon: string;
    system_prompt: string; enabled_tools: string[] | null;
    temperature: number; max_tokens: number; color: string;
  }>({
    id: '', name: '', description: '', icon: '🤖',
    system_prompt: '', enabled_tools: null,
    temperature: 0.7, max_tokens: 4096, color: '#6366f1',
  });

  let allToolsEnabled = $state(true);

  function resetForm() {
    form = {
      id: '', name: '', description: '', icon: '🤖',
      system_prompt: '', enabled_tools: null,
      temperature: 0.7, max_tokens: 4096, color: '#6366f1',
    };
    allToolsEnabled = true;
  }

  function openCreate() {
    resetForm();
    mode = 'create';
  }

  function openEdit(skill: Skill) {
    form = { ...skill };
    allToolsEnabled = skill.enabled_tools === null;
    mode = 'edit';
  }

  function cancelForm() {
    mode = 'list';
  }

  function slugify(text: string): string {
    return text.toLowerCase().trim()
      .replace(/[^\w\s-]/g, '')
      .replace(/[\s_]+/g, '-')
      .slice(0, 64) || 'custom-skill';
  }

  function toggleTool(tool: string) {
    if (!form.enabled_tools) form.enabled_tools = [...AVAILABLE_TOOLS];
    const idx = form.enabled_tools.indexOf(tool);
    if (idx >= 0) {
      form.enabled_tools = form.enabled_tools.filter(t => t !== tool);
    } else {
      form.enabled_tools = [...form.enabled_tools, tool];
    }
  }

  function toggleAllTools() {
    allToolsEnabled = !allToolsEnabled;
    form.enabled_tools = allToolsEnabled ? null : [...AVAILABLE_TOOLS];
  }

  async function saveSkill() {
    if (!form.name.trim() || !form.system_prompt.trim()) {
      toast('error', 'Le nom et le system prompt sont obligatoires');
      return;
    }

    saving = true;
    try {
      const payload = {
        ...form,
        id: mode === 'create' ? slugify(form.name) : form.id,
        enabled_tools: allToolsEnabled ? null : form.enabled_tools,
      };

      if (mode === 'create') {
        await skillsApi.create(payload as Skill);
        toast('success', `Skill "${form.name}" créée`);
      } else {
        await skillsApi.update(form.id, payload as Skill);
        toast('success', `Skill "${form.name}" mise à jour`);
      }

      const updated = await skillsApi.list();
      skills.set(updated);
      mode = 'list';
    } catch (e) {
      toast('error', `Erreur: ${e instanceof Error ? e.message : e}`);
    } finally {
      saving = false;
    }
  }

  async function deleteSkill(skill: Skill) {
    if (DEFAULT_IDS.includes(skill.id)) {
      toast('error', 'Impossible de supprimer une skill par défaut');
      return;
    }
    try {
      await skillsApi.delete(skill.id);
      const updated = await skillsApi.list();
      skills.set(updated);
      if ($activeSkill?.id === skill.id) activeSkill.set(null);
      toast('success', `Skill "${skill.name}" supprimée`);
    } catch (e) {
      toast('error', `Erreur suppression: ${e instanceof Error ? e.message : e}`);
    }
  }

  // ── Sélection et import ─────────────────────────────────────────
  function selectSkill(skill: Skill | null) {
    activeSkill.set(skill);
    showSkillSelector.set(false);
    toast('info', skill ? `Skill "${skill.name}" activée` : 'Skill désactivée');
  }

  function close() { showSkillSelector.set(false); }

  function triggerImport() { fileInput?.click(); }

  async function handleFileImport(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    importing = true;
    try {
      const result = await skillsApi.importJson(file);
      const updatedSkills = await skillsApi.list();
      skills.set(updatedSkills);
      if (result.imported > 0) toast('success', `${result.imported} compétence(s) importée(s)`);
      if (result.skipped > 0) toast('warning', `${result.skipped} entrée(s) ignorée(s)`);
      if (result.errors > 0) toast('error', `${result.errors} erreur(s) lors de l'import`);
    } catch (e) {
      toast('error', `Erreur d'import: ${e instanceof Error ? e.message : e}`);
    } finally {
      importing = false;
      input.value = '';
    }
  }
</script>

<div class="overlay" onclick={close} role="presentation"></div>

<div class="panel" role="dialog" aria-label="Gestion des skills">
  <div class="panel-header">
    <h2>{mode === 'list' ? 'Compétences (Skills)' : mode === 'create' ? 'Nouvelle compétence' : `Modifier : ${form.name}`}</h2>
    <div class="header-actions">
      {#if mode === 'list'}
        <button class="action-btn create-btn" onclick={openCreate}>+ Créer</button>
        <button class="action-btn" onclick={triggerImport} disabled={importing}>
          {importing ? 'Import…' : 'Importer JSON'}
        </button>
      {/if}
      <button class="close-btn" onclick={mode === 'list' ? close : cancelForm}>
        {mode === 'list' ? '✕' : '← Retour'}
      </button>
    </div>
    <input type="file" accept=".json" style="display:none" bind:this={fileInput} onchange={handleFileImport} />
  </div>

  {#if mode === 'list'}
    <!-- ── LISTE DES SKILLS ─────────────────────────────────────── -->
    <div class="skills-grid">
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
        <div class="skill-row">
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
                <span class="meta-badge">T:{skill.temperature}</span>
                <span class="meta-badge">{skill.max_tokens} tok</span>
              </div>
            </div>
            {#if $activeSkill?.id === skill.id}
              <span class="active-check">✓</span>
            {/if}
          </button>
          <div class="card-actions">
            <button class="card-btn edit-btn" title="Modifier" onclick={() => openEdit(skill)}>✎</button>
            {#if !DEFAULT_IDS.includes(skill.id)}
              <button class="card-btn delete-btn" title="Supprimer" onclick={() => deleteSkill(skill)}>🗑</button>
            {/if}
          </div>
        </div>
      {/each}
    </div>

    <div class="panel-footer">
      <span class="hint">Cliquez sur une skill pour l'activer. ✎ pour modifier, 🗑 pour supprimer.</span>
    </div>

  {:else}
    <!-- ── FORMULAIRE CREATE / EDIT ─────────────────────────────── -->
    <div class="form-scroll">
      <div class="form-grid">
        <div class="form-row inline">
          <label class="form-label">
            Icône
            <input type="text" class="form-input icon-input" bind:value={form.icon} maxlength="2" />
          </label>
          <label class="form-label" style="flex:1">
            Nom
            <input type="text" class="form-input" bind:value={form.name} placeholder="Ex: Python Expert" />
          </label>
          <label class="form-label">
            Couleur
            <input type="color" class="form-color" bind:value={form.color} />
          </label>
        </div>

        <label class="form-label">
          Description
          <input type="text" class="form-input" bind:value={form.description} placeholder="Courte description de la compétence" />
        </label>

        <label class="form-label">
          System Prompt
          <textarea class="form-textarea" bind:value={form.system_prompt} rows="6" placeholder="Instructions pour le modèle..."></textarea>
        </label>

        <div class="form-row inline">
          <label class="form-label" style="flex:1">
            Température
            <div class="range-row">
              <input type="range" min="0" max="2" step="0.1" bind:value={form.temperature} />
              <span class="range-value">{form.temperature}</span>
            </div>
          </label>
          <label class="form-label" style="flex:1">
            Max tokens
            <input type="number" class="form-input" bind:value={form.max_tokens} min="256" max="32768" step="256" />
          </label>
        </div>

        <div class="tools-section">
          <div class="tools-header">
            <span class="form-label-text">Outils activés</span>
            <button class="toggle-all-btn" onclick={toggleAllTools}>
              {allToolsEnabled ? 'Sélectionner individuellement' : 'Activer tous'}
            </button>
          </div>
          {#if !allToolsEnabled}
            <div class="tools-grid">
              {#each AVAILABLE_TOOLS as tool}
                <button
                  class="tool-chip {form.enabled_tools?.includes(tool) ? 'on' : 'off'}"
                  onclick={() => toggleTool(tool)}
                >
                  {tool}
                </button>
              {/each}
            </div>
          {:else}
            <span class="hint">Tous les outils sont activés</span>
          {/if}
        </div>
      </div>
    </div>

    <div class="panel-footer form-footer">
      <button class="action-btn" onclick={cancelForm}>Annuler</button>
      <button class="action-btn save-btn" onclick={saveSkill} disabled={saving}>
        {saving ? 'Enregistrement…' : mode === 'create' ? 'Créer la skill' : 'Enregistrer'}
      </button>
    </div>
  {/if}
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
    width: 680px;
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

  .action-btn {
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
  .action-btn:hover {
    background: var(--bg-hover);
    border-color: var(--accent, #7c5cff);
    color: var(--accent, #7c5cff);
  }
  .action-btn:disabled { opacity: 0.5; cursor: wait; }

  .create-btn {
    background: var(--accent, #7c5cff);
    color: #fff;
    border-color: var(--accent, #7c5cff);
  }
  .create-btn:hover {
    filter: brightness(1.15);
    color: #fff;
  }

  .save-btn {
    background: var(--accent, #7c5cff);
    color: #fff;
    border-color: var(--accent, #7c5cff);
  }
  .save-btn:hover { filter: brightness(1.15); color: #fff; }

  .close-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-size: 12px;
    cursor: pointer;
    padding: 4px 10px;
    border-radius: 6px;
  }
  .close-btn:hover { background: var(--bg-hover); }

  /* ── Liste ──────────────────────────────────────────────────── */
  .skills-grid {
    padding: 12px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .skill-row {
    display: flex;
    gap: 4px;
    align-items: stretch;
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
    flex: 1;
    min-width: 0;
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

  .card-actions {
    display: flex;
    flex-direction: column;
    gap: 2px;
    justify-content: center;
  }

  .card-btn {
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    border-radius: 6px;
    padding: 6px 8px;
    cursor: pointer;
    font-size: 13px;
    line-height: 1;
    transition: all 0.12s;
  }
  .edit-btn:hover { background: var(--bg-hover); border-color: var(--accent, #7c5cff); }
  .delete-btn:hover { background: #3a1c1c; border-color: #f87171; }

  /* ── Formulaire ─────────────────────────────────────────────── */
  .form-scroll {
    padding: 16px 18px;
    overflow-y: auto;
    flex: 1;
  }

  .form-grid {
    display: flex;
    flex-direction: column;
    gap: 14px;
  }

  .form-row.inline {
    display: flex;
    gap: 12px;
    align-items: flex-end;
  }

  .form-label {
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-label-text {
    font-size: 12px;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-input {
    padding: 7px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 13px;
    outline: none;
    transition: border-color 0.12s;
  }
  .form-input:focus { border-color: var(--accent, #7c5cff); }

  .icon-input { width: 48px; text-align: center; font-size: 18px; }

  .form-color {
    width: 36px;
    height: 36px;
    border: 1px solid var(--border);
    border-radius: 6px;
    cursor: pointer;
    background: transparent;
    padding: 2px;
  }

  .form-textarea {
    padding: 8px 10px;
    border-radius: 6px;
    border: 1px solid var(--border);
    background: var(--bg-primary);
    color: var(--text-primary);
    font-size: 12.5px;
    font-family: var(--font-mono);
    line-height: 1.5;
    resize: vertical;
    outline: none;
    transition: border-color 0.12s;
  }
  .form-textarea:focus { border-color: var(--accent, #7c5cff); }

  .range-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .range-row input[type="range"] { flex: 1; accent-color: var(--accent, #7c5cff); }
  .range-value {
    font-size: 12px;
    color: var(--text-muted);
    font-family: var(--font-mono);
    min-width: 28px;
  }

  .tools-section {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .tools-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .toggle-all-btn {
    font-size: 11px;
    padding: 3px 8px;
    border-radius: 4px;
    border: 1px solid var(--border);
    background: var(--bg-tertiary);
    color: var(--text-muted);
    cursor: pointer;
    transition: all 0.12s;
  }
  .toggle-all-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }

  .tools-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .tool-chip {
    font-size: 11px;
    padding: 4px 10px;
    border-radius: 12px;
    border: 1px solid var(--border);
    cursor: pointer;
    font-family: var(--font-mono);
    transition: all 0.12s;
  }
  .tool-chip.on {
    background: color-mix(in srgb, var(--accent, #7c5cff) 15%, transparent);
    border-color: var(--accent, #7c5cff);
    color: var(--accent, #7c5cff);
  }
  .tool-chip.off {
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }
  .tool-chip:hover { filter: brightness(1.1); }

  .panel-footer {
    padding: 10px 18px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .form-footer {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
  }

  .hint {
    font-size: 10.5px;
    color: var(--text-muted);
    line-height: 1.4;
  }
</style>
