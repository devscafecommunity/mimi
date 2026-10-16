#!/bin/sh
set -eu
# Skill worker entry (M3.1.2). Runs MIMI_SKILL_CMD or passes args through.

if [ -n "${MIMI_HEARTBEAT_INTERVAL:-}" ]; then
  (
    while true; do
      echo "mimi-heartbeat $(date +%s)"
      sleep "${MIMI_HEARTBEAT_INTERVAL}"
    done
  ) >&2 &
fi

if [ -n "${MIMI_SKILL_CMD:-}" ]; then
  exec sh -c "$MIMI_SKILL_CMD"
fi
exec "$@"
