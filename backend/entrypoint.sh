#!/bin/bash
set -e

# Assure que les volumes montés sont accessibles en écriture
# Le conteneur tourne en root, donc chown/chmod fonctionnent
for dir in /app/data /app/workspace /app/documents; do
    if [ -d "$dir" ]; then
        chmod 755 "$dir" 2>/dev/null || true
    else
        mkdir -p "$dir"
    fi
done

# Crée les sous-dossiers nécessaires
mkdir -p /app/data/skills /app/data/templates

exec "$@"
