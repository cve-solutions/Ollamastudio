#!/bin/bash

# ── Initialise les répertoires de données ────────────────────────────
# Les volumes bind-mount peuvent arriver avec des permissions restrictives
# selon le filesystem hôte (SELinux, NFS, user-namespace, etc.).

ensure_dir() {
    local d="$1"
    if [ ! -d "$d" ]; then
        mkdir -p "$d" 2>/dev/null || true
    fi
    # Tente de fixer les permissions (silencieux si non supporté)
    chown root:root "$d" 2>/dev/null || true
    chmod 755 "$d"       2>/dev/null || true
}

# Répertoires racines (points de montage)
for dir in /app/data /app/workspace /app/documents; do
    ensure_dir "$dir"
done

# Sous-dossiers requis
for sub in /app/data/skills /app/data/templates; do
    ensure_dir "$sub"
done

# ── Vérification : le backend peut-il écrire dans /app/data ? ────────
_test_file="/app/data/.write_test_$$"
if touch "$_test_file" 2>/dev/null; then
    rm -f "$_test_file"
else
    echo "============================================================"
    echo "ERREUR : /app/data n'est PAS accessible en écriture."
    echo ""
    echo "Corrigez les permissions côté hôte :"
    echo "  sudo chown -R \$(id -u):\$(id -g) ./backend/data"
    echo "  sudo chmod -R 755 ./backend/data"
    echo ""
    echo "Puis relancez : docker compose up -d --build"
    echo "============================================================"
    exit 1
fi

exec "$@"
