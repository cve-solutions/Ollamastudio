#!/bin/bash
# Point d'entrée OllamaStudio backend.
# La création des répertoires est gérée par Python (settings.ensure_dirs())
# au démarrage de l'app — pas besoin de le faire ici.

exec "$@"
