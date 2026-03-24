#!/bin/bash
set -e

# Crée les sous-dossiers nécessaires dans les volumes montés
mkdir -p /app/data/skills /app/data/templates /app/workspace /app/documents

exec "$@"
