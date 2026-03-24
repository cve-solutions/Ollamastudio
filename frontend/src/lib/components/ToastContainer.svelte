<script lang="ts">
  import { toasts } from '$lib/stores';
  const ICONS = { success: '✓', error: '✗', info: 'ℹ', warning: '⚠' };
</script>

<div class="toast-container">
  {#each $toasts as t (t.id)}
    <div class="toast {t.type} fade-in">
      <span class="toast-icon">{ICONS[t.type]}</span>
      <span>{t.message}</span>
    </div>
  {/each}
</div>

<style>
  .toast-container {
    position: fixed;
    bottom: 20px;
    right: 20px;
    z-index: 999;
    display: flex;
    flex-direction: column;
    gap: 6px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 9px 14px;
    border-radius: 8px;
    font-size: 13px;
    max-width: 360px;
    border: 1px solid;
    pointer-events: all;
  }

  .toast.success { background: var(--green-dim); border-color: rgba(74,222,128,0.2); color: var(--green); }
  .toast.error   { background: var(--red-dim);   border-color: rgba(248,113,113,0.2); color: var(--red); }
  .toast.info    { background: var(--accent-dim); border-color: var(--accent-border); color: var(--accent-light); }
  .toast.warning { background: var(--coral-dim);  border-color: rgba(249,115,22,0.2); color: var(--coral); }

  .toast-icon { font-size: 14px; flex-shrink: 0; }
</style>
