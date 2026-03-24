/**
 * Client API OllamaStudio — toutes les requêtes HTTP vers le backend FastAPI.
 */

const BASE = import.meta.env.PUBLIC_API_URL || 'http://localhost:8000';

async function api<T>(path: string, init?: RequestInit): Promise<T> {
  const resp = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json', ...init?.headers },
    ...init,
  });
  if (!resp.ok) {
    const err = await resp.json().catch(() => ({ detail: resp.statusText }));
    throw new Error(err.detail || `Erreur ${resp.status}`);
  }
  return resp.json();
}

// ── Modèles ─────────────────────────────────────────────────────────────────

export interface ModelInfo {
  name: string;
  size: number | null;
  modified_at: string | null;
}

export const modelsApi = {
  list: (baseUrl?: string) => {
    const qs = baseUrl ? `?base_url=${encodeURIComponent(baseUrl)}` : '';
    return api<ModelInfo[]>(`/api/models/${qs}`);
  },
  getConfig: () => api<Record<string, string>>('/api/models/config'),
};

// ── Sessions ─────────────────────────────────────────────────────────────────

export interface Session {
  id: number;
  title: string;
  model: string;
  skill_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface Message {
  id: number;
  session_id: number;
  role: 'user' | 'assistant' | 'tool';
  content: string;
  tool_calls: unknown[] | null;
  tool_results: unknown[] | null;
  created_at: string;
}

export const sessionsApi = {
  list: () => api<Session[]>('/api/sessions/'),
  create: (data: { title?: string; model: string; skill_id?: string }) =>
    api<Session>('/api/sessions/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: number, data: Partial<Session>) =>
    api<Session>(`/api/sessions/${id}`, { method: 'PATCH', body: JSON.stringify(data) }),
  delete: (id: number) =>
    fetch(`${BASE}/api/sessions/${id}`, { method: 'DELETE' }),
  messages: (id: number) => api<Message[]>(`/api/sessions/${id}/messages`),
};

// ── Chat streaming ────────────────────────────────────────────────────────────

export interface ChatRequest {
  session_id: number;
  content: string;
  model: string;
  ollama_base_url?: string;
  ollama_api_mode?: string;
  enabled_tools?: string[] | null;
  temperature?: number;
  max_tokens?: number;
}

export type ChatEvent =
  | { type: 'text'; content: string }
  | { type: 'tool_start'; name: string; args: Record<string, unknown>; call_id: string }
  | { type: 'tool_result'; name: string; result: Record<string, unknown>; call_id: string }
  | { type: 'error'; message: string }
  | { type: 'done'; iterations: number };

export async function* streamChat(req: ChatRequest): AsyncGenerator<ChatEvent> {
  const resp = await fetch(`${BASE}/api/chat/stream`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(req),
  });

  if (!resp.ok || !resp.body) {
    throw new Error(`Erreur chat: ${resp.statusText}`);
  }

  const reader = resp.body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });
    const lines = buffer.split('\n');
    buffer = lines.pop() ?? '';

    for (const line of lines) {
      if (line.startsWith('data: ')) {
        try {
          yield JSON.parse(line.slice(6)) as ChatEvent;
        } catch {
          /* ignore */
        }
      }
    }
  }
}

// ── Fichiers ─────────────────────────────────────────────────────────────────

export interface FileEntry {
  name: string;
  path: string;
  type: 'file' | 'directory';
  size: number | null;
  modified: number;
  mime: string | null;
  has_children?: boolean;
}

export const filesApi = {
  tree: (path = '.') =>
    api<{ path: string; entries: FileEntry[] }>(`/api/files/tree?path=${encodeURIComponent(path)}`),
  read: (path: string) =>
    api<{ path: string; content: string; mime: string | null; size: number }>
      (`/api/files/read?path=${encodeURIComponent(path)}`),
  write: (path: string, content: string) =>
    api<{ path: string; size: number }>('/api/files/write', {
      method: 'POST',
      body: JSON.stringify({ path, content }),
    }),
  delete: (path: string) =>
    api<{ deleted: string }>('/api/files/delete', {
      method: 'DELETE',
      body: JSON.stringify({ path }),
    }),
  rename: (old_path: string, new_path: string) =>
    api('/api/files/rename', { method: 'POST', body: JSON.stringify({ old_path, new_path }) }),
  mkdir: (path: string) =>
    api('/api/files/mkdir', { method: 'POST', body: JSON.stringify({ path }) }),
};

// ── Templates ────────────────────────────────────────────────────────────────

export interface Template {
  id: string;
  name: string;
  category: string;
  icon: string;
  content: string;
  variables: string[];
}

export const templatesApi = {
  list: () => api<Template[]>('/api/templates/'),
  get: (id: string) => api<Template>(`/api/templates/${id}`),
  create: (data: Omit<Template, never>) =>
    api<Template>('/api/templates/', { method: 'POST', body: JSON.stringify(data) }),
  delete: (id: string) =>
    fetch(`${BASE}/api/templates/${id}`, { method: 'DELETE' }),
};

// ── Skills ────────────────────────────────────────────────────────────────────

export interface Skill {
  id: string;
  name: string;
  description: string;
  icon: string;
  system_prompt: string;
  enabled_tools: string[] | null;
  temperature: number;
  max_tokens: number;
  color: string;
}

export interface SkillImportResult {
  imported: number;
  skipped: number;
  errors: number;
  details: {
    imported: { id: string; name: string }[];
    skipped: { index: number; name: string; reason: string }[];
    errors: { index: number; name?: string; error: string }[];
  };
}

export const skillsApi = {
  list: () => api<Skill[]>('/api/skills/'),
  get: (id: string) => api<Skill>(`/api/skills/${id}`),
  create: (data: Skill) =>
    api<Skill>('/api/skills/', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: string, data: Skill) =>
    api<Skill>(`/api/skills/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: string) =>
    fetch(`${BASE}/api/skills/${id}`, { method: 'DELETE' }),
  importJson: async (file: File): Promise<SkillImportResult> => {
    const form = new FormData();
    form.append('file', file);
    const resp = await fetch(`${BASE}/api/skills/import`, {
      method: 'POST',
      body: form,
    });
    if (!resp.ok) {
      const err = await resp.json().catch(() => ({ detail: resp.statusText }));
      throw new Error(err.detail || `Erreur ${resp.status}`);
    }
    return resp.json();
  },
};

// ── Documents ────────────────────────────────────────────────────────────────

export interface Document {
  id: number;
  filename: string;
  relative_path: string;
  mime_type: string;
  size_bytes: number;
  chunk_count: number;
  imported_at: string;
}

export const documentsApi = {
  list: () => api<Document[]>('/api/documents/'),
  search: (q: string, max?: number) =>
    api<{ results: unknown[]; count: number; query: string }>(
      `/api/documents/search?q=${encodeURIComponent(q)}${max ? `&max_results=${max}` : ''}`
    ),
  importFolder: (folder_path: string) =>
    api('/api/documents/import-folder', {
      method: 'POST',
      body: JSON.stringify({ folder_path }),
    }),
  delete: (id: number) =>
    fetch(`${BASE}/api/documents/${id}`, { method: 'DELETE' }),
};

// ── Settings (Paramètres persistés en BDD) ──────────────────────────────────

export interface AppSetting {
  key: string;
  value: unknown;
  category: string;
  label: string;
  description: string;
  value_type: string;
}

export interface ConnectionTestResult {
  connected: boolean;
  model_count: number;
  models: ModelInfo[];
}

export const settingsApi = {
  list: () => api<AppSetting[]>('/api/settings/'),
  byCategory: (category: string) => api<AppSetting[]>(`/api/settings/category/${category}`),
  get: (key: string) => api<{ key: string; value: unknown }>(`/api/settings/${key}`),
  update: (key: string, value: unknown) =>
    api<AppSetting>(`/api/settings/${key}`, {
      method: 'PUT',
      body: JSON.stringify({ value }),
    }),
  bulkUpdate: (settings: Record<string, unknown>) =>
    api<AppSetting[]>('/api/settings/', {
      method: 'PUT',
      body: JSON.stringify({ settings }),
    }),
  testConnection: (base_url: string) =>
    api<ConnectionTestResult>('/api/settings/ollama/test-connection', {
      method: 'POST',
      body: JSON.stringify({ base_url }),
    }),
};

// ── MCP ───────────────────────────────────────────────────────────────────────

export interface McpServer {
  id: string;
  name: string;
  description: string;
  url: string;
  enabled: boolean;
  type: string;
}

export const mcpApi = {
  list: () => api<McpServer[]>('/api/mcp/servers'),
  add: (data: McpServer) =>
    api<McpServer>('/api/mcp/servers', { method: 'POST', body: JSON.stringify(data) }),
  update: (id: string, data: McpServer) =>
    api<McpServer>(`/api/mcp/servers/${id}`, { method: 'PUT', body: JSON.stringify(data) }),
  delete: (id: string) =>
    fetch(`${BASE}/api/mcp/servers/${id}`, { method: 'DELETE' }),
  ping: (id: string) =>
    api<{ reachable: boolean; status?: number; error?: string }>(`/api/mcp/servers/${id}/ping`),
  tools: (id: string) =>
    api<{ tools: unknown[] }>(`/api/mcp/servers/${id}/tools`),
};
