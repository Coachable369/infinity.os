#!/usr/bin/env bash
# Install only the InfinityOS vhost redirect rules. Keep a backup outside the web root.
set -eu
cd "$(dirname "$0")"
if [[ -z "${SSHPASS:-}" ]]; then
  set -a
  for config in /opt/codebase/coachable/v2.0/.env /opt/codebase/coachable/v2.0/environment.env; do
    if [[ -f "$config" ]]; then source "$config"; fi
  done
  set +a
  export SSHPASS="${SSH_PASSWORD:-${DEPLOY_SSH_PASSWORD:-${REMOTE_SSH_PASSWORD:-}}}"
fi
host="${INFINITY_DEPLOY_HOST:-root@31.220.59.85}"
stamp="$(date -u +%Y%m%dT%H%M%SZ)"
directory="/var/www/meetinfinityos.com/backups/seo-$stamp"
sshpass -e ssh -o ControlMaster=auto -o ControlPersist=120 -o 'ControlPath=/tmp/ssh-%r@%h:%p' "$host" "mkdir -p '$directory' && cat > '$directory/rules.inc'" < seo-vhost.inc
sshpass -e ssh -o ControlMaster=auto -o ControlPersist=120 -o 'ControlPath=/tmp/ssh-%r@%h:%p' "$host" bash -s -- "$directory" <<'REMOTE'
set -eu
dir="$1"
conf=/etc/httpd/conf.d/meetinfinityos.com-le-ssl.conf
inc=/etc/httpd/conf.d/meetinfinityos-canonical.inc
cp -p "$conf" "$dir/vhost.conf"
if test -f "$inc"; then cp -p "$inc" "$dir/previous.inc"; fi
install -m 644 "$dir/rules.inc" "$inc"
if ! grep -Fxq "Include $inc" "$conf"; then sed -i "/<\/VirtualHost>/i Include $inc" "$conf"; fi
if ! apachectl configtest || ! systemctl reload httpd; then
  cp -p "$dir/vhost.conf" "$conf"
  if test -f "$dir/previous.inc"; then cp -p "$dir/previous.inc" "$inc"; else rm -f "$inc"; fi
  apachectl configtest && systemctl reload httpd
  exit 1
fi
REMOTE
