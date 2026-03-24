<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { get } from 'svelte/store';
  import {
    activeSessionId, messages, streaming, isStreaming,
    selectedModel, activeSkill, ollamaConfig, sessions,
    showTemplates, toast
  } from '$lib/stores';
  import { sessionsApi, streamChat } from '$lib/api';
  import type { ChatEvent } from '$lib/api';
  import MessageBubble from './MessageBubble.svelte';
  import ChatInput     from './ChatInput.svelte';
  import ToolCallCard  from './ToolCallCard.svelte';
  import WelcomeScreen from './WelcomeScreen.svelte';

  let messagesEl = $state<HTMLElement | undefined>(undefined);
  let abortController: AbortController | null = null;

  // Charge les messages à chaque changement de session
  $effect(() => {
    const sid = $activeSessionId;
    if (sid == null) { messages.set([]); return; }
    sessionsApi.messages(sid).then(msgs => {
      messages.set(msgs);
      scrollBottom();
    });
  });

  async function scrollBottom() {
    await tick();
    if (messagesEl) messagesEl.scrollTop = messagesEl.scrollHeight;
  }

  async function sendMessage(content: string) {
    if ($isStreaming) return;

    const cfg = get(ollamaConfig);
    const model = get(selectedModel);
    const skill = get(activeSkill);

    // Crée une session si aucune n'est active
    let sid = get(activeSessionId);
    if (sid == null) {
      const s = await sessionsApi.create({
        title: content.slice(0, 60),
        model,
        skill_id: skill?.id,
      });
      sessions.update(list => [s, ...list]);
      activeSessionId.set(s.id);
      sid = s.id;
    }

    // Ajoute le message utilisateur localement (optimistic UI)
    messages.update(m => [...m, {
      id: Date.now(),
      session_id: sid!,
      role: 'user',
      content,
      tool_calls: null,
      tool_results: null,
      created_at: new Date().toISOString(),
    }]);
    await scrollBottom();

    // Lance le streaming
    streaming.set({ content: '', tool_events: [], done: false });
    let assistantText = '';

    try {
      for await (const evt of streamChat({
        session_id: sid!,
        content,
        model,
        ollama_base_url: cfg.base_url,
        ollama_api_mode: cfg.api_mode,
        enabled_tools: skill?.enabled_tools ?? undefined,
        temperature: skill?.temperature ?? 0.7,
        max_tokens: skill?.max_tokens ?? 4096,
      })) {
        if (evt.type === 'text') {
          assistantText += evt.content;
          streaming.update(s => s ? { ...s, content: assistantText } : s);
          await scrollBottom();
        } else if (evt.type === 'tool_start') {
          streaming.update(s => s ? {
            ...s,
            tool_events: [...s.tool_events, { type: 'start', name: evt.name, args: evt.args, call_id: evt.call_id }]
          } : s);
          await scrollBottom();
        } else if (evt.type === 'tool_result') {
          streaming.update(s => s ? {
            ...s,
            tool_events: [...s.tool_events, { type: 'result', name: evt.name, result: evt.result, call_id: evt.call_id }]
          } : s);
          await scrollBottom();
        } else if (evt.type === 'error') {
          toast('error', evt.message);
        }
      }
    } catch (e) {
      toast('error', `Erreur stream: ${e instanceof Error ? e.message : e}`);
    } finally {
      // Déplace le message assistant dans la liste
      const toolEvents = get(streaming)?.tool_events ?? [];
      streaming.set(null);

      if (assistantText || toolEvents.length) {
        messages.update(m => [...m, {
          id: Date.now() + 1,
          session_id: sid!,
          role: 'assistant',
          content: assistantText,
          tool_calls: toolEvents.filter(e => e.type === 'start').map(e => ({ name: e.name, args: e.args })),
          tool_results: toolEvents.filter(e => e.type === 'result').map(e => ({ name: e.name, result: e.result })),
          created_at: new Date().toISOString(),
        }]);
        await scrollBottom();
      }
    }
  }

  function stopStreaming() {
    abortController?.abort();
    streaming.set(null);
  }
</script>

<div class="chat-panel">
  {#if $messages.length === 0 && !$streaming}
    <WelcomeScreen onSend={sendMessage} />
  {:else}
    <div class="messages" bind:this={messagesEl}>
      {#each $messages as msg (msg.id)}
        <MessageBubble {msg} />
      {/each}

      <!-- Message en cours de streaming -->
      {#if $streaming}
        <div class="streaming-wrap fade-in">
          {#if $streaming.tool_events.length > 0}
            {#each $streaming.tool_events as evt}
              <ToolCallCard {evt} />
            {/each}
          {/if}
          {#if $streaming.content}
            <MessageBubble msg={{
              id: -1,
              session_id: $activeSessionId ?? -1,
              role: 'assistant',
              content: $streaming.content,
              tool_calls: null,
              tool_results: null,
              created_at: new Date().toISOString(),
            }} isStreaming />
          {:else if $streaming.tool_events.length === 0}
            <div class="thinking">
              <span class="dot"></span>
              <span class="dot"></span>
              <span class="dot"></span>
            </div>
          {/if}
        </div>
      {/if}
    </div>
  {/if}

  <ChatInput
    onSend={sendMessage}
    onStop={stopStreaming}
    disabled={false}
  />
</div>

<style>
  .chat-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .messages {
    flex: 1;
    overflow-y: auto;
    padding: 16px 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .streaming-wrap {
    padding: 0 16px;
  }

  .thinking {
    display: flex;
    gap: 4px;
    padding: 12px 16px;
    align-items: center;
  }

  .dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse-dot 1.2s ease-in-out infinite;
  }
  .dot:nth-child(2) { animation-delay: 0.2s; }
  .dot:nth-child(3) { animation-delay: 0.4s; }
</style>
