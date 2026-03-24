# Guide d'Installation — OllamaStudio v0.1.0

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
2. [Public visé](#2-public-visé)
3. [Prérequis](#3-prérequis)
4. [Installation par Docker Compose (recommandé)](#4-installation-par-docker-compose-recommandé)
5. [Installation locale (développement)](#5-installation-locale-développement)
6. [Vérification post-installation](#6-vérification-post-installation)
7. [Mise à jour](#7-mise-à-jour)
8. [Désinstallation](#8-désinstallation)
9. [Dépannage](#9-dépannage)

---

## 1. Objet du document

Ce document décrit les procédures d'installation d'**OllamaStudio**, interface web IDE pour modèles de langage locaux via Ollama. Il couvre les déploiements Docker (production) et local (développement).

## 2. Public visé

- Administrateurs systèmes
- Ingénieurs DevOps
- Développeurs contribuant au projet

## 3. Prérequis

### 3.1 Matériel minimum

| Composant   | Minimum        | Recommandé       |
|-------------|----------------|-------------------|
| CPU         | 4 cœurs        | 8 cœurs           |
| RAM         | 8 Go           | 16 Go+            |
| Disque      | 20 Go libres   | 50 Go+ (modèles)  |
| GPU         | Optionnel      | NVIDIA 8 Go+ VRAM |

> **Note :** Les exigences GPU dépendent des modèles Ollama utilisés, pas d'OllamaStudio lui-même.

### 3.2 Logiciels requis

| Logiciel         | Version minimale | Vérification                  |
|------------------|------------------|-------------------------------|
| Docker Engine    | 20.10+           | `docker --version`            |
| Docker Compose   | 1.29+ / v2       | `docker compose version`      |
| Ollama           | 0.3.0+           | `ollama --version`            |
| Git              | 2.30+            | `git --version`               |

**Pour le développement local uniquement :**

| Logiciel | Version minimale | Vérification       |
|----------|------------------|---------------------|
| Python   | 3.11+            | `python3 --version` |
| Node.js  | 22+              | `node --version`    |
| npm      | 9+               | `npm --version`     |

### 3.3 Réseau

| Port  | Service            | Direction |
|-------|--------------------|-----------|
| 3000  | Frontend SvelteKit | Entrant   |
| 8000  | Backend FastAPI     | Entrant   |
| 11434 | Ollama Server      | Sortant   |

### 3.4 Prérequis Ollama

Ollama doit être installé et démarré **avant** OllamaStudio :

```bash
# Installation Ollama (Linux)
curl -fsSL https://ollama.com/install.sh | sh

# Démarrage du service
systemctl start ollama
# ou
ollama serve

# Télécharger au moins un modèle
ollama pull qwen2.5-coder:7b
```

**Modèles recommandés :**

| Modèle                | Taille  | Usage principal           |
|-----------------------|---------|---------------------------|
| qwen2.5-coder:7b     | ~4.7 Go | Code, usage général       |
| qwen3-coder           | ~4.7 Go | Code avancé, tool-calling |
| deepseek-coder-v2:16b | ~8.9 Go | Code haute performance    |
| glm-4:9b             | ~5.5 Go | Multilingue               |

---

## 4. Installation par Docker Compose (recommandé)

### 4.1 Cloner le dépôt

```bash
git clone https://github.com/cve-solutions/ollamastudio.git
cd ollamastudio
```

### 4.2 Configurer l'environnement

```bash
cp .env.example .env
```

Éditer `.env` selon votre environnement :

```ini
# URL d'accès à Ollama depuis les conteneurs Docker
OLLAMA_BASE_URL=http://host.docker.internal:11434

# Origines CORS autorisées
CORS_ORIGINS=["http://localhost:3000"]

# Timeout des commandes shell (secondes)
SHELL_TIMEOUT=30
```

> **Important :** Sur Linux natif (pas Docker Desktop), `host.docker.internal` peut ne pas être résolu automatiquement. Le fichier `docker-compose.yml` inclut déjà la directive `extra_hosts` nécessaire.

### 4.3 Construire et démarrer

```bash
# Construction des images et démarrage
docker compose up -d --build

# Suivre les logs
docker compose logs -f
```

### 4.4 Vérifier le déploiement

```bash
# Santé du backend
curl http://localhost:8000/health
# Réponse attendue : {"status": "ok", "version": "0.1.0"}

# Accéder à l'interface
# Ouvrir http://localhost:3000 dans un navigateur
```

### 4.5 Structure des volumes persistants

```
./backend/data/         → Base SQLite, skills YAML, templates, config MCP
./backend/workspace/    → Fichiers utilisateur (zone de travail de l'agent)
./backend/documents/    → Documents importés pour le RAG
```

---

## 5. Installation locale (développement)

### 5.1 Backend

```bash
cd backend

# Créer un environnement virtuel (recommandé)
python3 -m venv .venv
source .venv/bin/activate

# Installer les dépendances (avec outils de développement)
pip install -e ".[dev]"

# Configurer l'environnement
export OLLAMA_BASE_URL=http://localhost:11434
export WORKSPACE_ROOT=$(pwd)/workspace
export DATA_DIR=$(pwd)/data
export DOCUMENTS_DIR=$(pwd)/documents

# Créer les répertoires
mkdir -p workspace data documents

# Démarrer le serveur en mode rechargement automatique
uvicorn app.main:app --reload --host 0.0.0.0 --port 8000
```

### 5.2 Frontend

Dans un second terminal :

```bash
cd frontend

# Installer les dépendances Node.js
npm install

# Configurer l'URL du backend
export PUBLIC_API_URL=http://localhost:8000

# Démarrer le serveur de développement
npm run dev
```

Le frontend de développement est accessible sur `http://localhost:5173`.

### 5.3 Variables d'environnement (développement)

| Variable           | Défaut                         | Description                    |
|--------------------|--------------------------------|--------------------------------|
| `OLLAMA_BASE_URL`  | `http://localhost:11434`       | URL du serveur Ollama          |
| `WORKSPACE_ROOT`   | `./workspace`                  | Répertoire de travail agent    |
| `DATA_DIR`         | `./data`                       | Données persistantes           |
| `DOCUMENTS_DIR`    | `./documents`                  | Documents RAG                  |
| `CORS_ORIGINS`     | `["http://localhost:5173",...]`| Origines CORS                  |
| `SHELL_TIMEOUT`    | `30`                           | Timeout commandes shell (sec)  |
| `PUBLIC_API_URL`   | `http://localhost:8000`        | URL backend (côté frontend)    |

---

## 6. Vérification post-installation

### 6.1 Checklist de validation

| # | Vérification                                | Commande / Action                              | Résultat attendu              |
|---|---------------------------------------------|------------------------------------------------|-------------------------------|
| 1 | Backend en ligne                            | `curl http://localhost:8000/health`            | `{"status": "ok"}`           |
| 2 | Frontend accessible                         | Ouvrir `http://localhost:3000`                  | Page de chat affichée         |
| 3 | Modèles Ollama visibles                     | Menu déroulant des modèles dans l'UI           | Liste des modèles disponibles |
| 4 | Chat fonctionnel                            | Envoyer un message de test                      | Réponse en streaming          |
| 5 | Terminal opérationnel                       | Ouvrir l'onglet Terminal                        | Shell interactif              |
| 6 | Éditeur de code                             | Ouvrir un fichier dans l'éditeur                | Syntaxe colorée, édition OK   |
| 7 | Import de documents                         | Importer un fichier .txt ou .md                 | Document indexé, recherchable |

### 6.2 Commandes de diagnostic

```bash
# État des conteneurs
docker compose ps

# Logs backend
docker compose logs backend --tail 50

# Logs frontend
docker compose logs frontend --tail 50

# Connectivité Ollama depuis le backend
docker compose exec backend curl http://host.docker.internal:11434/api/tags
```

---

## 7. Mise à jour

```bash
# Arrêter les services
docker compose down

# Récupérer les dernières modifications
git pull origin main

# Reconstruire et redémarrer
docker compose up -d --build
```

> **Note :** Les données persistantes (SQLite, workspace, documents) sont conservées dans les volumes montés et ne sont pas affectées par la mise à jour.

---

## 8. Désinstallation

```bash
# Arrêter et supprimer les conteneurs
docker compose down

# Supprimer les images construites
docker rmi ollamastudio-backend ollamastudio-frontend

# (Optionnel) Supprimer les données persistantes
rm -rf backend/data backend/workspace backend/documents
```

---

## 9. Dépannage

### 9.1 Le backend ne démarre pas

| Symptôme                        | Cause probable                  | Solution                                    |
|---------------------------------|---------------------------------|---------------------------------------------|
| `Connection refused` port 8000  | Conteneur non démarré           | `docker compose up -d backend`              |
| `ModuleNotFoundError`           | Dépendances manquantes          | `pip install -e ".[dev]"`                   |
| `Permission denied` sur data/   | Droits fichiers incorrects      | `chmod -R 755 backend/data`                 |

### 9.2 Le frontend ne se connecte pas au backend

| Symptôme                         | Cause probable                 | Solution                                     |
|----------------------------------|--------------------------------|----------------------------------------------|
| `fetch failed` dans la console   | `PUBLIC_API_URL` incorrect     | Vérifier la variable d'environnement          |
| Erreur CORS dans la console      | Origine non autorisée          | Ajouter l'origine dans `CORS_ORIGINS`        |
| `502 Bad Gateway`                | Backend non prêt               | Attendre le health check ou relancer          |

### 9.3 Ollama inaccessible

| Symptôme                               | Cause probable                   | Solution                                        |
|----------------------------------------|----------------------------------|-------------------------------------------------|
| `Connection refused` port 11434        | Ollama non démarré               | `systemctl start ollama` ou `ollama serve`      |
| `host.docker.internal` non résolu      | Linux sans Docker Desktop        | Vérifier `extra_hosts` dans docker-compose.yml  |
| Aucun modèle affiché                   | Aucun modèle téléchargé          | `ollama pull qwen2.5-coder:7b`                  |

---

*Fin du document — Guide d'Installation OllamaStudio v0.1.0*
