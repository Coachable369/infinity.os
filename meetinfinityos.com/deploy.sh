#!/usr/bin/env bash
set -euo pipefail

SITE_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
HOST="${INFINITY_DEPLOY_HOST:-root@31.220.59.85}"
ROOT="/var/www/meetinfinityos.com"
STAMP="$(date -u +%Y%m%dT%H%M%SZ)"

case "${1:-}" in
  --help|-h)
    printf '%s\n' \
      'Usage: ./deploy.sh [--check | --help]' \
      'Deploys only meetinfinityos.com using the existing Coachable SSH credentials.' \
      'Creates a remote backup, uploads allowlisted files, and verifies HTTPS pages.' \
      '--check validates local upload files without credentials or network access.'
    exit 0
    ;;
  ''|--check) ;;
  *) printf 'Unknown option: %s\n' "$1" >&2; exit 2 ;;
esac
if (( $# > 1 )); then
  printf 'Expected at most one option. Use --help.\n' >&2
  exit 2
fi

FILES=(
  index.html whitepaper.html invest.html styles.css whitepaper.css navigation.js app.js art.js
  emblem-geometry.js scene-math.mjs assets/emblem-outline.json
  assets/brand.webp assets/infinity-emblem.webp assets/infinity-detail.webp
  assets/screenshots/sign-in-960.webp assets/screenshots/sign-in-1600.webp assets/screenshots/sign-in-full.webp
  assets/screenshots/desktop-ai-960.webp assets/screenshots/desktop-ai-1600.webp assets/screenshots/desktop-ai-full.webp
  assets/screenshots/app-library-960.webp assets/screenshots/app-library-1600.webp assets/screenshots/app-library-full.webp
  assets/Inter-Variable.ttf assets/vendor/three.module.min.js
  assets/vendor/three.core.min.js assets/vendor/BufferGeometryUtils.js
  assets/OFL-inter.txt assets/LICENSE-lucide.txt assets/LICENSE-three.txt
)
for file in "${FILES[@]}"; do
  if [[ ! -f "$SITE_DIR/$file" || ! -r "$SITE_DIR/$file" || -L "$SITE_DIR/$file" ]]; then
    printf 'Missing, unreadable, or symlinked deployment file: %s\n' "$file" >&2
    exit 1
  fi
done
if [[ "${1:-}" == --check ]]; then
  printf 'Validated %s public files. No production connection made.\n' "${#FILES[@]}"
  exit 0
fi
for executable in ssh sshpass tar; do
  command -v "$executable" >/dev/null || { printf 'Required command not found: %s\n' "$executable" >&2; exit 1; }
done

if [[ -z "${SSHPASS:-}" ]]; then
  for env_file in /opt/codebase/coachable/v2.0/.env /opt/codebase/coachable/v2.0/environment.env; do
    if [[ -f "$env_file" ]]; then
      set -a
      source "$env_file"
      set +a
    fi
  done
  export SSHPASS="${SSH_PASSWORD:-${DEPLOY_SSH_PASSWORD:-${REMOTE_SSH_PASSWORD:-}}}"
fi

# Share Coachable's authenticated connection across backup, upload and publish.
SSH_OPTS=(-o ConnectTimeout=15 -o ControlMaster=auto -o ControlPersist=120 -o 'ControlPath=/tmp/ssh-%r@%h:%p')

# Backups live outside the public document root.
sshpass -e ssh "${SSH_OPTS[@]}" "$HOST" bash -s -- "$ROOT" "$STAMP" <<'REMOTE'
set -euo pipefail
test -f "$1/public_html/index.html"
mkdir -p "$1/backups" "$1/public_html/assets" "$1/staging-$2"
tar -czf "$1/backups/website-$2.tar.gz" -C "$1" public_html
REMOTE

COPYFILE_DISABLE=1 tar -czf - -C "$SITE_DIR" "${FILES[@]}" |
  sshpass -e ssh "${SSH_OPTS[@]}" "$HOST" \
    "tar -xzf - -C '$ROOT/staging-$STAMP'"
sshpass -e ssh "${SSH_OPTS[@]}" "$HOST" bash -s -- "$ROOT" "$STAMP" <<'REMOTE'
set -euo pipefail
mkdir -p "$1/public_html/assets/vendor"
mkdir -p "$1/public_html/assets/screenshots"
mv "$1/staging-$2/assets/screenshots/"* "$1/public_html/assets/screenshots/"
rmdir "$1/staging-$2/assets/screenshots"
mv "$1/staging-$2/assets/vendor/"* "$1/public_html/assets/vendor/"
rmdir "$1/staging-$2/assets/vendor"
mv "$1/staging-$2/assets/"* "$1/public_html/assets/"
mv "$1/staging-$2/"*.css "$1/staging-$2/"*.js "$1/staging-$2/"*.mjs "$1/public_html/"
mv "$1/staging-$2/invest.html" "$1/public_html/invest.html"
mv "$1/staging-$2/whitepaper.html" "$1/public_html/whitepaper.html"
mv "$1/staging-$2/index.html" "$1/public_html/index.html"
rmdir "$1/staging-$2/assets" "$1/staging-$2"
chmod -R u=rwX,go=rX "$1/public_html"
printf 'Backup: %s/backups/website-%s.tar.gz\n' "$1" "$2"
sha256sum "$1/public_html/index.html" "$1/public_html/app.js" "$1/public_html/styles.css"
systemctl is-active httpd
curl --fail --silent --output /dev/null --write-out 'HTTPS apex: %{http_code}\n' https://meetinfinityos.com/
curl --fail --silent --output /dev/null --write-out 'HTTPS www: %{http_code}\n' https://www.meetinfinityos.com/
curl --fail --silent --output /dev/null --write-out 'HTTPS whitepaper: %{http_code}\n' https://meetinfinityos.com/whitepaper.html
curl --fail --silent --output /dev/null --write-out 'HTTPS investors: %{http_code}\n' https://meetinfinityos.com/invest.html
REMOTE
