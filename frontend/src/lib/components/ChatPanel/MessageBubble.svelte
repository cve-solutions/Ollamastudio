<script lang="ts">
  import { marked } from 'marked';
  import hljs from 'highlight.js';
  import type { Message } from '$lib/api';

  interface Props {
    msg: Message;
    isStreaming?: boolean;
  }

  let { msg, isStreaming = false }: Props = $props();

  // Configuration marked avec highlight.js
  marked.setOptions({
    highlight: (code: string, lang: string) => {
      if (lang && hljs.getLanguage(lang)) {
        try { return hljs.highlight(code, { language: lang }).value; } catch {}
      }
      return hljs.highlightAuto(code).value;
    },
    breaks: true,
    gfm: true,
  } as any);

  function renderContent(content: string): string {
    if (!content) return '';
    return marked.parse(content) as string;
  }

  function copyCode(e: MouseEvent) {
    const btn = e.currentTarget as HTMLElement;
    const pre = btn.closest('.code-block')?.querySelector('code');
    if (pre) {
      navigator.clipboard.writeText(pre.innerText);
      btn.textContent = '✓';
      setTimeout(() => btn.textContent = '⎘', 1500);
    }
  }
</script>

<div class="message {msg.role} {isStreaming ? 'streaming' : ''}">
  {#if msg.role === 'user'}
    <div class="user-bubble">
      <p>{msg.content}</p>
    </div>
  {:else}
    <div class="assistant-wrap">
      <div class="avatar">◎</div>
      <div class="assistant-content">
        <div class="prose" style="position:relative">
          {@html renderContent(msg.content)}
          {#if isStreaming}
            <span class="cursor">▋</span>
          {/if}
        </div>
        {#if msg.tool_calls && msg.tool_calls.length > 0}
          <div class="tool-summary">
            {#each msg.tool_calls as tc: any}
              <span class="tool-badge">⚙ {tc.name}</span>
            {/each}
          </div>
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .message {
    padding: 4px 16px;
    animation: fade-in 0.1s ease-out;
  }

  /* Message utilisateur — bulle droite */
  .user-bubble {
    display: flex;
    justify-content: flex-end;
    margin: 6px 0;
  }
  .user-bubble p {
    max-width: 72%;
    background: var(--accent-dim);
    border: 1px solid var(--accent-border);
    color: var(--text-primary);
    padding: 9px 14px;
    border-radius: 14px 14px 4px 14px;
    font-size: 14px;
    line-height: 1.55;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Message assistant */
  .assistant-wrap {
    display: flex;
    gap: 10px;
    align-items: flex-start;
    margin: 4px 0;
  }

  .avatar {
    flex-shrink: 0;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--accent-dim);
    border: 1px solid var(--accent-border);
    color: var(--accent-light);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 13px;
    margin-top: 2px;
  }

  .assistant-content {
    flex: 1;
    min-width: 0;
    max-width: 88%;
  }

  .cursor {
    display: inline-block;
    color: var(--accent);
    animation: pulse-dot 0.8s ease-in-out infinite;
    margin-left: 2px;
    font-size: 16px;
    vertical-align: middle;
  }

  .tool-summary {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
    margin-top: 6px;
  }

  .tool-badge {
    font-size: 11px;
    padding: 2px 7px;
    border-radius: 10px;
    background: var(--bg-tertiary);
    border: 1px solid var(--border);
    color: var(--text-muted);
    font-family: var(--font-mono);
  }

  /* Surcharge styles code dans le chat */
  :global(.message .prose pre) {
    position: relative;
  }
  :global(.message .prose pre::after) {
    content: '⎘';
    position: absolute;
    top: 8px;
    right: 10px;
    font-size: 14px;
    color: var(--text-muted);
    cursor: pointer;
    opacity: 0.6;
  }
  :global(.message .prose pre:hover::after) { opacity: 1; }
</style>
