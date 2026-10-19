#!/bin/sh
set -eu
# Railway assigns PORT; an explicit BIND_ADDR remains available for other hosts.
export BIND_ADDR="${BIND_ADDR:-0.0.0.0:${PORT:-8080}}"
# Railway volumes start root-owned. Initialize directories, then drop privileges.
if [ "$(id -u)" = 0 ]; then
    mkdir -p /data/evidence /data/agreements
    chown worker:worker /data /data/evidence /data/agreements
    exec gosu worker "$@"
fi
exec "$@"
