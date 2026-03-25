<script lang="ts">
  import { onMount, onDestroy } from 'svelte';

  let container: HTMLElement;
  let terminal: any;
  let fitAddon: any;
  let ws: WebSocket | null = null;
  let mounted = false;
  let dataListenerDispose: (() => void) | null = null;

  // WebSocket URL — même hostname que le navigateur, port 8000 (backend)
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
      // Garantit que xterm.js traite tous les événements clavier
      disableStdin: false,
    });

    fitAddon = new FitAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(new WebLinksAddon());
    terminal.open(container);
    fitAddon.fit();
    mounted = true;

    // Enregistre le handler de données — envoie les frappes au WS
    const disposable = terminal.onData((data: string) => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        ws.send(data);
      }
    });
    dataListenerDispose = () => disposable.dispose();

    connectWs();

    // Resize observer
    const ro = new ResizeObserver(() => {
      fitAddon?.fit();
      sendResize();
    });
    ro.observe(container);

    return () => {
      ro.disconnect();
      dataListenerDispose?.();
    };
  });

  function connectWs() {
    if (!mounted) return;

    ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      terminal.writeln('\x1b[32m● Connecté au shell OllamaStudio\x1b[0m');
      terminal.writeln('\x1b[90mRépertoire de travail: /workspace\x1b[0m\n');
      sendResize();

      // Force le focus sur le textarea interne de xterm.js
      requestAnimationFrame(() => {
        terminal.focus();
        // Double-check : cherche le textarea helper et le focus
        const ta = container?.querySelector('.xterm-helper-textarea') as HTMLTextAreaElement;
        if (ta) {
          ta.focus();
        }
      });
    };

    ws.onmessage = (e) => {
      if (e.data instanceof ArrayBuffer) {
        terminal.write(new Uint8Array(e.data));
      } else if (e.data instanceof Blob) {
        e.data.arrayBuffer().then((buf: ArrayBuffer) => terminal.write(new Uint8Array(buf)));
      } else {
        terminal.write(e.data);
      }
    };

    ws.onclose = (ev) => {
      const reason = ev.reason ? ` (${ev.reason})` : '';
      terminal?.writeln(`\n\x1b[31m● Connexion fermée [code=${ev.code}]${reason}\x1b[0m`);
      terminal?.writeln(`\x1b[90m  Cliquez "Reconnecter" pour réessayer\x1b[0m`);
    };

    ws.onerror = () => {
      terminal?.writeln(`\n\x1b[31m● Erreur de connexion WebSocket\x1b[0m`);
      terminal?.writeln(`\x1b[90m  URL: ${WS_URL}\x1b[0m`);
    };
  }

  function sendResize() {
    if (ws?.readyState === WebSocket.OPEN && terminal) {
      ws.send(JSON.stringify({
        type: 'resize',
        rows: terminal.rows,
        cols: terminal.cols,
      }));
    }
  }

  function reconnect() {
    ws?.close();
    terminal?.clear();
    connectWs();
  }

  function focusTerminal() {
    if (!terminal) return;
    terminal.focus();
    // Aussi focus le textarea helper directement
    const ta = container?.querySelector('.xterm-helper-textarea') as HTMLTextAreaElement;
    if (ta) ta.focus();
  }

  // Fallback : si xterm.js ne capte pas les touches, on les envoie
  // directement via un keydown listener sur le conteneur
  function handleContainerKeydown(e: KeyboardEvent) {
    if (!ws || ws.readyState !== WebSocket.OPEN) return;
    // Ne pas interférer si xterm.js gère déjà (target = textarea helper)
    if ((e.target as HTMLElement)?.classList?.contains('xterm-helper-textarea')) return;

    // Touches spéciales
    if (e.key === 'Enter') { ws.send('\r'); e.preventDefault(); return; }
    if (e.key === 'Backspace') { ws.send('\x7f'); e.preventDefault(); return; }
    if (e.key === 'Tab') { ws.send('\t'); e.preventDefault(); return; }
    if (e.key === 'Escape') { ws.send('\x1b'); e.preventDefault(); return; }
    if (e.key === 'ArrowUp') { ws.send('\x1b[A'); e.preventDefault(); return; }
    if (e.key === 'ArrowDown') { ws.send('\x1b[B'); e.preventDefault(); return; }
    if (e.key === 'ArrowRight') { ws.send('\x1b[C'); e.preventDefault(); return; }
    if (e.key === 'ArrowLeft') { ws.send('\x1b[D'); e.preventDefault(); return; }

    // Ctrl+C, Ctrl+D, etc.
    if (e.ctrlKey && e.key.length === 1) {
      const code = e.key.toLowerCase().charCodeAt(0) - 96;
      if (code > 0 && code <= 26) {
        ws.send(String.fromCharCode(code));
        e.preventDefault();
        return;
      }
    }

    // Caractères imprimables
    if (e.key.length === 1 && !e.ctrlKey && !e.altKey && !e.metaKey) {
      ws.send(e.key);
      e.preventDefault();
    }
  }

  onDestroy(() => {
    ws?.close();
    dataListenerDispose?.();
    terminal?.dispose();
  });
</script>

<div class="terminal-panel">
  <div class="term-topbar">
    <span class="term-title">Terminal Shell</span>
    <div class="term-actions">
      <button class="term-btn" onclick={() => terminal?.clear()} title="Effacer">Effacer</button>
      <button class="term-btn" onclick={reconnect} title="Reconnecter">Reconnecter</button>
    </div>
  </div>
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div
    class="xterm-wrap"
    bind:this={container}
    onclick={focusTerminal}
    onkeydown={handleContainerKeydown}
    tabindex="0"
  ></div>
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
    overflow: hidden;
    position: relative;
    outline: none;
  }

  .xterm-wrap:focus-within {
    /* Indication visuelle que le terminal a le focus */
    box-shadow: inset 0 0 0 1px rgba(124,111,247,0.3);
  }

  /* S'assure que le textarea helper de xterm.js est accessible */
  :global(.xterm-helper-textarea) {
    position: absolute !important;
    opacity: 0 !important;
    z-index: 10 !important;
    pointer-events: auto !important;
  }

  :global(.xterm-viewport) { overflow-y: auto !important; }
  :global(.xterm-screen) { image-rendering: pixelated; }
</style>
