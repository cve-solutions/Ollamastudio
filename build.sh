#!/bin/bash
set -e

# ═══════════════════════════════════════════════════════════════════════
# OllamaStudio — Build & Package complet (.deb / .rpm)
#
# Compile le backend Rust, build le frontend Node, et génère des
# packages installables avec TOUTES les dépendances incluses.
#
# Usage :
#   ./build.sh              # Build tout + packages
#   ./build.sh --deb-only   # Seulement .deb
#   ./build.sh --rpm-only   # Seulement .rpm
#   ./build.sh --skip-frontend  # Ne rebuild pas le frontend
# ═══════════════════════════════════════════════════════════════════════

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT_DIR"

VERSION="$(cat VERSION 2>/dev/null || echo '0.0.16')"
APP_NAME="ollamastudio"
DESCRIPTION="OllamaStudio — Interface web IA locale compatible Claude Code pour Ollama"
MAINTAINER="GoverByte <contact@goverbyte.com>"
URL="https://github.com/cve-solutions/ollamastudio"
LICENSE="Proprietary"

ARCH="$(dpkg --print-architecture 2>/dev/null || uname -m)"
case "$ARCH" in
    x86_64|amd64)  ARCH="amd64"; RPM_ARCH="x86_64" ;;
    aarch64|arm64)  ARCH="arm64"; RPM_ARCH="aarch64" ;;
    *)              RPM_ARCH="$ARCH" ;;
esac

DIST_DIR="$ROOT_DIR/dist"
BACKEND_BIN="backend-rs/target/release/ollamastudio-backend"
AGENT_BIN="agent-mcp/target/release/agent-mcp"
FRONTEND_BUILD="frontend/build"

SKIP_FRONTEND=false
DEB_ONLY=false
RPM_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --skip-frontend) SKIP_FRONTEND=true ;;
        --deb-only)      DEB_ONLY=true ;;
        --rpm-only)      RPM_ONLY=true ;;
    esac
done

# ── Couleurs ──────────────────────────────────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; BLUE='\033[0;34m'; YELLOW='\033[0;33m'; NC='\033[0m'
info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}  [OK]${NC} $*"; }
warn()  { echo -e "${YELLOW}[WARN]${NC} $*"; }
fail()  { echo -e "${RED}[ERREUR]${NC} $*"; exit 1; }
step()  { echo -e "\n${GREEN}══ $* ══${NC}"; }

echo ""
echo "╔═══════════════════════════════════════════════════════════╗"
echo "║  OllamaStudio Build — v$VERSION ($ARCH)"
echo "╚═══════════════════════════════════════════════════════════╝"

# ═══════════════════════════════════════════════════════════════════════
# 1. DÉPENDANCES DE BUILD
# ═══════════════════════════════════════════════════════════════════════
step "1/5 — Vérification des dépendances de build"

install_build_deps_deb() {
    info "Installation des dépendances de build (Debian/Ubuntu)..."
    sudo apt-get update -qq
    sudo apt-get install -y -qq \
        build-essential pkg-config curl \
        libsqlite3-dev libssl-dev \
        dpkg-dev 2>/dev/null || true
}

install_build_deps_rpm() {
    local PM="$1"
    info "Installation des dépendances de build (RPM)..."
    sudo "$PM" install -y -q \
        gcc make pkg-config curl \
        sqlite-devel openssl-devel \
        rpm-build 2>/dev/null || true
}

# Rust
if ! command -v cargo >/dev/null 2>&1; then
    info "Installation de Rust via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
fi
command -v cargo >/dev/null 2>&1 || fail "cargo introuvable après installation"
ok "Rust $(rustc --version | awk '{print $2}')"

# Node.js
if ! command -v node >/dev/null 2>&1; then
    fail "Node.js non trouvé. Installez Node.js 22+ : https://nodejs.org"
fi
ok "Node.js $(node --version)"

# Libs système
if ! pkg-config --exists sqlite3 2>/dev/null || ! pkg-config --exists openssl 2>/dev/null; then
    if command -v apt-get >/dev/null 2>&1; then
        install_build_deps_deb
    elif command -v dnf >/dev/null 2>&1; then
        install_build_deps_rpm dnf
    elif command -v yum >/dev/null 2>&1; then
        install_build_deps_rpm yum
    else
        warn "Impossible de détecter le gestionnaire de paquets. Assurez-vous que libsqlite3-dev et libssl-dev sont installés."
    fi
fi
ok "Bibliothèques système (sqlite3, openssl)"

# ═══════════════════════════════════════════════════════════════════════
# 2. COMPILATION BACKEND RUST
# ═══════════════════════════════════════════════════════════════════════
step "2/5 — Compilation du backend Rust (release)"

cd "$ROOT_DIR/backend-rs"
cargo build --release 2>&1 | grep -E "Compiling|Finished|warning\[" | tail -5

if [ ! -f "$ROOT_DIR/$BACKEND_BIN" ]; then
    fail "Binaire backend non trouvé: $BACKEND_BIN"
fi
ok "Backend compilé: $(du -h "$ROOT_DIR/$BACKEND_BIN" | cut -f1)"

# ═══════════════════════════════════════════════════════════════════════
# 2b. COMPILATION AGENT MCP (Rust)
# ═══════════════════════════════════════════════════════════════════════
step "2b/6 — Compilation de l'Agent MCP (release)"

cd "$ROOT_DIR/agent-mcp"
cargo build --release 2>&1 | grep -E "Compiling|Finished|warning\[" | tail -5

if [ ! -f "$ROOT_DIR/$AGENT_BIN" ]; then
    fail "Binaire agent-mcp non trouvé: $AGENT_BIN"
fi
ok "Agent MCP compilé: $(du -h "$ROOT_DIR/$AGENT_BIN" | cut -f1)"

# ═══════════════════════════════════════════════════════════════════════
# 3. BUILD FRONTEND
# ═══════════════════════════════════════════════════════════════════════
if [ "$SKIP_FRONTEND" = false ]; then
    step "3/5 — Build du frontend SvelteKit"

    cd "$ROOT_DIR/frontend"
    npm install --silent 2>/dev/null
    npm run build 2>&1 | tail -3

    if [ ! -d "$ROOT_DIR/$FRONTEND_BUILD" ]; then
        fail "Build frontend non trouvé: $FRONTEND_BUILD"
    fi
    ok "Frontend compilé"
else
    step "3/5 — Frontend (skip)"
    warn "Frontend non recompilé (--skip-frontend)"
fi

cd "$ROOT_DIR"

# ═══════════════════════════════════════════════════════════════════════
# 4. PRÉPARATION COMMUNE
# ═══════════════════════════════════════════════════════════════════════
step "4/5 — Préparation des packages"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

# Fonction commune pour peupler un dossier d'installation
populate_install_tree() {
    local PREFIX="$1"

    # Backend
    mkdir -p "$PREFIX/usr/bin"
    cp "$ROOT_DIR/$BACKEND_BIN" "$PREFIX/usr/bin/ollamastudio-backend"
    chmod 755 "$PREFIX/usr/bin/ollamastudio-backend"
    strip "$PREFIX/usr/bin/ollamastudio-backend" 2>/dev/null || true

    # Frontend (application complète)
    mkdir -p "$PREFIX/opt/ollamastudio/frontend"
    cp -r "$ROOT_DIR/$FRONTEND_BUILD" "$PREFIX/opt/ollamastudio/frontend/build"
    cp "$ROOT_DIR/frontend/package.json" "$PREFIX/opt/ollamastudio/frontend/"
    cp "$ROOT_DIR/frontend/server.js" "$PREFIX/opt/ollamastudio/frontend/"

    # Node modules (production seulement)
    cd "$ROOT_DIR/frontend"
    mkdir -p "$PREFIX/opt/ollamastudio/frontend/node_modules"
    # Copie node_modules — en prod on n'a besoin que des deps runtime
    if [ -d node_modules ]; then
        cp -r node_modules "$PREFIX/opt/ollamastudio/frontend/"
    fi
    cd "$ROOT_DIR"

    # Données
    mkdir -p "$PREFIX/var/lib/ollamastudio/data/skills"
    mkdir -p "$PREFIX/var/lib/ollamastudio/data/templates"
    mkdir -p "$PREFIX/var/lib/ollamastudio/data/mcp"
    mkdir -p "$PREFIX/var/lib/ollamastudio/workspace"
    mkdir -p "$PREFIX/var/lib/ollamastudio/documents"

    # Configuration
    mkdir -p "$PREFIX/etc/ollamastudio"
    cat > "$PREFIX/etc/ollamastudio/backend.env" <<ENVEOF
# OllamaStudio Backend Configuration
WORKSPACE_ROOT=/var/lib/ollamastudio/workspace
DATA_DIR=/var/lib/ollamastudio/data
DOCUMENTS_DIR=/var/lib/ollamastudio/documents
APP_VERSION=$VERSION
LOG_LEVEL=info
PORT=8000
ENVEOF

    cat > "$PREFIX/etc/ollamastudio/frontend.env" <<ENVEOF
# OllamaStudio Frontend Configuration
BACKEND_URL=http://127.0.0.1:8000
HOST=127.0.0.1
PORT=3000
ENVEOF

    # Nginx config
    mkdir -p "$PREFIX/etc/nginx/sites-available"
    mkdir -p "$PREFIX/etc/nginx/sites-enabled"
    cp "$ROOT_DIR/nginx/ollamastudio.conf" "$PREFIX/etc/nginx/sites-available/ollamastudio"

    # Script de génération SSL
    mkdir -p "$PREFIX/usr/lib/ollamastudio"
    cp "$ROOT_DIR/nginx/generate-ssl.sh" "$PREFIX/usr/lib/ollamastudio/generate-ssl.sh"
    chmod 755 "$PREFIX/usr/lib/ollamastudio/generate-ssl.sh"

    # Répertoire SSL (sera peuplé au postinst)
    mkdir -p "$PREFIX/etc/ollamastudio/ssl"

    # Services systemd
    local SYSTEMD_DIR
    if [ -d "$PREFIX/usr/lib/systemd/system" ] || echo "$PREFIX" | grep -q rpm; then
        SYSTEMD_DIR="$PREFIX/usr/lib/systemd/system"
    else
        SYSTEMD_DIR="$PREFIX/lib/systemd/system"
    fi
    mkdir -p "$SYSTEMD_DIR"

    cat > "$SYSTEMD_DIR/ollamastudio-backend.service" <<'SVCEOF'
[Unit]
Description=OllamaStudio Backend API (Rust)
After=network.target
Wants=ollamastudio-frontend.service

[Service]
Type=simple
User=ollamastudio
Group=ollamastudio
EnvironmentFile=/etc/ollamastudio/backend.env
ExecStart=/usr/bin/ollamastudio-backend
Restart=on-failure
RestartSec=5
WorkingDirectory=/var/lib/ollamastudio

[Install]
WantedBy=multi-user.target
SVCEOF

    cat > "$SYSTEMD_DIR/ollamastudio-frontend.service" <<'SVCEOF'
[Unit]
Description=OllamaStudio Frontend (Node.js)
After=ollamastudio-backend.service
Requires=ollamastudio-backend.service

[Service]
Type=simple
User=ollamastudio
Group=ollamastudio
EnvironmentFile=/etc/ollamastudio/frontend.env
ExecStart=/usr/bin/node /opt/ollamastudio/frontend/server.js
Restart=on-failure
RestartSec=5
WorkingDirectory=/opt/ollamastudio/frontend

[Install]
WantedBy=multi-user.target
SVCEOF

    # Script de contrôle
    mkdir -p "$PREFIX/usr/bin"
    cat > "$PREFIX/usr/bin/ollamastudio-ctl" <<'CTLEOF'
#!/bin/bash
SERVICES="ollamastudio-backend ollamastudio-frontend nginx"

case "${1:-status}" in
    start)
        sudo systemctl start $SERVICES
        echo "OllamaStudio démarré — https://$(hostname)"
        ;;
    stop)
        sudo systemctl stop ollamastudio-frontend ollamastudio-backend
        echo "OllamaStudio arrêté (nginx reste actif)"
        ;;
    restart)
        sudo systemctl restart $SERVICES
        echo "OllamaStudio redémarré — https://$(hostname)"
        ;;
    status)
        systemctl status $SERVICES --no-pager 2>/dev/null
        ;;
    logs)
        journalctl -u ollamastudio-backend -u ollamastudio-frontend -f
        ;;
    renew-ssl)
        sudo /usr/lib/ollamastudio/generate-ssl.sh "$2"
        sudo systemctl reload nginx
        echo "Certificat SSL regénéré"
        ;;
    *)
        echo "Usage: ollamastudio-ctl {start|stop|restart|status|logs|renew-ssl [hostname]}"
        ;;
esac
CTLEOF
    chmod 755 "$PREFIX/usr/bin/ollamastudio-ctl"
}

# ═══════════════════════════════════════════════════════════════════════
# 5. GÉNÉRATION DES PACKAGES
# ═══════════════════════════════════════════════════════════════════════
step "5/5 — Génération des packages"

# ── .deb ──────────────────────────────────────────────────────────────
build_deb() {
    if ! command -v dpkg-deb >/dev/null 2>&1; then
        warn "dpkg-deb non disponible — .deb ignoré"
        return
    fi

    info "Construction du package .deb..."

    local DEB_DIR="$DIST_DIR/deb-root"
    local DEB_NAME="${APP_NAME}_${VERSION}_${ARCH}.deb"

    rm -rf "$DEB_DIR"
    populate_install_tree "$DEB_DIR"

    # DEBIAN/control
    mkdir -p "$DEB_DIR/DEBIAN"
    cat > "$DEB_DIR/DEBIAN/control" <<CTLEOF
Package: $APP_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: $DESCRIPTION
 Backend Rust + Frontend SvelteKit pour interagir avec Ollama.
 Inclut un terminal web, un explorateur de fichiers, un système
 de skills/templates, et un chat agentique avec tool calling.
Homepage: $URL
Depends: libsqlite3-0, libssl3 | libssl1.1, libc6, bash, nodejs (>= 18), nginx, openssl
Section: web
Priority: optional
Installed-Size: $(du -sk "$DEB_DIR" | cut -f1)
CTLEOF

    cat > "$DEB_DIR/DEBIAN/postinst" <<'POSTEOF'
#!/bin/bash
set -e

# Crée l'utilisateur système
if ! id ollamastudio >/dev/null 2>&1; then
    useradd --system --home-dir /var/lib/ollamastudio --shell /usr/sbin/nologin ollamastudio
fi

# Permissions
chown -R ollamastudio:ollamastudio /var/lib/ollamastudio
chown -R ollamastudio:ollamastudio /opt/ollamastudio

# Génère le certificat SSL self-signed si absent
if [ ! -f /etc/ollamastudio/ssl/ollamastudio.crt ]; then
    echo "Génération du certificat SSL self-signed..."
    /usr/lib/ollamastudio/generate-ssl.sh
fi

# Configure Nginx
if [ -d /etc/nginx/sites-enabled ]; then
    # Debian/Ubuntu style
    rm -f /etc/nginx/sites-enabled/default 2>/dev/null || true
    ln -sf /etc/nginx/sites-available/ollamastudio /etc/nginx/sites-enabled/ollamastudio
elif [ -d /etc/nginx/conf.d ]; then
    # RHEL/Fedora style
    cp /etc/nginx/sites-available/ollamastudio /etc/nginx/conf.d/ollamastudio.conf
fi

# Teste la config nginx
nginx -t 2>/dev/null && echo "  Config Nginx OK" || echo "  ATTENTION: config Nginx invalide — vérifiez manuellement"

# Systemd
systemctl daemon-reload
systemctl enable nginx 2>/dev/null || true
systemctl restart nginx 2>/dev/null || true

echo ""
echo "╔═══════════════════════════════════════════════════╗"
echo "║           OllamaStudio installé !                ║"
echo "╠═══════════════════════════════════════════════════╣"
echo "║                                                   ║"
echo "║  Démarrer :  sudo ollamastudio-ctl start          ║"
echo "║  Arrêter  :  sudo ollamastudio-ctl stop           ║"
echo "║  Logs     :  sudo ollamastudio-ctl logs           ║"
echo "║  Status   :  ollamastudio-ctl status              ║"
echo "║                                                   ║"
echo "║  Interface : https://$(hostname)                  ║"
echo "║  Config    : /etc/ollamastudio/                   ║"
echo "║  SSL       : /etc/ollamastudio/ssl/               ║"
echo "║                                                   ║"
echo "║  Remplacer le certificat SSL :                    ║"
echo "║    sudo cp mon-cert.crt /etc/ollamastudio/ssl/ollamastudio.crt"
echo "║    sudo cp ma-cle.key /etc/ollamastudio/ssl/ollamastudio.key"
echo "║    sudo systemctl reload nginx                    ║"
echo "║                                                   ║"
echo "╚═══════════════════════════════════════════════════╝"
echo ""
POSTEOF
    chmod 755 "$DEB_DIR/DEBIAN/postinst"

    cat > "$DEB_DIR/DEBIAN/prerm" <<'PRERMEOF'
#!/bin/bash
set -e
systemctl stop ollamastudio-frontend 2>/dev/null || true
systemctl stop ollamastudio-backend 2>/dev/null || true
systemctl disable ollamastudio-frontend 2>/dev/null || true
systemctl disable ollamastudio-backend 2>/dev/null || true
PRERMEOF
    chmod 755 "$DEB_DIR/DEBIAN/prerm"

    cat > "$DEB_DIR/DEBIAN/conffiles" <<'CONFEOF'
/etc/ollamastudio/backend.env
/etc/ollamastudio/frontend.env
/etc/nginx/sites-available/ollamastudio
CONFEOF

    dpkg-deb --build --root-owner-group "$DEB_DIR" "$DIST_DIR/$DEB_NAME" 2>/dev/null || \
    dpkg-deb --build "$DEB_DIR" "$DIST_DIR/$DEB_NAME"

    rm -rf "$DEB_DIR"
    ok ".deb créé: dist/$DEB_NAME ($(du -h "$DIST_DIR/$DEB_NAME" | cut -f1))"
}

# ── .rpm ──────────────────────────────────────────────────────────────
build_rpm() {
    if ! command -v rpmbuild >/dev/null 2>&1; then
        warn "rpmbuild non disponible — .rpm ignoré"
        warn "Installez avec: sudo apt-get install rpm  OU  sudo dnf install rpm-build"
        return
    fi

    info "Construction du package .rpm..."

    local RPM_ROOT="$DIST_DIR/rpmbuild"
    rm -rf "$RPM_ROOT"
    mkdir -p "$RPM_ROOT"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}

    local BR="$RPM_ROOT/BUILDROOT/${APP_NAME}-${VERSION}-1.${RPM_ARCH}"
    populate_install_tree "$BR"

    # Spec file — on utilise %install pour copier depuis un staging dir
    local STAGING="$RPM_ROOT/STAGING"
    mv "$BR" "$STAGING"
    mkdir -p "$BR"  # rpmbuild needs the BUILDROOT dir to exist

    cat > "$RPM_ROOT/SPECS/$APP_NAME.spec" <<SPECEOF
Name:           $APP_NAME
Version:        $VERSION
Release:        1
Summary:        $DESCRIPTION
License:        $LICENSE
URL:            $URL
BuildArch:      $RPM_ARCH
AutoReqProv:    no

Requires:       sqlite-libs, openssl-libs, openssl, glibc, bash, nodejs >= 18, nginx

%description
$DESCRIPTION
Backend Rust + Frontend SvelteKit pour interagir avec Ollama.
Inclut un terminal web, un explorateur de fichiers, un systeme
de skills/templates, et un chat agentique avec tool calling.

%install
cp -a $STAGING/* %{buildroot}/

%files
%attr(755, root, root) /usr/bin/ollamastudio-backend
%attr(755, root, root) /usr/bin/ollamastudio-ctl
%config(noreplace) /etc/ollamastudio/backend.env
%config(noreplace) /etc/ollamastudio/frontend.env
/etc/nginx/sites-available/ollamastudio
/usr/lib/ollamastudio/generate-ssl.sh
/usr/lib/systemd/system/ollamastudio-backend.service
/usr/lib/systemd/system/ollamastudio-frontend.service
/opt/ollamastudio
%dir /etc/ollamastudio/ssl
%dir /var/lib/ollamastudio
%dir /var/lib/ollamastudio/data
%dir /var/lib/ollamastudio/data/skills
%dir /var/lib/ollamastudio/data/templates
%dir /var/lib/ollamastudio/data/mcp
%dir /var/lib/ollamastudio/workspace
%dir /var/lib/ollamastudio/documents

%pre
getent group ollamastudio >/dev/null || groupadd -r ollamastudio
getent passwd ollamastudio >/dev/null || useradd -r -g ollamastudio -d /var/lib/ollamastudio -s /sbin/nologin ollamastudio

%post
chown -R ollamastudio:ollamastudio /var/lib/ollamastudio
chown -R ollamastudio:ollamastudio /opt/ollamastudio
# Génère le certificat SSL self-signed si absent
if [ ! -f /etc/ollamastudio/ssl/ollamastudio.crt ]; then
    /usr/lib/ollamastudio/generate-ssl.sh
fi
# Configure Nginx
if [ -d /etc/nginx/conf.d ]; then
    cp /etc/nginx/sites-available/ollamastudio /etc/nginx/conf.d/ollamastudio.conf 2>/dev/null || true
fi
nginx -t 2>/dev/null || true
systemctl daemon-reload
systemctl enable nginx 2>/dev/null || true
systemctl restart nginx 2>/dev/null || true
echo ""
echo "=== OllamaStudio installé ==="
echo "  Démarrer  : sudo ollamastudio-ctl start"
echo "  Interface : https://$(hostname)"
echo ""

%preun
systemctl stop ollamastudio-frontend 2>/dev/null || true
systemctl stop ollamastudio-backend 2>/dev/null || true
systemctl disable ollamastudio-frontend 2>/dev/null || true
systemctl disable ollamastudio-backend 2>/dev/null || true
SPECEOF

    rpmbuild --define "_topdir $RPM_ROOT" -bb "$RPM_ROOT/SPECS/$APP_NAME.spec" 2>&1 | tail -3

    find "$RPM_ROOT/RPMS" -name "*.rpm" -exec cp {} "$DIST_DIR/" \;
    rm -rf "$RPM_ROOT"

    local RPM_FILE=$(ls "$DIST_DIR"/*.rpm 2>/dev/null | head -1)
    if [ -n "$RPM_FILE" ]; then
        ok ".rpm créé: $(basename "$RPM_FILE") ($(du -h "$RPM_FILE" | cut -f1))"
    fi
}

# ── Agent MCP packages ────────────────────────────────────────────────

build_agent_deb() {
    if ! command -v dpkg-deb >/dev/null 2>&1; then return; fi

    info "Construction du package agent-mcp .deb..."

    local DEB_DIR="$DIST_DIR/agent-deb-root"
    local DEB_NAME="agent-mcp_${VERSION}_${ARCH}.deb"

    rm -rf "$DEB_DIR"
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/lib/systemd/system"
    mkdir -p "$DEB_DIR/etc/ollamastudio"

    cp "$ROOT_DIR/$AGENT_BIN" "$DEB_DIR/usr/bin/agent-mcp"
    chmod 755 "$DEB_DIR/usr/bin/agent-mcp"
    strip "$DEB_DIR/usr/bin/agent-mcp" 2>/dev/null || true

    cat > "$DEB_DIR/etc/ollamastudio/agent-mcp.env" <<'ENVEOF'
# Agent MCP Configuration
MCP_PORT=9100
LOG_LEVEL=info
ENVEOF

    cat > "$DEB_DIR/lib/systemd/system/agent-mcp.service" <<'SVCEOF'
[Unit]
Description=OllamaStudio Agent MCP — Administration système
After=network.target

[Service]
Type=simple
User=root
EnvironmentFile=/etc/ollamastudio/agent-mcp.env
ExecStart=/usr/bin/agent-mcp
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

    cat > "$DEB_DIR/DEBIAN/control" <<CTLEOF
Package: agent-mcp
Version: $VERSION
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: OllamaStudio Agent MCP — Serveur MCP pour administration système complète
 Permet au LLM d'interagir avec la machine en tant que root :
 fichiers, packages, services, cron, réseau, processus, utilisateurs.
 Tourne en tant que service systemd sur le port 9100.
Homepage: $URL
Depends: bash, coreutils
Section: admin
Priority: optional
Installed-Size: $(du -sk "$DEB_DIR/usr" | cut -f1)
CTLEOF

    cat > "$DEB_DIR/DEBIAN/postinst" <<'POSTEOF'
#!/bin/bash
set -e
systemctl daemon-reload
systemctl enable agent-mcp 2>/dev/null || true
systemctl start agent-mcp 2>/dev/null || true
echo ""
echo "=== Agent MCP installé ==="
echo "  Service : agent-mcp (port 9100)"
echo "  Statut  : sudo systemctl status agent-mcp"
echo "  Config  : /etc/ollamastudio/agent-mcp.env"
echo ""
echo "  ATTENTION : ce service tourne en tant que root"
echo "  et peut exécuter n'importe quelle commande système."
echo ""
POSTEOF
    chmod 755 "$DEB_DIR/DEBIAN/postinst"

    cat > "$DEB_DIR/DEBIAN/prerm" <<'PRERMEOF'
#!/bin/bash
systemctl stop agent-mcp 2>/dev/null || true
systemctl disable agent-mcp 2>/dev/null || true
PRERMEOF
    chmod 755 "$DEB_DIR/DEBIAN/prerm"

    cat > "$DEB_DIR/DEBIAN/conffiles" <<'CONFEOF'
/etc/ollamastudio/agent-mcp.env
CONFEOF

    dpkg-deb --build --root-owner-group "$DEB_DIR" "$DIST_DIR/$DEB_NAME" 2>/dev/null || \
    dpkg-deb --build "$DEB_DIR" "$DIST_DIR/$DEB_NAME"

    rm -rf "$DEB_DIR"
    ok "agent-mcp .deb créé: dist/$DEB_NAME ($(du -h "$DIST_DIR/$DEB_NAME" | cut -f1))"
}

build_agent_rpm() {
    if ! command -v rpmbuild >/dev/null 2>&1; then return; fi

    info "Construction du package agent-mcp .rpm..."

    local RPM_ROOT="$DIST_DIR/agent-rpmbuild"
    rm -rf "$RPM_ROOT"
    mkdir -p "$RPM_ROOT"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}

    local STAGING="$RPM_ROOT/STAGING"
    mkdir -p "$STAGING/usr/bin"
    mkdir -p "$STAGING/usr/lib/systemd/system"
    mkdir -p "$STAGING/etc/ollamastudio"

    cp "$ROOT_DIR/$AGENT_BIN" "$STAGING/usr/bin/agent-mcp"
    chmod 755 "$STAGING/usr/bin/agent-mcp"
    strip "$STAGING/usr/bin/agent-mcp" 2>/dev/null || true

    cat > "$STAGING/etc/ollamastudio/agent-mcp.env" <<'ENVEOF'
MCP_PORT=9100
LOG_LEVEL=info
ENVEOF

    cat > "$STAGING/usr/lib/systemd/system/agent-mcp.service" <<'SVCEOF'
[Unit]
Description=OllamaStudio Agent MCP — Administration système
After=network.target

[Service]
Type=simple
User=root
EnvironmentFile=/etc/ollamastudio/agent-mcp.env
ExecStart=/usr/bin/agent-mcp
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
SVCEOF

    cat > "$RPM_ROOT/SPECS/agent-mcp.spec" <<SPECEOF
Name:           agent-mcp
Version:        $VERSION
Release:        1
Summary:        OllamaStudio Agent MCP — Administration système
License:        $LICENSE
URL:            $URL
BuildArch:      $RPM_ARCH
AutoReqProv:    no
Requires:       bash, coreutils

%description
Serveur MCP pour administration système complète.
Tourne en root, port 9100. Fichiers, packages, services, cron, réseau.

%install
cp -a $STAGING/* %{buildroot}/

%files
%attr(755, root, root) /usr/bin/agent-mcp
%config(noreplace) /etc/ollamastudio/agent-mcp.env
/usr/lib/systemd/system/agent-mcp.service

%post
systemctl daemon-reload
systemctl enable agent-mcp 2>/dev/null || true
systemctl start agent-mcp 2>/dev/null || true

%preun
systemctl stop agent-mcp 2>/dev/null || true
systemctl disable agent-mcp 2>/dev/null || true
SPECEOF

    rpmbuild --define "_topdir $RPM_ROOT" -bb "$RPM_ROOT/SPECS/agent-mcp.spec" 2>&1 | tail -3
    find "$RPM_ROOT/RPMS" -name "*.rpm" -exec cp {} "$DIST_DIR/" \;
    rm -rf "$RPM_ROOT"
    ok "agent-mcp .rpm créé"
}

# ── Exécution ─────────────────────────────────────────────────────────

if [ "$RPM_ONLY" = false ]; then
    build_deb
    build_agent_deb
fi

if [ "$DEB_ONLY" = false ]; then
    build_rpm
    build_agent_rpm
fi

# ── Résumé ────────────────────────────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
info "Packages générés dans dist/:"
ls -lh "$DIST_DIR"/*.{deb,rpm} 2>/dev/null || warn "Aucun package généré"
echo ""
echo "  Installation Debian/Ubuntu :"
echo "    sudo apt install ./dist/${APP_NAME}_${VERSION}_${ARCH}.deb ./dist/agent-mcp_${VERSION}_${ARCH}.deb"
echo ""
echo "  Installation Fedora/RHEL :"
echo "    sudo dnf install ./dist/${APP_NAME}-${VERSION}-1.${RPM_ARCH}.rpm ./dist/agent-mcp-${VERSION}-1.${RPM_ARCH}.rpm"
echo ""
echo "  Après installation :"
echo "    sudo ollamastudio-ctl start"
echo "    # OllamaStudio : https://localhost (SSL self-signed)"
echo "    # Agent MCP    : http://localhost:9100 (JSON-RPC, root)"
echo ""
echo "  Remplacer le certificat SSL :"
echo "    sudo cp cert.crt /etc/ollamastudio/ssl/ollamastudio.crt"
echo "    sudo cp cert.key /etc/ollamastudio/ssl/ollamastudio.key"
echo "    sudo systemctl reload nginx"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
