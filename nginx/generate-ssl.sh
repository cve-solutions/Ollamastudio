#!/bin/bash
# ═══════════════════════════════════════════════════════════════════
# Génère un certificat SSL self-signed pour OllamaStudio
#
# Usage :
#   ./generate-ssl.sh                    # Hostname auto-détecté
#   ./generate-ssl.sh monserveur.local   # Hostname personnalisé
#
# Fichiers générés :
#   /etc/ollamastudio/ssl/ollamastudio.crt  (certificat)
#   /etc/ollamastudio/ssl/ollamastudio.key  (clé privée)
#
# Pour remplacer par un vrai certificat (Let's Encrypt, CA interne) :
#   1. Remplacez les fichiers .crt et .key
#   2. sudo systemctl reload nginx
# ═══════════════════════════════════════════════════════════════════
set -e

SSL_DIR="/etc/ollamastudio/ssl"
CERT="$SSL_DIR/ollamastudio.crt"
KEY="$SSL_DIR/ollamastudio.key"
DAYS=3650  # 10 ans

# Hostname : argument ou auto-détection
HOSTNAME="${1:-$(hostname -f 2>/dev/null || hostname)}"
IP="$(hostname -I 2>/dev/null | awk '{print $1}')"

echo "=== Génération du certificat SSL self-signed ==="
echo "  Hostname : $HOSTNAME"
echo "  IP       : ${IP:-inconnue}"
echo "  Validité : $DAYS jours"
echo "  Sortie   : $CERT"

# Crée le répertoire
mkdir -p "$SSL_DIR"
chmod 700 "$SSL_DIR"

# Génère le certificat avec SAN (Subject Alternative Names)
# pour éviter les warnings navigateur sur les IP
SAN="DNS:$HOSTNAME,DNS:localhost"
[ -n "$IP" ] && SAN="$SAN,IP:$IP,IP:127.0.0.1"

openssl req -x509 -nodes -newkey rsa:2048 \
    -keyout "$KEY" \
    -out "$CERT" \
    -days "$DAYS" \
    -subj "/C=FR/O=OllamaStudio/CN=$HOSTNAME" \
    -addext "subjectAltName=$SAN" \
    -addext "keyUsage=digitalSignature,keyEncipherment" \
    -addext "extendedKeyUsage=serverAuth" \
    2>/dev/null

# Permissions
chmod 600 "$KEY"
chmod 644 "$CERT"

echo ""
echo "  Certificat : $CERT"
echo "  Clé privée : $KEY"
echo "  SAN        : $SAN"
echo ""
echo "  Pour remplacer par un vrai certificat :"
echo "    sudo cp mon-cert.crt $CERT"
echo "    sudo cp ma-cle.key $KEY"
echo "    sudo systemctl reload nginx"
echo ""
