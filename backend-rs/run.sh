#!/bin/bash
# Lance OllamaStudio backend Rust en mode natif (sans Docker)
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Répertoires de données
export WORKSPACE_ROOT="${WORKSPACE_ROOT:-$SCRIPT_DIR/workspace}"
export DATA_DIR="${DATA_DIR:-$SCRIPT_DIR/data}"
export DOCUMENTS_DIR="${DOCUMENTS_DIR:-$SCRIPT_DIR/documents}"
export APP_VERSION="$(cat ../VERSION 2>/dev/null || echo '0.0.14')"
export LOG_LEVEL="${LOG_LEVEL:-info}"
export PORT="${PORT:-8000}"

# Crée les répertoires
mkdir -p "$WORKSPACE_ROOT" "$DATA_DIR/skills" "$DATA_DIR/templates" "$DOCUMENTS_DIR"

echo "=== OllamaStudio Backend v$APP_VERSION ==="
echo "  Port:      $PORT"
echo "  Workspace: $WORKSPACE_ROOT"
echo "  Data:      $DATA_DIR"
echo "  Documents: $DOCUMENTS_DIR"
echo "  Log level: $LOG_LEVEL"
echo ""

# Compile si nécessaire et lance
if [ "$1" = "--release" ]; then
    cargo run --release
else
    cargo run
fi
