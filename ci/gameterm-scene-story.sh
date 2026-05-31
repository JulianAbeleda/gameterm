#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  ci/gameterm-scene-story.sh export SCENE OUTPUT
  ci/gameterm-scene-story.sh import SCENE STATE OUTPUT
  ci/gameterm-scene-story.sh validate STATE
  ci/gameterm-scene-story.sh inspect STATE

Explicit helper for Scene Mode runtime story/RPG state. It never rewrites the
source scene JSON.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ $# -lt 1 ]]; then
  usage >&2
  exit 2
fi

case "$1" in
  export|import|validate|inspect)
    (cd "${repo_root}" && cargo run -q -p gameterm-visual --example scene_story_state -- "$@")
    ;;
  -h|--help)
    usage
    ;;
  *)
    echo "unknown command: $1" >&2
    usage >&2
    exit 2
    ;;
esac
