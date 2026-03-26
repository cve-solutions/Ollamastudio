#!/bin/bash
set -e

# ═══════════════════════════════════════════════════════════════════
# OllamaStudio Backend — Build & Package Script
# Génère un binaire release + packages .deb et .rpm
# ═══════════════════════════════════════════════════════════════════

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

APP_NAME="ollamastudio-backend"
VERSION="$(cat ../VERSION 2>/dev/null || echo '0.0.14')"
ARCH="$(dpkg --print-architecture 2>/dev/null || uname -m)"
DESCRIPTION="OllamaStudio — Backend API pour interface web Ollama"
MAINTAINER="GoverByte <contact@goverbyte.com>"
URL="https://github.com/cve-solutions/ollamastudio"
LICENSE="Proprietary"

# Normalise l'architecture
case "$ARCH" in
    x86_64|amd64)  ARCH="amd64"; RPM_ARCH="x86_64" ;;
    aarch64|arm64)  ARCH="arm64"; RPM_ARCH="aarch64" ;;
    *)              RPM_ARCH="$ARCH" ;;
esac

BINARY="target/release/$APP_NAME"
BUILD_DIR="$SCRIPT_DIR/dist"

# ── Couleurs ──────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

info()  { echo -e "${BLUE}[INFO]${NC} $*"; }
ok()    { echo -e "${GREEN}[OK]${NC} $*"; }
fail()  { echo -e "${RED}[ERREUR]${NC} $*"; exit 1; }

# ── 1. Vérification des dépendances de build ─────────────────────
info "Vérification des outils de build..."

command -v cargo  >/dev/null 2>&1 || fail "cargo non trouvé. Installez Rust: https://rustup.rs"
command -v rustc  >/dev/null 2>&1 || fail "rustc non trouvé."

# Installe les dépendances système si nécessaire
install_build_deps() {
    if command -v apt-get >/dev/null 2>&1; then
        info "Détecté: Debian/Ubuntu — installation des dépendances..."
        sudo apt-get update -qq
        sudo apt-get install -y -qq \
            build-essential pkg-config \
            libsqlite3-dev libssl-dev
    elif command -v dnf >/dev/null 2>&1; then
        info "Détecté: Fedora/RHEL — installation des dépendances..."
        sudo dnf install -y -q \
            gcc make pkg-config \
            sqlite-devel openssl-devel
    elif command -v yum >/dev/null 2>&1; then
        info "Détecté: CentOS/RHEL — installation des dépendances..."
        sudo yum install -y -q \
            gcc make pkgconfig \
            sqlite-devel openssl-devel
    else
        info "Système non reconnu — assurez-vous que libsqlite3-dev et libssl-dev sont installés"
    fi
}

# Vérifie si les libs sont disponibles
if ! pkg-config --exists sqlite3 2>/dev/null || ! pkg-config --exists openssl 2>/dev/null; then
    install_build_deps
fi

ok "Outils de build OK"

# ── 2. Compilation release ────────────────────────────────────────
info "Compilation en mode release..."
cargo build --release 2>&1 | tail -3

if [ ! -f "$BINARY" ]; then
    fail "Binaire non trouvé: $BINARY"
fi

BINARY_SIZE=$(du -h "$BINARY" | cut -f1)
ok "Binaire compilé: $BINARY ($BINARY_SIZE)"

# ── 3. Préparation du dossier de distribution ─────────────────────
rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR"

# ── 4. Génération du package .deb ─────────────────────────────────
build_deb() {
    if ! command -v dpkg-deb >/dev/null 2>&1; then
        info "dpkg-deb non disponible — .deb ignoré"
        return
    fi

    info "Construction du package .deb..."

    local DEB_DIR="$BUILD_DIR/deb"
    local DEB_NAME="${APP_NAME}_${VERSION}_${ARCH}.deb"

    # Structure
    mkdir -p "$DEB_DIR/DEBIAN"
    mkdir -p "$DEB_DIR/usr/bin"
    mkdir -p "$DEB_DIR/etc/ollamastudio"
    mkdir -p "$DEB_DIR/var/lib/ollamastudio/data/skills"
    mkdir -p "$DEB_DIR/var/lib/ollamastudio/data/templates"
    mkdir -p "$DEB_DIR/var/lib/ollamastudio/workspace"
    mkdir -p "$DEB_DIR/var/lib/ollamastudio/documents"
    mkdir -p "$DEB_DIR/lib/systemd/system"

    # Binaire
    cp "$BINARY" "$DEB_DIR/usr/bin/$APP_NAME"
    chmod 755 "$DEB_DIR/usr/bin/$APP_NAME"
    strip "$DEB_DIR/usr/bin/$APP_NAME" 2>/dev/null || true

    # Fichier de config
    cat > "$DEB_DIR/etc/ollamastudio/env" <<ENVEOF
# OllamaStudio Backend Configuration
WORKSPACE_ROOT=/var/lib/ollamastudio/workspace
DATA_DIR=/var/lib/ollamastudio/data
DOCUMENTS_DIR=/var/lib/ollamastudio/documents
APP_VERSION=$VERSION
LOG_LEVEL=info
PORT=8000
ENVEOF

    # Service systemd
    cat > "$DEB_DIR/lib/systemd/system/ollamastudio.service" <<SVCEOF
[Unit]
Description=OllamaStudio Backend API
After=network.target

[Service]
Type=simple
User=ollamastudio
Group=ollamastudio
EnvironmentFile=/etc/ollamastudio/env
ExecStart=/usr/bin/$APP_NAME
Restart=on-failure
RestartSec=5
WorkingDirectory=/var/lib/ollamastudio

[Install]
WantedBy=multi-user.target
SVCEOF

    # DEBIAN/control
    cat > "$DEB_DIR/DEBIAN/control" <<CTLEOF
Package: $APP_NAME
Version: $VERSION
Architecture: $ARCH
Maintainer: $MAINTAINER
Description: $DESCRIPTION
Homepage: $URL
Depends: libsqlite3-0, libssl3 | libssl1.1, libc6, bash
Section: web
Priority: optional
Installed-Size: $(du -sk "$DEB_DIR/usr" | cut -f1)
CTLEOF

    # Scripts post-install / pre-remove
    cat > "$DEB_DIR/DEBIAN/postinst" <<'POSTEOF'
#!/bin/bash
set -e
# Crée l'utilisateur système
if ! id ollamastudio >/dev/null 2>&1; then
    useradd --system --home-dir /var/lib/ollamastudio --shell /usr/sbin/nologin ollamastudio
fi
chown -R ollamastudio:ollamastudio /var/lib/ollamastudio
systemctl daemon-reload
echo ""
echo "=== OllamaStudio installé ==="
echo "  Configurer: /etc/ollamastudio/env"
echo "  Démarrer:   sudo systemctl start ollamastudio"
echo "  Activer:    sudo systemctl enable ollamastudio"
echo ""
POSTEOF
    chmod 755 "$DEB_DIR/DEBIAN/postinst"

    cat > "$DEB_DIR/DEBIAN/prerm" <<'PRERMEOF'
#!/bin/bash
set -e
systemctl stop ollamastudio 2>/dev/null || true
systemctl disable ollamastudio 2>/dev/null || true
PRERMEOF
    chmod 755 "$DEB_DIR/DEBIAN/prerm"

    # Construction
    dpkg-deb --build "$DEB_DIR" "$BUILD_DIR/$DEB_NAME"
    ok ".deb créé: dist/$DEB_NAME"
}

# ── 5. Génération du package .rpm ─────────────────────────────────
build_rpm() {
    if ! command -v rpmbuild >/dev/null 2>&1; then
        info "rpmbuild non disponible — .rpm ignoré"
        return
    fi

    info "Construction du package .rpm..."

    local RPM_BUILD="$BUILD_DIR/rpmbuild"
    local RPM_NAME="${APP_NAME}-${VERSION}-1.${RPM_ARCH}.rpm"

    mkdir -p "$RPM_BUILD"/{BUILD,RPMS,SOURCES,SPECS,SRPMS,BUILDROOT}

    # Crée la structure dans BUILDROOT
    local BR="$RPM_BUILD/BUILDROOT/${APP_NAME}-${VERSION}-1.${RPM_ARCH}"
    mkdir -p "$BR/usr/bin"
    mkdir -p "$BR/etc/ollamastudio"
    mkdir -p "$BR/var/lib/ollamastudio/data/skills"
    mkdir -p "$BR/var/lib/ollamastudio/data/templates"
    mkdir -p "$BR/var/lib/ollamastudio/workspace"
    mkdir -p "$BR/var/lib/ollamastudio/documents"
    mkdir -p "$BR/usr/lib/systemd/system"

    cp "$BINARY" "$BR/usr/bin/$APP_NAME"
    chmod 755 "$BR/usr/bin/$APP_NAME"
    strip "$BR/usr/bin/$APP_NAME" 2>/dev/null || true

    cat > "$BR/etc/ollamastudio/env" <<ENVEOF
WORKSPACE_ROOT=/var/lib/ollamastudio/workspace
DATA_DIR=/var/lib/ollamastudio/data
DOCUMENTS_DIR=/var/lib/ollamastudio/documents
APP_VERSION=$VERSION
LOG_LEVEL=info
PORT=8000
ENVEOF

    cat > "$BR/usr/lib/systemd/system/ollamastudio.service" <<SVCEOF
[Unit]
Description=OllamaStudio Backend API
After=network.target

[Service]
Type=simple
User=ollamastudio
Group=ollamastudio
EnvironmentFile=/etc/ollamastudio/env
ExecStart=/usr/bin/$APP_NAME
Restart=on-failure
RestartSec=5
WorkingDirectory=/var/lib/ollamastudio

[Install]
WantedBy=multi-user.target
SVCEOF

    # Spec file
    cat > "$RPM_BUILD/SPECS/$APP_NAME.spec" <<SPECEOF
Name:           $APP_NAME
Version:        $VERSION
Release:        1
Summary:        $DESCRIPTION
License:        $LICENSE
URL:            $URL
BuildArch:      $RPM_ARCH

Requires:       sqlite-libs, openssl-libs, glibc, bash

%description
$DESCRIPTION — interface web Claude Code-compatible pour Ollama, 100%% local.

%install
cp -a %{_builddir}/../BUILDROOT/%{name}-%{version}-%{release}.%{_arch}/* %{buildroot}/

%files
%attr(755, root, root) /usr/bin/$APP_NAME
%config(noreplace) /etc/ollamastudio/env
/usr/lib/systemd/system/ollamastudio.service
%dir /var/lib/ollamastudio
%dir /var/lib/ollamastudio/data
%dir /var/lib/ollamastudio/data/skills
%dir /var/lib/ollamastudio/data/templates
%dir /var/lib/ollamastudio/workspace
%dir /var/lib/ollamastudio/documents

%pre
getent group ollamastudio >/dev/null || groupadd -r ollamastudio
getent passwd ollamastudio >/dev/null || useradd -r -g ollamastudio -d /var/lib/ollamastudio -s /sbin/nologin ollamastudio

%post
chown -R ollamastudio:ollamastudio /var/lib/ollamastudio
systemctl daemon-reload
echo ""
echo "=== OllamaStudio installé ==="
echo "  Configurer: /etc/ollamastudio/env"
echo "  Démarrer:   sudo systemctl start ollamastudio"
echo "  Activer:    sudo systemctl enable ollamastudio"
echo ""

%preun
systemctl stop ollamastudio 2>/dev/null || true
systemctl disable ollamastudio 2>/dev/null || true
SPECEOF

    rpmbuild --define "_topdir $RPM_BUILD" -bb "$RPM_BUILD/SPECS/$APP_NAME.spec"

    # Copie le RPM dans dist/
    find "$RPM_BUILD/RPMS" -name "*.rpm" -exec cp {} "$BUILD_DIR/" \;
    ok ".rpm créé: dist/$RPM_NAME"
}

# ── Exécution ─────────────────────────────────────────────────────
echo ""
echo "╔═══════════════════════════════════════════════╗"
echo "║  OllamaStudio Build — v$VERSION"
echo "╚═══════════════════════════════════════════════╝"
echo ""

build_deb
build_rpm

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
info "Packages générés dans dist/:"
ls -lh "$BUILD_DIR"/*.{deb,rpm} 2>/dev/null || info "(aucun package — outils dpkg-deb/rpmbuild manquants)"
echo ""
info "Installation .deb:  sudo dpkg -i dist/${APP_NAME}_${VERSION}_${ARCH}.deb"
info "Installation .rpm:  sudo rpm -i dist/${APP_NAME}-${VERSION}-1.${RPM_ARCH}.rpm"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
