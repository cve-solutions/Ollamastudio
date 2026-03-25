#!/bin/bash
set -e

# Assure que les volumes montés sont accessibles en écriture.
# Le conteneur tourne en root, donc chown/chmod fonctionnent.
for dir in /app/data /app/workspace /app/documents; do
    mkdir -p "$dir"
    chown root:root "$dir"
    chmod 755 "$dir"
done

# Crée les sous-dossiers nécessaires
mkdir -p /app/data/skills /app/data/templates

exec "$@"
