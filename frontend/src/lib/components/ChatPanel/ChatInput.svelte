<script lang="ts">
  import { isStreaming, showTemplates } from '$lib/stores';

  interface Props {
    onSend: (content: string) => void;
    onStop: () => void;
    disabled?: boolean;
  }

  let { onSend, onStop, disabled = false }: Props = $props();
  let value = $state('');
  let textareaEl: HTMLTextAreaElement;

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      submit();
    }
  }

  function submit() {
    const msg = value.trim();
    if (!msg || $isStreaming) return;
    value = '';
    resizeTextarea();
    onSend(msg);
  }

  function resizeTextarea() {
    if (!textareaEl) return;
    textareaEl.style.height = 'auto';
    textareaEl.style.height = Math.min(textareaEl.scrollHeight, 200) + 'px';
  }

  // Injection de texte depuis les templates
  export function injectText(text: string) {
    value = text;
    resizeTextarea();
    textareaEl?.focus();
  }
</script>

<div class="chat-input-area">
  <div class="input-wrapper {$isStreaming ? 'streaming' : ''}">
    <textarea
      bind:this={textareaEl}
      bind:value
      onkeydown={handleKeydown}
      oninput={resizeTextarea}
      placeholder="Message… (Entrée pour envoyer, Shift+Entrée pour retour à la ligne)"
      rows="1"
      disabled={disabled}
    ></textarea>

    <div class="input-actions">
      <button
        class="btn-icon"
        onclick={() => showTemplates.update(v => !v)}
        title="Templates de prompts"
        tabindex="-1"
      >📋</button>

      {#if $isStreaming}
        <button class="send-btn stop" onclick={onStop} title="Arrêter">⬛</button>
      {:else}
        <button
          class="send-btn {value.trim() ? 'active' : ''}"
          onclick={submit}
          disabled={!value.trim() || disabled}
          title="Envoyer (Entrée)"
        >➤</button>
      {/if}
    </div>
  </div>

  <p class="hint">Modèles Ollama · Les fichiers restent sur votre serveur</p>
</div>

<style>
  .chat-input-area {
    padding: 8px 16px 12px;
    flex-shrink: 0;
    border-top: 1px solid var(--border);
    background: var(--bg-primary);
  }

  .input-wrapper {
    display: flex;
    align-items: flex-end;
    gap: 8px;
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 8px 8px 8px 14px;
    transition: border-color 0.15s;
  }
  .input-wrapper:focus-within { border-color: var(--border-focus); }
  .input-wrapper.streaming { border-color: var(--accent-border); }

  textarea {
    flex: 1;
    background: transparent;
    border: none;
    outline: none;
    color: var(--text-primary);
    font-family: var(--font-sans);
    font-size: 14px;
    line-height: 1.5;
    resize: none;
    min-height: 22px;
    max-height: 200px;
    overflow-y: auto;
  }
  textarea::placeholder { color: var(--text-muted); }
  textarea:disabled { opacity: 0.5; cursor: not-allowed; }

  .input-actions {
    display: flex;
    align-items: center;
    gap: 4px;
    flex-shrink: 0;
  }

  .btn-icon {
    width: 28px;
    height: 28px;
    border-radius: 5px;
    border: none;
    background: transparent;
    font-size: 15px;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.12s;
  }
  .btn-icon:hover { background: var(--bg-hover); color: var(--text-secondary); }

  .send-btn {
    width: 32px;
    height: 32px;
    border-radius: 7px;
    border: none;
    font-size: 14px;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 0.15s;
    background: var(--bg-tertiary);
    color: var(--text-muted);
  }
  .send-btn.active {
    background: var(--accent);
    color: #fff;
  }
  .send-btn.active:hover { background: var(--accent-light); }
  .send-btn.stop {
    background: var(--red-dim);
    color: var(--red);
    border: 1px solid rgba(248,113,113,0.2);
  }
  .send-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .hint {
    font-size: 10.5px;
    color: var(--text-muted);
    text-align: center;
    margin-top: 6px;
  }
</style>
