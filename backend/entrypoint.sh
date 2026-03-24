#!/bin/bash
set -e

# Crée les sous-dossiers nécessaires dans les volumes montés
# Utilise install -d pour gérer les permissions, avec fallback silencieux
for dir in /app/data/skills /app/data/templates /app/workspace /app/documents; do
    if [ ! -d "$dir" ]; then
        mkdir -p "$dir" 2>/dev/null || true
    fi
done

exec "$@"
