#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd "$(dirname "$0")" && pwd)"
HOST="${INFINITY_DEPLOY_HOST:-root@31.220.59.85}"
ROOT="/var/www/meetinfinityos.com"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

if [[ -z "${SSHPASS:-}" ]]; then
  source /opt/codebase/coachable/v2.0/environment.env
  export SSHPASS="$SSH_PASSWORD"
fi

# Backups live outside the public document root.
sshpass -e ssh -o ConnectTimeout=15 -o NumberOfPasswordPrompts=1 "$HOST" bash -s -- "$ROOT" "$STAMP" <<'REMOTE'
set -euo pipefail
test -f "$1/public_html/index.html"
mkdir -p "$1/backups" "$1/public_html/assets" "$1/staging-$2"
tar -czf "$1/backups/website-$2.tar.gz" -C "$1" public_html
REMOTE

COPYFILE_DISABLE=1 tar -czf - -C "$SITE_DIR" index.html styles.css app.js art.js \
  emblem-geometry.js scene-math.mjs assets/emblem-outline.json \
  assets/brand.webp assets/infinity-emblem.webp assets/infinity-detail.webp \
  assets/Inter-Variable.ttf assets/vendor/three.module.min.js \
  assets/vendor/three.core.min.js assets/vendor/BufferGeometryUtils.js \
  assets/OFL-inter.txt assets/LICENSE-lucide.txt assets/LICENSE-three.txt |
  sshpass -e ssh -o ConnectTimeout=15 -o NumberOfPasswordPrompts=1 "$HOST" \
    "tar -xzf - -C '$ROOT/staging-$STAMP'"
sshpass -e ssh -o ConnectTimeout=15 -o NumberOfPasswordPrompts=1 "$HOST" bash -s -- "$ROOT" "$STAMP" <<'REMOTE'
set -euo pipefail
mkdir -p "$1/public_html/assets/vendor"
mv "$1/staging-$2/assets/vendor/"* "$1/public_html/assets/vendor/"
rmdir "$1/staging-$2/assets/vendor"
mv "$1/staging-$2/assets/"* "$1/public_html/assets/"
mv "$1/staging-$2/styles.css" "$1/staging-$2/"*.js "$1/staging-$2/"*.mjs "$1/public_html/"
mv "$1/staging-$2/index.html" "$1/public_html/index.html"
rmdir "$1/staging-$2/assets" "$1/staging-$2"
chmod -R u=rwX,go=rX "$1/public_html"
printf 'Backup: %s/backups/website-%s.tar.gz\n' "$1" "$2"
sha256sum "$1/public_html/index.html" "$1/public_html/app.js" "$1/public_html/styles.css"
systemctl is-active httpd
curl --fail --silent --output /dev/null --write-out 'HTTPS apex: %{http_code}\n' https://meetinfinityos.com/
curl --fail --silent --output /dev/null --write-out 'HTTPS www: %{http_code}\n' https://www.meetinfinityos.com/
REMOTE
