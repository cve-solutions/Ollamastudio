# OllamaStudio — Architecture technique

## Vue d'ensemble

```mermaid
graph TB
    subgraph Client["Navigateur"]
        UI[SvelteKit Frontend<br/>Port 3000]
    end

    subgraph Server["Serveur"]
        FE[Node.js server.js<br/>SvelteKit SSR + WS Proxy]
        BE[Rust Backend<br/>Axum — Port 8000]
        DB[(SQLite<br/>ollamastudio.db)]
        FS[Filesystem<br/>YAML + workspace]
    end

    subgraph LLM["LLM Local"]
        OL[Ollama<br/>Port 11434]
    end

    UI -->|HTTP /api/*| FE
    UI -->|WS /api/shell/ws| FE
    FE -->|Proxy HTTP| BE
    FE -->|Proxy WS| BE
    BE -->|SQLx| DB
    BE -->|tokio::fs| FS
    BE -->|reqwest SSE| OL
    BE -->|PTY fork| Shell[bash PTY]
```

---

## Architecture du backend (Rust)

```mermaid
graph LR
    subgraph Routes["Routes (axum)"]
        H[health]
        C[chat/stream]
        S[sessions]
        F[files]
        SH[shell/ws]
        D[documents]
        M[models]
        SK[skills]
        T[templates]
        SE[settings]
        MC[mcp]
        DB2[debug]
    end

    subgraph Services
        OC[OllamaClient]
        DP[DocumentProcessor]
        DBuf[DebugBuffer]
    end

    subgraph Agents
        AL[AgenticLoop]
        TL[Tools x8]
    end

    subgraph State
        AS[AppState]
        CFG[Config]
        Pool[SqlitePool]
    end

    C --> AL --> OC
    AL --> TL
    TL --> Pool
    TL --> CFG
    D --> DP
    Routes --> AS
    AS --> Pool
    AS --> CFG
    AS --> DBuf
```

---

## Flux de données — Chat avec outils

```mermaid
sequenceDiagram
    participant B as Navigateur
    participant F as Frontend (Node)
    participant R as Backend (Rust)
    participant O as Ollama
    participant W as Workspace

    B->>F: POST /api/chat/stream
    F->>R: Proxy POST
    R->>R: Persist user message (SQLite)
    R->>R: Load skill system_prompt

    loop Boucle agentique (max 20 itérations)
        R->>O: POST /v1/chat/completions (SSE)
        O-->>R: Stream tokens
        R-->>F: SSE: {type: "text", content: "..."}
        F-->>B: SSE forwarded

        alt LLM demande un outil
            R-->>F: SSE: {type: "tool_start", name: "run_command"}
            R->>W: Exécute l'outil (ex: ls -la)
            W-->>R: Résultat
            R-->>F: SSE: {type: "tool_result", result: {...}}
            R->>O: Renvoi résultat outil + continue
        end
    end

    R-->>F: SSE: {type: "done", iterations: 3}
    R->>R: Persist assistant message (SQLite)
```

---

## Flux de données — Terminal WebSocket

```mermaid
sequenceDiagram
    participant B as Navigateur (xterm.js)
    participant F as Frontend (Node)
    participant R as Backend (Rust)
    participant P as PTY (bash)

    B->>F: WS Upgrade /api/shell/ws
    F->>R: WS Proxy (TCP bidirectionnel)
    R->>R: forkpty() + exec bash
    R-->>B: Prompt bash (binary)

    B->>R: Frappe clavier "ls\r"
    R->>P: write(fd, "ls\r")
    P-->>R: read(fd) → sortie ls
    R-->>B: Binary data (sortie)

    B->>R: JSON {"type":"resize","rows":30,"cols":120}
    R->>P: ioctl(TIOCSWINSZ)

    B->>R: JSON {"type":"signal","signal":"SIGINT"}
    R->>P: kill(pid, SIGINT)
```

---

## Modèle de données

```mermaid
erDiagram
    sessions ||--o{ messages : "contient"
    documents ||--o{ document_chunks : "découpé en"

    sessions {
        int id PK
        text title
        text skill_id
        text model
        text created_at
        text updated_at
        text metadata
    }

    messages {
        int id PK
        int session_id FK
        text role
        text content
        text tool_calls
        text tool_results
        int tokens_used
        text created_at
    }

    documents {
        int id PK
        text filename
        text relative_path
        text mime_type
        int size_bytes
        int chunk_count
        text imported_at
    }

    document_chunks {
        int id PK
        int document_id FK
        int chunk_index
        text content
        int tokens
    }

    app_settings {
        text key PK
        text value
        text category
        text label
        text description
        text value_type
        text updated_at
    }
```

---

## Structure du projet

```
ollamastudio/
├── VERSION                          # Version unique (source de vérité)
├── README.md
├── favicon.ico
│
├── backend-rs/                      # Backend Rust (Axum)
│   ├── Cargo.toml
│   ├── build.sh                     # Compile + génère .deb/.rpm
│   ├── run.sh                       # Lancement dev
│   ├── migrations/
│   │   └── 001_init.sql             # Schéma SQLite (5 tables + triggers)
│   └── src/
│       ├── main.rs                  # Point d'entrée, init, serveur
│       ├── config.rs                # Config depuis variables d'env
│       ├── db.rs                    # Pool SQLite + migrations
│       ├── error.rs                 # AppError → HTTP responses
│       ├── state.rs                 # AppState partagé
│       ├── models/
│       │   ├── session.rs
│       │   ├── message.rs
│       │   ├── setting.rs
│       │   ├── skill.rs
│       │   └── template.rs
│       ├── routes/                  # 38 endpoints
│       │   ├── health.rs
│       │   ├── config_route.rs
│       │   ├── chat.rs              # SSE streaming
│       │   ├── sessions.rs
│       │   ├── files.rs
│       │   ├── shell.rs             # WebSocket PTY
│       │   ├── documents.rs
│       │   ├── models.rs
│       │   ├── skills.rs
│       │   ├── templates.rs
│       │   ├── settings.rs
│       │   ├── mcp.rs
│       │   └── debug.rs
│       ├── services/
│       │   ├── ollama.rs            # Client HTTP SSE (OpenAI/Anthropic)
│       │   ├── document_processor.rs
│       │   └── debug.rs             # Ring buffer
│       └── agents/
│           ├── loop.rs              # Boucle agentique (Stream SSE)
│           └── tools.rs             # 8 outils (read, write, run, grep...)
│
├── frontend/                        # Frontend SvelteKit
│   ├── server.js                    # Serveur custom (SSR + WS proxy)
│   ├── svelte.config.js
│   ├── vite.config.ts
│   ├── src/
│   │   ├── app.html
│   │   ├── app.css
│   │   ├── hooks.server.ts          # Proxy HTTP /api/* → backend
│   │   ├── routes/
│   │   │   ├── +layout.svelte
│   │   │   └── +page.svelte
│   │   └── lib/
│   │       ├── api/index.ts          # Client API typé
│   │       ├── stores/index.ts       # Stores Svelte
│   │       └── components/
│   │           ├── TopBar.svelte
│   │           ├── ChatPanel/
│   │           ├── Terminal/
│   │           ├── FileExplorer/
│   │           ├── SkillSelector/
│   │           ├── Settings/
│   │           └── ...
│   └── static/
│       └── favicon.ico
│
├── docs/                            # Documentation
│   ├── 03-Installation.md
│   ├── 04-Exploitation.md
│   ├── 05-Architecture.md
│   └── 06-SBOM.md
│
└── docker-compose.yml               # Déploiement Docker (legacy Python)
```

---

## Stack technique

### Backend

| Composant | Technologie | Rôle |
|-----------|-------------|------|
| Langage | Rust 2024 edition | Performance, sécurité mémoire |
| Framework web | Axum 0.8 | Routing, middleware, extractors |
| Base de données | SQLite via SQLx 0.8 | Persistance async, migrations |
| Client HTTP | reqwest 0.12 | Communication avec Ollama et MCP |
| SSE streaming | axum::response::Sse | Chat en temps réel |
| WebSocket | axum::extract::ws | Terminal interactif |
| PTY | nix (forkpty) + libc | Pseudo-terminal pour shell |
| Sérialisation | serde + serde_json + serde_yaml | JSON/YAML |
| Logging | tracing + tracing-subscriber | Logs structurés |
| Runtime async | Tokio (full features) | I/O async, timers, process |

### Frontend

| Composant | Technologie | Rôle |
|-----------|-------------|------|
| Framework | SvelteKit 2 + Svelte 5 | UI réactive, SSR |
| Terminal | xterm.js 5.5 | Terminal web interactif |
| Éditeur | Monaco Editor | Édition de code |
| Markdown | marked + highlight.js | Rendu des réponses chat |
| Build | Vite 5 | Bundling |
| Runtime | Node.js 22 (adapter-node) | SSR + WS proxy |

---

## Sécurité

### Principes

- **Isolation** : Toutes les opérations fichiers sont restreintes au workspace
- **Path traversal** : `safe_path()` résout et vérifie que le chemin reste dans le workspace
- **Commandes shell** : Timeout configurable, exécution dans le workspace uniquement
- **Pas de données externes** : Aucun envoi vers des serveurs tiers
- **CORS** : Configuré explicitement
- **SQLite** : WAL mode pour la concurrence

### Matrice des risques MITRE ATT&CK

| Technique | ID | Mitigation |
|-----------|-----|-----------|
| Command Injection | T1059 | Timeout strict, workspace isolé |
| Path Traversal | T1083 | `safe_path()` avec `canonicalize()` |
| Arbitrary File Read | T1005 | Validation chemin, taille max |
| Denial of Service | T1499 | Timeout Ollama, max iterations (20) |
