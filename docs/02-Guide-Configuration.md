# Guide de Configuration — OllamaStudio v0.1.0

| Métadonnée        | Valeur                          |
|-------------------|---------------------------------|
| **Version doc**   | 1.0.0                           |
| **Date**          | 2026-03-24                      |
| **Classification**| Interne                         |
| **Auteur**        | Équipe Infrastructure — GoverByte |
| **Approbateur**   | RSSI                            |

---

## Table des matières

1. [Objet du document](#1-objet-du-document)
2. [Vue d'ensemble de la configuration](#2-vue-densemble-de-la-configuration)
3. [Variables d'environnement](#3-variables-denvironnement)
4. [Configuration backend (config.py)](#4-configuration-backend-configpy)
5. [Configuration Docker Compose](#5-configuration-docker-compose)
6. [Configuration frontend](#6-configuration-frontend)
7. [Configuration Ollama](#7-configuration-ollama)
8. [Configuration des Skills (Personas IA)](#8-configuration-des-skills-personas-ia)
9. [Configuration des Templates](#9-configuration-des-templates)
10. [Configuration MCP (Model Context Protocol)](#10-configuration-mcp-model-context-protocol)
11. [Configuration CORS](#11-configuration-cors)
12. [Configuration du RAG (Documents)](#12-configuration-du-rag-documents)
13. [Configuration de sécurité](#13-configuration-de-sécurité)
14. [Bonnes pratiques](#14-bonnes-pratiques)

---

## 1. Objet du document

Ce document détaille l'ensemble des paramètres configurables d'OllamaStudio, leurs valeurs par défaut, et les recommandations pour les environnements de développement, test et production.

## 2. Vue d'ensemble de la configuration

OllamaStudio utilise une approche de configuration en couches :

```
Priorité (haute → basse) :
1. Variables d'environnement système
2. Fichier .env (racine du projet)
3. Valeurs par défaut dans config.py (Pydantic Settings)
```

La gestion centralisée se fait via **Pydantic Settings** dans `backend/app/config.py`.

---

## 3. Variables d'environnement

### 3.1 Référence complète

| Variable              | Type     | Défaut                                            | Description                                      |
|-----------------------|----------|---------------------------------------------------|--------------------------------------------------|
| `OLLAMA_BASE_URL`     | `str`    | `http://localhost:11434`                          | URL du serveur Ollama                            |
| `OLLAMA_API_MODE`     | `str`    | `openai`                                          | Mode API : `openai` ou `anthropic`               |
| `OLLAMA_TIMEOUT`      | `float`  | `120.0`                                           | Timeout des requêtes Ollama (secondes)            |
| `WORKSPACE_ROOT`      | `str`    | `./workspace`                                     | Répertoire de travail de l'agent                  |
| `DATA_DIR`            | `str`    | `./data`                                          | Répertoire des données persistantes               |
| `DOCUMENTS_DIR`       | `str`    | `./documents`                                     | Répertoire des documents RAG                      |
| `CORS_ORIGINS`        | `list`   | `["http://localhost:5173","http://localhost:3000"]`| Origines CORS autorisées (JSON array)            |
| `SHELL_TIMEOUT`       | `int`    | `30`                                              | Timeout des commandes shell (secondes)            |
| `MAX_FILE_SIZE`       | `int`    | `10485760` (10 Mo)                                | Taille maximale des fichiers (octets)             |
| `MAX_AGENT_ITERATIONS`| `int`    | `20`                                              | Nombre max d'itérations de la boucle agentique    |
| `CHUNK_SIZE`          | `int`    | `1000`                                            | Taille des chunks de documents (caractères)       |
| `CHUNK_OVERLAP`       | `int`    | `200`                                             | Chevauchement entre chunks (caractères)           |
| `MAX_CHUNKS_CONTEXT`  | `int`    | `5`                                               | Nombre max de chunks injectés dans le contexte    |
| `PUBLIC_API_URL`      | `str`    | `http://localhost:8000`                           | URL du backend (côté frontend uniquement)         |

### 3.2 Fichier .env.example

```ini
# === Ollama ===
OLLAMA_BASE_URL=http://host.docker.internal:11434

# === CORS ===
CORS_ORIGINS=["http://localhost:3000"]

# === Sécurité ===
SHELL_TIMEOUT=30

# === Frontend ===
PUBLIC_API_URL=http://backend:8000
```

---

## 4. Configuration backend (config.py)

Le fichier `backend/app/config.py` centralise la configuration via une classe Pydantic :

```python
class Settings(BaseSettings):
    # Ollama
    ollama_base_url: str = "http://localhost:11434"
    ollama_api_mode: str = "openai"      # "openai" | "anthropic"
    ollama_timeout: float = 120.0

    # Chemins
    workspace_root: str = "./workspace"
    data_dir: str = "./data"
    documents_dir: str = "./documents"

    # Sécurité
    cors_origins: list[str] = [...]
    shell_timeout: int = 30
    max_file_size: int = 10_485_760

    # Agent
    max_agent_iterations: int = 20

    # RAG
    chunk_size: int = 1000
    chunk_overlap: int = 200
    max_chunks_context: int = 5
```

### 4.1 Accès aux paramètres dans le code

```python
from app.config import settings

url = settings.ollama_base_url
timeout = settings.shell_timeout
```

L'instance `settings` est un singleton chargé au démarrage.

---

## 5. Configuration Docker Compose

### 5.1 Service Backend

```yaml
backend:
  build: ./backend
  ports:
    - "8000:8000"
  volumes:
    - ./backend/data:/app/data           # Base de données, skills, templates
    - ./backend/workspace:/app/workspace # Fichiers utilisateur
    - ./backend/documents:/app/documents # Documents RAG
  environment:
    - OLLAMA_BASE_URL=http://host.docker.internal:11434
    - WORKSPACE_ROOT=/app/workspace
    - DATA_DIR=/app/data
    - DOCUMENTS_DIR=/app/documents
    - CORS_ORIGINS=["http://localhost:3000"]
    - SHELL_TIMEOUT=30
  extra_hosts:
    - "host.docker.internal:host-gateway"
  healthcheck:
    test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
    interval: 15s
    timeout: 5s
    retries: 3
```

### 5.2 Service Frontend

```yaml
frontend:
  build: ./frontend
  ports:
    - "3000:3000"
  environment:
    - PUBLIC_API_URL=http://backend:8000
  depends_on:
    backend:
      condition: service_healthy
```

### 5.3 Personnalisation des ports

Pour utiliser des ports différents, modifier `docker-compose.yml` :

```yaml
# Exemple : frontend sur port 8080, backend sur port 9000
backend:
  ports:
    - "9000:8000"

frontend:
  ports:
    - "8080:3000"
  environment:
    - PUBLIC_API_URL=http://backend:8000  # Port interne inchangé
```

> **Important :** `PUBLIC_API_URL` utilise le réseau Docker interne — le port interne reste 8000.

---

## 6. Configuration frontend

### 6.1 SvelteKit (svelte.config.js)

```javascript
import adapter from '@sveltejs/adapter-node';

export default {
  kit: {
    adapter: adapter()
  }
};
```

L'adaptateur `adapter-node` génère un serveur Node.js pour la production.

### 6.2 Vite (vite.config.ts)

```typescript
export default defineConfig({
  plugins: [sveltekit()],
  server: {
    host: '0.0.0.0',
    port: 5173
  }
});
```

### 6.3 TypeScript (tsconfig.json)

Configuration standard SvelteKit avec strict mode activé.

### 6.4 npm (.npmrc)

```ini
legacy-peer-deps=true
```

Nécessaire pour la compatibilité Monaco Editor avec Svelte 5.

---

## 7. Configuration Ollama

### 7.1 Mode API

OllamaStudio supporte deux modes de communication avec Ollama :

| Mode        | Endpoint                   | Avantages                          |
|-------------|----------------------------|------------------------------------|
| `openai`    | `/v1/chat/completions`     | Compatibilité large, défaut       |
| `anthropic` | `/v1/messages`             | Format natif Claude, tool_use     |

Configurer via :
- Variable : `OLLAMA_API_MODE=anthropic`
- Interface : Menu Settings dans l'UI

### 7.2 Timeout

Pour les modèles lents ou les longues générations :

```bash
OLLAMA_TIMEOUT=300  # 5 minutes
```

### 7.3 URL selon l'environnement

| Environnement              | `OLLAMA_BASE_URL`                            |
|----------------------------|----------------------------------------------|
| Docker (Docker Desktop)    | `http://host.docker.internal:11434`          |
| Docker (Linux natif)       | `http://host.docker.internal:11434` (avec extra_hosts) |
| Développement local        | `http://localhost:11434`                     |
| Ollama sur serveur distant | `http://<ip-serveur>:11434`                  |

---

## 8. Configuration des Skills (Personas IA)

Les Skills définissent des personas IA avec des comportements spécialisés.

### 8.1 Stockage

```
backend/data/skills/
├── general-assistant.yaml
├── code-review.yaml
├── devops-infra.yaml
├── expert-rust.yaml
└── security-audit.yaml
```

### 8.2 Format YAML

```yaml
name: "Security Audit"
description: "Expert en sécurité et audit de code"
icon: "🔒"
color: "#dc2626"
system_prompt: |
  Tu es un expert en cybersécurité spécialisé dans l'audit de code.
  Tu identifies les vulnérabilités OWASP Top 10, les failles d'injection,
  les problèmes d'authentification et de contrôle d'accès.
tools:
  - read_file
  - grep_files
  - list_files
temperature: 0.3
max_tokens: 4096
```

### 8.3 Paramètres de skill

| Champ           | Type       | Requis | Description                                    |
|-----------------|------------|--------|------------------------------------------------|
| `name`          | `str`      | Oui    | Nom affiché                                    |
| `description`   | `str`      | Oui    | Description courte                             |
| `icon`          | `str`      | Non    | Emoji ou icône                                 |
| `color`         | `str`      | Non    | Couleur hexadécimale                           |
| `system_prompt` | `str`      | Oui    | Prompt système envoyé au LLM                   |
| `tools`         | `list[str]`| Non    | Sous-ensemble d'outils autorisés               |
| `temperature`   | `float`    | Non    | Température de génération (0.0–2.0)            |
| `max_tokens`    | `int`      | Non    | Limite de tokens par réponse                   |

### 8.4 Outils disponibles pour les skills

| Outil               | Description                                |
|----------------------|--------------------------------------------|
| `read_file`          | Lire le contenu d'un fichier               |
| `write_file`         | Écrire/créer un fichier                    |
| `patch_file`         | Appliquer des modifications ciblées        |
| `list_files`         | Lister les fichiers d'un répertoire        |
| `run_command`        | Exécuter une commande shell                |
| `grep_files`         | Rechercher dans les fichiers               |
| `git_status`         | Obtenir le statut Git du workspace         |
| `search_documents`   | Rechercher dans les documents RAG          |

---

## 9. Configuration des Templates

### 9.1 Stockage

```
backend/data/templates/
```

### 9.2 Templates par défaut

| Template                | Variables          | Usage                        |
|-------------------------|--------------------|------------------------------|
| Expliquer du code       | `{code}`, `{lang}` | Explication pédagogique      |
| Refactoring             | `{code}`, `{lang}` | Amélioration de code         |
| Écrire des tests        | `{code}`, `{lang}` | Génération de tests          |
| Optimiser Dockerfile    | `{dockerfile}`      | Optimisation d'images Docker |
| Revue de sécurité       | `{code}`, `{lang}` | Audit sécurité              |
| Message de commit Git   | `{diff}`            | Rédaction de commits         |
| Documentation API       | `{code}`, `{lang}` | Génération de docs API       |
| Débugger                | `{code}`, `{error}` | Analyse d'erreurs           |

### 9.3 Format YAML d'un template

```yaml
name: "Revue de sécurité"
description: "Analyser un extrait de code pour les vulnérabilités"
prompt: |
  Analyse le code suivant ({lang}) pour les vulnérabilités de sécurité.
  Identifie les failles OWASP Top 10 et propose des corrections.

  ```{lang}
  {code}
  ```
variables:
  - name: "code"
    label: "Code à analyser"
    type: "textarea"
  - name: "lang"
    label: "Langage"
    type: "text"
    default: "python"
```

---

## 10. Configuration MCP (Model Context Protocol)

### 10.1 Stockage

```
backend/data/mcp_servers.yaml
```

### 10.2 Serveurs MCP par défaut

| Serveur          | URL                              | Description                    |
|------------------|----------------------------------|--------------------------------|
| Filesystem MCP   | Configuration locale             | Accès fichiers système         |
| Git MCP          | Configuration locale             | Opérations Git                 |
| DataGouv.FR      | API data.gouv.fr                 | Données ouvertes françaises    |

### 10.3 Ajouter un serveur MCP

Via l'interface (onglet MCP) ou directement dans le YAML :

```yaml
servers:
  - id: "mon-serveur"
    name: "Mon Serveur MCP"
    url: "http://localhost:9090"
    enabled: true
    description: "Serveur MCP personnalisé"
```

### 10.4 Paramètres MCP

| Champ         | Type   | Requis | Description                  |
|---------------|--------|--------|------------------------------|
| `id`          | `str`  | Oui    | Identifiant unique           |
| `name`        | `str`  | Oui    | Nom affiché                  |
| `url`         | `str`  | Oui    | URL HTTP du serveur          |
| `enabled`     | `bool` | Non    | Actif/inactif (défaut: true) |
| `description` | `str`  | Non    | Description                  |

---

## 11. Configuration CORS

### 11.1 Configuration par défaut

```python
CORS_ORIGINS=["http://localhost:5173", "http://localhost:3000"]
```

### 11.2 Personnalisation

```bash
# Autoriser un domaine spécifique
CORS_ORIGINS='["https://studio.mondomaine.com"]'

# Multiples origines
CORS_ORIGINS='["http://localhost:3000","https://studio.mondomaine.com"]'
```

> **Sécurité :** Ne **jamais** utiliser `["*"]` en production. Spécifier explicitement les origines autorisées.

### 11.3 Middleware CORS appliqué

```python
app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origins,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)
```

---

## 12. Configuration du RAG (Documents)

### 12.1 Paramètres de chunking

| Paramètre            | Défaut | Description                                    |
|----------------------|--------|------------------------------------------------|
| `CHUNK_SIZE`         | 1000   | Taille cible d'un chunk (caractères)           |
| `CHUNK_OVERLAP`      | 200    | Chevauchement entre chunks consécutifs          |
| `MAX_CHUNKS_CONTEXT` | 5      | Nombre max de chunks injectés par requête       |

### 12.2 Formats de fichiers supportés

```
Texte :    .txt, .md, .rst
Code :     .py, .rs, .go, .js, .ts, .java, .c, .cpp, .h
Config :   .yaml, .yml, .json, .toml, .ini
Web :      .html, .css, .xml
Data :     .sql, .csv
DevOps :   .dockerfile, Dockerfile, .sh
```

### 12.3 Ajustement pour les gros corpus

Pour des corpus volumineux (>1000 documents) :

```bash
CHUNK_SIZE=500            # Chunks plus petits, plus précis
CHUNK_OVERLAP=100         # Moins de redondance
MAX_CHUNKS_CONTEXT=10     # Plus de contexte injecté
```

---

## 13. Configuration de sécurité

### 13.1 Commandes shell bloquées

Les commandes suivantes sont interdites dans le terminal et l'outil `run_command` :

```
su, sudo, passwd, adduser, useradd, chsh
```

### 13.2 Limites de sécurité

| Paramètre              | Valeur        | Configurable | Description                     |
|------------------------|---------------|-------------- |---------------------------------|
| Shell timeout          | 30s           | Oui          | Timeout par commande            |
| Max output shell       | 65 Ko         | Non          | Taille max sortie commande      |
| Max file size          | 10 Mo         | Oui          | Taille max lecture/écriture     |
| Max agent iterations   | 20            | Oui          | Boucle agentique max            |
| Path traversal         | Bloqué        | Non          | Vérification _safe_path()       |

### 13.3 Isolation des chemins

Tous les accès fichiers sont confinés à :
- `WORKSPACE_ROOT` pour les opérations fichiers
- `DOCUMENTS_DIR` pour les documents RAG
- `DATA_DIR` pour les données internes

---

## 14. Bonnes pratiques

### 14.1 Production

- Définir `CORS_ORIGINS` uniquement avec les domaines autorisés
- Réduire `SHELL_TIMEOUT` au minimum nécessaire
- Monter les volumes avec des permissions restreintes
- Placer un reverse proxy (Nginx/Traefik) devant les services
- Activer HTTPS via le reverse proxy
- Ne pas exposer le port 8000 directement

### 14.2 Développement

- Utiliser `.env` pour centraliser la configuration
- Ne pas modifier `config.py` pour des valeurs spécifiques à un environnement
- Utiliser `--reload` pour le backend en développement

### 14.3 Sauvegardes

Éléments à sauvegarder régulièrement :
- `backend/data/ollamastudio.db` — Base de données (sessions, messages)
- `backend/data/skills/` — Skills personnalisés
- `backend/data/templates/` — Templates personnalisés
- `backend/data/mcp_servers.yaml` — Configuration MCP
- `backend/workspace/` — Fichiers de travail

---

*Fin du document — Guide de Configuration OllamaStudio v0.1.0*
