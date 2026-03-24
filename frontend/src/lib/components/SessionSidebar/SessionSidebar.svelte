<script lang="ts">
  import { sessions, activeSessionId, messages, selectedModel, activeSkill, toast } from '$lib/stores';
  import { sessionsApi } from '$lib/api';
  import { get } from 'svelte/store';

  async function newSession() {
    const model = get(selectedModel);
    const skill = get(activeSkill);
    try {
      const s = await sessionsApi.create({ title: 'Nouvelle session', model, skill_id: skill?.id });
      sessions.update(list => [s, ...list]);
      activeSessionId.set(s.id);
      messages.set([]);
    } catch (e) {
      toast('error', `Erreur création session: ${e instanceof Error ? e.message : e}`);
    }
  }

  async function deleteSession(id: number, e: MouseEvent) {
    e.stopPropagation();
    try {
      await sessionsApi.delete(id);
      sessions.update(list => list.filter(s => s.id !== id));
      if (get(activeSessionId) === id) { activeSessionId.set(null); messages.set([]); }
    } catch { toast('error', 'Suppression échouée'); }
  }

  function formatDate(iso: string): string {
    const d = new Date(iso);
    const diff = (Date.now() - d.getTime()) / 1000;
    if (diff < 60) return "À l'instant";
    if (diff < 3600) return `Il y a ${Math.floor(diff / 60)} min`;
    if (diff < 86400) return `Il y a ${Math.floor(diff / 3600)} h`;
    return d.toLocaleDateString('fr-FR', { day: '2-digit', month: 'short' });
  }
</script>

<div class="sidebar-inner">
  <div class="sidebar-head">
    <span class="sidebar-title">Sessions</span>
    <button class="new-btn" onclick={newSession} title="Nouvelle session">✚</button>
  </div>

  <div class="session-list">
    {#each $sessions as s (s.id)}
      <div
        class="session-item {$activeSessionId === s.id ? 'active' : ''}"
        onclick={() => activeSessionId.set(s.id)}
        onkeydown={(e) => e.key === 'Enter' && activeSessionId.set(s.id)}
        role="button"
        tabindex="0"
      >
        <div class="session-info">
          <span class="session-title">{s.title}</span>
          <span class="session-meta">{formatDate(s.updated_at)}</span>
        </div>
        <button class="del-btn" onclick={(e) => deleteSession(s.id, e)} title="Supprimer">✕</button>
      </div>
    {:else}
      <p class="empty-hint">Aucune session<br/>Cliquez sur ✚ pour commencer</p>
    {/each}
  </div>
</div>

<style>
  .sidebar-inner { display: flex; flex-direction: column; height: 100%; overflow: hidden; }
  .sidebar-head { display: flex; align-items: center; justify-content: space-between; padding: 10px 12px; border-bottom: 1px solid var(--border); flex-shrink: 0; }
  .sidebar-title { font-size: 11px; font-weight: 500; text-transform: uppercase; letter-spacing: 0.07em; color: var(--text-muted); }
  .new-btn { background: var(--accent-dim); border: 1px solid var(--accent-border); color: var(--accent-light); width: 22px; height: 22px; border-radius: 5px; font-size: 13px; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.12s; }
  .new-btn:hover { background: var(--accent); color: #fff; }
  .session-list { flex: 1; overflow-y: auto; padding: 6px 0; }
  .session-item { display: flex; align-items: center; width: 100%; padding: 8px 10px 8px 12px; cursor: pointer; gap: 4px; transition: background 0.1s; border-right: 2px solid transparent; }
  .session-item:hover { background: var(--bg-hover); }
  .session-item:focus { outline: none; background: var(--bg-hover); }
  .session-item.active { background: var(--bg-active); border-right-color: var(--accent); }
  .session-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
  .session-title { font-size: 12.5px; color: var(--text-primary); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
  .session-meta { font-size: 10.5px; color: var(--text-muted); }
  .del-btn { background: transparent; border: none; color: var(--text-muted); font-size: 11px; cursor: pointer; padding: 2px 4px; border-radius: 3px; opacity: 0; transition: all 0.1s; flex-shrink: 0; }
  .session-item:hover .del-btn { opacity: 1; }
  .del-btn:hover { background: var(--red-dim); color: var(--red); }
  .empty-hint { text-align: center; color: var(--text-muted); font-size: 12px; padding: 24px 12px; line-height: 1.7; }
</style>
