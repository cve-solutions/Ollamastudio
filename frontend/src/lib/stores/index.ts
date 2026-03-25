/**
 * Stores globaux OllamaStudio — Svelte 5 avec $state runes.
 */
import { writable, derived, get } from 'svelte/store';
import type { Session, Message, Skill, ModelInfo, AppSetting } from '$api';

// ── Configuration Ollama (synchronisée avec la BDD) ─────────────────────────

function createOllamaConfig() {
  const { subscribe, set, update } = writable({
    base_url: 'http://localhost:11434',
    api_mode: 'openai',
    default_model: 'qwen3-coder',
  });

  return {
    subscribe,
    set,
    update,
  };
}

export const ollamaConfig = createOllamaConfig();

// ── Paramètres applicatifs (depuis BDD) ─────────────────────────────────────

export const appSettings = writable<AppSetting[]>([]);

// ── Modèles disponibles ───────────────────────────────────────────────────────

export const models = writable<ModelInfo[]>([]);
export const modelsLoading = writable(false);

// ── Sessions ─────────────────────────────────────────────────────────────────

export const sessions = writable<Session[]>([]);
export const activeSessionId = writable<number | null>(null);
export const messages = writable<Message[]>([]);

// ── UI streamng ───────────────────────────────────────────────────────────────

export interface StreamingMessage {
  content: string;
  tool_events: ToolEvent[];
  done: boolean;
}

export interface ToolEvent {
  type: 'start' | 'result';
  name: string;
  args?: Record<string, unknown>;
  result?: Record<string, unknown>;
  call_id: string;
}

export const streaming = writable<StreamingMessage | null>(null);
export const isStreaming = derived(streaming, ($s) => $s !== null && !$s?.done);

// ── Skill active ──────────────────────────────────────────────────────────────

export const activeSkill = writable<Skill | null>(null);
export const skills = writable<Skill[]>([]);

// ── Layout ────────────────────────────────────────────────────────────────────

export type PanelId = 'chat' | 'editor' | 'terminal' | 'documents' | 'mcp';

export const activePanel = writable<PanelId>('chat');
export const showFileExplorer = writable(true);
export const showSessionSidebar = writable(true);
export const showSettings = writable(false);
export const showTemplates = writable(false);
export const showSkillSelector = writable(false);

// ── Fichier ouvert dans l'éditeur ─────────────────────────────────────────────

export interface OpenFile {
  path: string;
  content: string;
  modified: boolean;
  language: string;
}

export const openFiles = writable<OpenFile[]>([]);
export const activeFilePath = writable<string | null>(null);

export const activeFile = derived(
  [openFiles, activeFilePath],
  ([$files, $path]) => $files.find((f) => f.path === $path) ?? null
);

// ── Notifications toast ───────────────────────────────────────────────────────

export interface Toast {
  id: string;
  type: 'success' | 'error' | 'info' | 'warning';
  message: string;
}

export const toasts = writable<Toast[]>([]);

let _toastId = 0;
export function toast(type: Toast['type'], message: string, duration = 3500) {
  const id = String(++_toastId);
  toasts.update((t) => [...t, { id, type, message }]);
  setTimeout(() => toasts.update((t) => t.filter((x) => x.id !== id)), duration);
}

// ── Modèle sélectionné ────────────────────────────────────────────────────────

export const selectedModel = writable<string>('qwen3-coder');

// ── Debug mode ───────────────────────────────────────────────────────────────

export const debugMode = writable(false);
export const showDebugPanel = writable(false);

// ── Terminal WebSocket URL ────────────────────────────────────────────────────

export const wsTerminalUrl = derived(
  ollamaConfig,
  () => {
    if (typeof window === 'undefined') return 'ws://localhost:3000/api/shell/ws';
    const proto = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    return `${proto}//${window.location.host}/api/shell/ws`;
  }
);
