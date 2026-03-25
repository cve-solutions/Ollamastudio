<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { ollamaConfig } from '$lib/stores';
  import { get } from 'svelte/store';

  let container: HTMLElement;
  let terminal: any;
  let fitAddon: any;
  let ws: WebSocket | null = null;
  let mounted = false;

  // WebSocket URL — même hostname que le navigateur, port 8000 (backend)
  // Le WS ne passe pas par le proxy SvelteKit (pas de support WS natif),
  // il est connecté directement au backend exposé sur le port 8000.
  const WS_URL = (() => {
    if (typeof window === 'undefined') return 'ws://localhost:8000/api/shell/ws';
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const hostname = window.location.hostname;
    return `${proto}//${hostname}:8000/api/shell/ws`;
  })();

  onMount(async () => {
    const { Terminal } = await import('@xterm/xterm');
    const { FitAddon } = await import('@xterm/addon-fit');
    const { WebLinksAddon } = await import('@xterm/addon-web-links');

    terminal = new Terminal({
      theme: {
        background: '#0d0d0f',
        foreground: '#e8e8f0',
        cursor: '#7c6ff7',
        cursorAccent: '#0d0d0f',
        selectionBackground: 'rgba(124,111,247,0.3)',
        black: '#13131a',
        red: '#f87171',
        green: '#4ade80',
        yellow: '#fbbf24',
        blue: '#7c6ff7',
        magenta: '#c084fc',
        cyan: '#2dd4bf',
        white: '#e8e8f0',
        brightBlack: '#5a5a72',
        brightRed: '#fca5a5',
        brightGreen: '#86efac',
        brightYellow: '#fde68a',
        brightBlue: '#a5b4fc',
        brightMagenta: '#d8b4fe',
        brightCyan: '#99f6e4',
        brightWhite: '#f1f5f9',
      },
      fontFamily: "'Geist Mono', 'JetBrains Mono', 'Fira Code', monospace",
      fontSize: 13,
      lineHeight: 1.4,
      cursorBlink: true,
      cursorStyle: 'bar',
      allowProposedApi: true,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.open(container);
    fitAddon.fit();
    mounted = true;

    connectWs();

    // Resize observer
    const ro = new ResizeObserver(() => fitAddon?.fit());
    ro.observe(container);

    return () => ro.disconnect();
  });

  function connectWs() {
    if (!mounted) return;
    ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      terminal.writeln('\x1b[32m● Connecté au shell OllamaStudio\x1b[0m');
      terminal.writeln('\x1b[90mRépertoire de travail: /workspace\x1b[0m\n');

      // Envoie le resize initial
      sendResize();

      // Redirige les touches vers le WS
      terminal.onData((data: string) => {
        if (ws?.readyState === WebSocket.OPEN) {
          ws.send(data);
        }
      });

      // Focus le terminal pour capturer les frappes
      terminal.focus();
    };

    ws.onmessage = (e) => {
      if (e.data instanceof ArrayBuffer) {
        terminal.write(new Uint8Array(e.data));
      } else if (e.data instanceof Blob) {
        e.data.arrayBuffer().then(buf => terminal.write(new Uint8Array(buf)));
      } else {
        terminal.write(e.data);
      }
    };

    ws.onclose = (ev) => {
      const reason = ev.reason ? ` (${ev.reason})` : '';
      terminal?.writeln(`\n\x1b[31m● Connexion fermée [code=${ev.code}]${reason}\x1b[0m`);
      terminal?.writeln(`\x1b[90m  URL: ${WS_URL}\x1b[0m`);
      terminal?.writeln(`\x1b[90m  Cliquez "Reconnecter" pour réessayer\x1b[0m`);
      console.warn('[Terminal WS] close', { code: ev.code, reason: ev.reason, wasClean: ev.wasClean, url: WS_URL });
    };

    ws.onerror = (ev) => {
      terminal?.writeln(`\n\x1b[31m● Erreur de connexion WebSocket\x1b[0m`);
      terminal?.writeln(`\x1b[90m  URL: ${WS_URL}\x1b[0m`);
      terminal?.writeln(`\x1b[90m  Vérifiez que le backend est démarré et accessible\x1b[0m`);
      console.error('[Terminal WS] error', ev, { url: WS_URL });
    };
  }

  function sendResize() {
    if (ws?.readyState === WebSocket.OPEN && terminal) {
      const ctrl = JSON.stringify({
        type: 'resize',
        rows: terminal.rows,
        cols: terminal.cols,
      });
      ws.send(ctrl);
    }
  }

  function reconnect() {
    ws?.close();
    terminal?.clear();
    connectWs();
  }

  onDestroy(() => {
    ws?.close();
    terminal?.dispose();
  });
</script>

<div class="terminal-panel">
  <div class="term-topbar">
    <span class="term-title">⌨️ Terminal Shell</span>
    <div class="term-actions">
      <button class="term-btn" onclick={() => terminal?.clear()} title="Effacer">⌫ Effacer</button>
      <button class="term-btn" onclick={reconnect} title="Reconnecter">↺ Reconnecter</button>
    </div>
  </div>
  <div class="xterm-wrap" bind:this={container} onclick={() => terminal?.focus()}></div>
</div>

<style>
  .terminal-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background: #0d0d0f;
  }

  .term-topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 7px 12px;
    background: var(--bg-secondary);
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .term-title {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .term-actions {
    display: flex;
    gap: 6px;
  }

  .term-btn {
    background: transparent;
    border: 1px solid var(--border);
    color: var(--text-muted);
    padding: 3px 8px;
    border-radius: 4px;
    font-size: 11px;
    cursor: pointer;
    transition: all 0.12s;
  }
  .term-btn:hover { background: var(--bg-hover); color: var(--text-secondary); }

  .xterm-wrap {
    flex: 1;
    padding: 8px;
    overflow: hidden;
  }

  :global(.xterm-viewport) { overflow-y: auto !important; }
  :global(.xterm-screen) { image-rendering: pixelated; }
</style>
