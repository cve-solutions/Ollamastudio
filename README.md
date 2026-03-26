# OllamaStudio

**v0.0.25**

**Interface web Claude Code-compatible pour Ollama** — 100% local, données sur votre infrastructure.

## Architecture

```
ollamastudio/
├── backend/          # Python 3.12 + FastAPI
│   ├── app/
│   │   ├── agents/   # Boucle agentique + définition des outils
│   │   ├── routers/  # Endpoints REST + WebSocket
│   │   └── services/ # Client Ollama, chunking documents
│   ├── data/         # SQLite, skills YAML, templates YAML
│   ├── workspace/    # Fichiers utilisateur (éditeur + agent)
│   └── documents/    # Documents importés indexés
├── frontend/         # SvelteKit 5 + TypeScript + Monaco
└── docker-compose.yml
```

## Démarrage rapide

### Prérequis

- Docker + Docker Compose
- Ollama installé et démarré sur votre machine ou réseau

### Installation

```bash
# 1. Cloner et configurer
cp .env.example .env
# Éditez .env pour pointer vers votre serveur Ollama

# 2. Lancer les conteneurs
docker compose up -d

# 3. Ouvrir dans le navigateur
open http://localhost:3000
```

### Développement local

```bash
# Backend
cd backend
pip install -e ".[dev]" --break-system-packages
uvicorn app.main:app --reload --port 8000

# Frontend (autre terminal)
cd frontend
npm install
npm run dev
```

## Fonctionnalités

### Chat agentique
- Streaming SSE token par token
- Boucle agentique autonome (tool calling itératif)
- Affichage détaillé des appels d'outils en temps réel
- Historique des sessions (SQLite)

### Outils de l'agent
| Outil | Description |
|-------|-------------|
| `read_file` | Lecture de fichiers dans le workspace |
| `write_file` | Création / modification de fichiers |
| `patch_file` | Modification ciblée (search & replace) |
| `list_files` | Arborescence du workspace |
| `run_command` | Exécution de commandes shell |
| `grep_files` | Recherche regex dans les fichiers |
| `git_status` | Statut du dépôt Git |
| `search_documents` | Recherche dans les documents importés |

### Éditeur de code
- Monaco Editor (même moteur que VS Code)
- Thème OllamaStudio personnalisé
- Multi-onglets avec gestion des modifications
- Sauvegarde Ctrl+S
- Détection automatique du langage (Rust, Python, JS, TS, Go…)

### Terminal PTY
- Terminal complet via WebSocket
- Compatible xterm.js avec rendu couleur 256 couleurs
- Support du redimensionnement (resize)
- Répertoire de travail : `/workspace`

### Skills (Personas)
Fichiers YAML dans `data/skills/` — personnalisent le comportement de l'agent :
- System prompt
- Outils activés (sous-ensemble)
- Température et max_tokens

Skills par défaut : Assistant général, Code Review, DevOps, Expert Rust, Audit Sécurité

### Templates de prompts
Snippets YAML avec variables `{{nom_variable}}` dans `data/templates/`.
Interface de remplissage des variables avant envoi.

### Import de documents (RAG)
- Import par dossier recursif (côté serveur)
- Upload drag-and-drop depuis le navigateur
- Chunking intelligent (par paragraphes pour .md/.txt, par fonctions pour le code)
- Recherche full-text dans les chunks
- Injection automatique dans le contexte via l'outil `search_documents`

### MCP (Model Context Protocol)
- Ajout de serveurs MCP via l'interface
- Test de connectivité (ping)
- Découverte des outils exposés par chaque serveur

## Configuration Ollama

| Mode | Endpoint | Requis |
|------|----------|--------|
| OpenAI (défaut) | `/v1/chat/completions` | Toutes versions |
| Anthropic | `/v1/messages` | Ollama ≥ v0.14.0 |

Le mode Anthropic permet d'utiliser Claude Code CLI avec des modèles locaux :

```bash
export ANTHROPIC_BASE_URL=http://localhost:11434
export ANTHROPIC_AUTH_TOKEN=ollama
claude --model qwen3-coder
```

## Modèles recommandés

```bash
# Meilleur pour le code (30B, 24 Go VRAM)
ollama pull qwen3-coder

# Bon équilibre (32B)
ollama pull qwen2.5-coder:32b

# Plus léger
ollama pull glm-4.7
ollama pull deepseek-coder-v2
```

## Sécurité

- Toutes les opérations fichiers sont confinées au répertoire `workspace/`
- Timeout strict sur les commandes shell
- Commandes sensibles bloquées (`su`, `sudo`, `passwd`…)
- CORS configuré explicitement
- Pas d'envoi de données vers des serveurs externes
