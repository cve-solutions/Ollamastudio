# OllamaStudio — Guide d'installation

## Prérequis

### Système

| Composant | Minimum | Recommandé |
|-----------|---------|------------|
| OS | Linux x86_64 / arm64 | Ubuntu 22.04+, RHEL 9+, Fedora 38+ |
| RAM | 2 Go | 8 Go+ (dépend du modèle Ollama) |
| Disque | 500 Mo (app) | 20 Go+ (modèles Ollama) |
| CPU | 2 cores | 4+ cores |

### Logiciels requis

| Logiciel | Version | Rôle |
|----------|---------|------|
| [Ollama](https://ollama.ai) | 0.3+ | Serveur LLM local |
| [Rust](https://rustup.rs) | 1.80+ | Compilation du backend (si build depuis sources) |
| [Node.js](https://nodejs.org) | 22+ | Build du frontend |
| SQLite | 3.35+ | Base de données (inclus via sqlx) |
| OpenSSL | 1.1+ / 3.x | TLS pour les requêtes HTTP |

---

## Installation rapide

### 1. Installer Ollama

```bash
curl -fsSL https://ollama.ai/install.sh | sh
ollama pull qwen3-coder    # ou tout autre modèle
```

### 2. Cloner le projet

```bash
git clone https://github.com/cve-solutions/ollamastudio.git
cd ollamastudio
```

### 3. Build et installation du backend (Rust)

```bash
cd backend-rs
./build.sh
```

Le script `build.sh` :
- Installe les dépendances système (`libsqlite3-dev`, `libssl-dev`)
- Compile le binaire en mode release
- Génère un package `.deb` et/ou `.rpm`

#### Installation via package

```bash
# Debian / Ubuntu
sudo dpkg -i dist/ollamastudio-backend_*_amd64.deb

# RHEL / Fedora
sudo rpm -i dist/ollamastudio-backend-*-1.x86_64.rpm
```

#### Installation manuelle (sans package)

```bash
# Copier le binaire
sudo cp target/release/ollamastudio-backend /usr/local/bin/

# Créer les répertoires
sudo mkdir -p /var/lib/ollamastudio/{data/skills,data/templates,workspace,documents}

# Créer le fichier de config
sudo mkdir -p /etc/ollamastudio
cat <<EOF | sudo tee /etc/ollamastudio/env
WORKSPACE_ROOT=/var/lib/ollamastudio/workspace
DATA_DIR=/var/lib/ollamastudio/data
DOCUMENTS_DIR=/var/lib/ollamastudio/documents
APP_VERSION=$(cat ../VERSION)
LOG_LEVEL=info
PORT=8000
EOF
```

### 4. Build du frontend

```bash
cd ../frontend
npm install
npm run build
```

### 5. Lancer le frontend (production)

```bash
# Le serveur custom gère SvelteKit + proxy WebSocket
BACKEND_URL=http://localhost:8000 PORT=3000 node server.js
```

---

## Démarrage avec systemd

### Backend

Le package `.deb`/`.rpm` installe automatiquement le service :

```bash
sudo systemctl start ollamastudio
sudo systemctl enable ollamastudio   # démarrage auto au boot
sudo systemctl status ollamastudio
```

### Frontend (service manuel)

```bash
cat <<EOF | sudo tee /lib/systemd/system/ollamastudio-frontend.service
[Unit]
Description=OllamaStudio Frontend
After=ollamastudio.service

[Service]
Type=simple
WorkingDirectory=/opt/ollamastudio/frontend
Environment=BACKEND_URL=http://localhost:8000
Environment=PORT=3000
Environment=HOST=0.0.0.0
ExecStart=/usr/bin/node server.js
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl start ollamastudio-frontend
sudo systemctl enable ollamastudio-frontend
```

---

## Vérification

```bash
# Backend
curl http://localhost:8000/health
# → {"status":"ok","version":"0.0.15"}

# Frontend
curl -s http://localhost:3000/health
# → {"status":"ok","version":"0.0.15"} (proxifié)

# Interface web
# Ouvrir http://<IP_SERVEUR>:3000 dans un navigateur
```

---

## Configuration réseau

| Service | Port | Accès |
|---------|------|-------|
| Backend API | 8000 | Interne uniquement (pas exposé) |
| Frontend | 3000 | Navigateur client |
| Ollama | 11434 | Interne (configurable via UI) |

Le frontend proxy toutes les requêtes `/api/*` et les WebSocket vers le backend. Seul le port **3000** doit être ouvert dans le firewall.

```bash
# Firewall (firewalld)
sudo firewall-cmd --permanent --add-port=3000/tcp
sudo firewall-cmd --reload

# Firewall (ufw)
sudo ufw allow 3000/tcp
```

---

## Mise à jour

```bash
cd ollamastudio
git pull

# Backend
cd backend-rs
./build.sh
sudo dpkg -i dist/ollamastudio-backend_*.deb   # ou rpm
sudo systemctl restart ollamastudio

# Frontend
cd ../frontend
npm install && npm run build
sudo systemctl restart ollamastudio-frontend
```

---

## Désinstallation

```bash
# Arrêt des services
sudo systemctl stop ollamastudio ollamastudio-frontend
sudo systemctl disable ollamastudio ollamastudio-frontend

# Suppression du package
sudo dpkg -r ollamastudio-backend   # Debian
sudo rpm -e ollamastudio-backend    # RHEL

# Nettoyage des données (ATTENTION : irréversible)
sudo rm -rf /var/lib/ollamastudio
sudo rm -rf /etc/ollamastudio
```
