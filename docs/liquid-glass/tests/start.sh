#!/usr/bin/env bash
# usage: start.sh NAME [FILE.pen]
# Starts this checkout's Pencraft as a private instance for one test. Its address file goes to out/NAME/home, so
# it neither sees nor disturbs a Pencraft you have open. Then: PENCRAFT_HOME=out/NAME/home python pc.py tools
set -eu
HERE="$(cd "$(dirname "$0")" && pwd)"
APP="$HERE/../../../app/target/debug/pencraft"
HOMEDIR="$HERE/out/$1/home"
mkdir -p "$HOMEDIR"
rm -f "$HOMEDIR/.pencraft/agent.json"
shift_file="${2:-}"
(HOME="$HOMEDIR" USERPROFILE="$HOMEDIR" "$APP" --background --private $shift_file >"$HERE/out/$1/app.log" 2>&1 &)
for i in $(seq 1 60); do [ -f "$HOMEDIR/.pencraft/agent.json" ] && break; sleep 0.5; done
PENCRAFT_HOME="$HOMEDIR" python "$HERE/pc.py" open_document '{}' >/dev/null
echo "started $1: PENCRAFT_HOME=$HOMEDIR"
