#!/usr/bin/env bash
# usage: stop.sh NAME   Stops the instance start.sh started for that test (by its own process id only).
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
PID=$(python -c "import json,sys; print(json.load(open(sys.argv[1]))['pid'])" "$HERE/out/$1/home/.pencraft/agent.json")
if command -v taskkill >/dev/null 2>&1; then taskkill //F //PID "$PID"; else kill "$PID"; fi
