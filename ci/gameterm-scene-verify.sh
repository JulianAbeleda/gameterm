#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/gameterm-scene-verify.sh [OPTIONS]

Runs noninteractive GameTerm Scene Mode verification against repository
fixtures. The script uses temporary config directories only.

Options:
  --all                 Run all checks. This is the default.
  --fixture NAME        Run one fixture setup check: basic, navigate, invalid,
                        sprites, or missing-sprite.
  -h, --help            Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
mode="all"
tmp_paths=()

cleanup() {
  for path in "${tmp_paths[@]}"; do
    if [[ -n "${path}" && -d "${path}" ]]; then
      rm -rf "${path}"
    fi
  done
}
trap cleanup EXIT

while [[ $# -gt 0 ]]; do
  case "$1" in
    --all)
      mode="all"
      shift
      ;;
    --fixture)
      mode="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

scene_file_for_fixture() {
  case "$1" in
    basic|navigate|sprites|missing-sprite)
      printf '%s\n' "${fixture_root}/default.json"
      ;;
    invalid)
      printf '%s\n' "${fixture_root}/invalid.json"
      ;;
    *)
      echo "unknown fixture: $1" >&2
      exit 2
      ;;
  esac
}

sprite_file_for_fixture() {
  case "$1" in
    sprites)
      printf '%s\n' "${fixture_root}/sprites.json"
      ;;
    missing-sprite)
      printf '%s\n' "${fixture_root}/sprites-missing.json"
      ;;
    *)
      return 1
      ;;
  esac
}

run_fixture_setup_check() {
  local fixture="$1"
  local tmp_home
  local scene_dir
  local sprite_file
  tmp_home="$(mktemp -d /tmp/gameterm-scene-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  scene_dir="${tmp_home}/gameterm/scenes"
  mkdir -p "${scene_dir}"
  cp "$(scene_file_for_fixture "${fixture}")" "${scene_dir}/default.json"
  if [[ "${fixture}" == "navigate" ]]; then
    cp "${fixture_root}/memory.json" "${scene_dir}/memory.json"
  fi
  if sprite_file="$(sprite_file_for_fixture "${fixture}")"; then
    cp "${sprite_file}" "${scene_dir}/sprites.json"
  fi

  jq empty "${scene_dir}/default.json"
  if [[ -f "${scene_dir}/memory.json" ]]; then
    jq empty "${scene_dir}/memory.json"
  fi
  if [[ -f "${scene_dir}/sprites.json" ]]; then
    jq empty "${scene_dir}/sprites.json"
  fi
  echo "fixture ${fixture}: setup ok"
}

run_static_checks() {
  for script in \
    "${repo_root}/ci/gameterm-scene-author.sh" \
    "${repo_root}/ci/gameterm-scene-doctor.sh" \
    "${repo_root}/ci/gameterm-scene-init.sh" \
    "${repo_root}/ci/gameterm-scene-smoke.sh" \
    "${repo_root}/ci/gameterm-scene-verify.sh"
  do
    bash -n "${script}"
  done
  jq empty "${fixture_root}"/*.json
  git -C "${repo_root}" diff --check
}

run_init_helper_check() {
  local tmp_home
  tmp_home="$(mktemp -d /tmp/gameterm-scene-init-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  "${repo_root}/ci/gameterm-scene-init.sh" --config-home "${tmp_home}" >/dev/null
  jq empty "${tmp_home}/gameterm/scenes/default.json"

  set +e
  "${repo_root}/ci/gameterm-scene-init.sh" --config-home "${tmp_home}" \
    >/tmp/gameterm-scene-init-verify.out \
    2>/tmp/gameterm-scene-init-verify.err
  overwrite_rc=$?
  set -e
  if [[ "${overwrite_rc}" -eq 0 ]]; then
    echo "expected init helper overwrite protection to fail" >&2
    exit 1
  fi

  "${repo_root}/ci/gameterm-scene-init.sh" \
    --config-home "${tmp_home}" \
    --force \
    --with-sprites >/dev/null
  jq empty \
    "${tmp_home}/gameterm/scenes/default.json" \
    "${tmp_home}/gameterm/scenes/sprites.json"
  echo "init helper: ok"
}

run_author_helper_check() {
  local tmp_home
  local fixtures
  tmp_home="$(mktemp -d /tmp/gameterm-scene-author-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")

  fixtures="$("${repo_root}/ci/gameterm-scene-author.sh" list-fixtures)"
  grep -qx navigate <<<"${fixtures}"
  "${repo_root}/ci/gameterm-scene-author.sh" \
    install-fixture \
    --config-home "${tmp_home}" \
    navigate >/dev/null
  jq empty \
    "${tmp_home}/gameterm/scenes/default.json" \
    "${tmp_home}/gameterm/scenes/memory.json"
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${tmp_home}/gameterm/scenes/default.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    new-scene \
    --force \
    --title "Author Check" \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-entity \
    --id author-task \
    --kind Task \
    --label "Author Task" \
    --x 1 \
    --y 1 \
    --sprite task_tile \
    --flag ready \
    --metadata source=verify \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-choice \
    --label "Run true" \
    --run-argv '["true"]' \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    update-choice \
    --choice-index 1 \
    --label "Open docs" \
    --open-file docs/gameterm-scene-mode.md \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    remove-choice \
    --choice-index 1 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    move-entity \
    --id author-task \
    --x 2 \
    --y 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-dialogue \
    --speaker "Author" \
    --text "Updated by verifier." \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    format "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    remove-entity \
    --id author-task \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${tmp_home}/gameterm/scenes/authored.json" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${fixture_root}/invalid.json" \
    >/tmp/gameterm-scene-author-verify.out \
    2>/tmp/gameterm-scene-author-verify.err
  invalid_rc=$?
  set -e
  if [[ "${invalid_rc}" -eq 0 ]]; then
    echo "expected author validate to reject invalid fixture" >&2
    exit 1
  fi

  echo "author helper: ok"
}

run_doctor_check() {
  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/default.json" \
    --sprites "${fixture_root}/sprites.json" \
    >/tmp/gameterm-scene-doctor-verify-ok.out
  grep -q "Doctor summary: 0 error(s), 0 warning(s)" \
    /tmp/gameterm-scene-doctor-verify-ok.out

  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/default.json" \
    --sprites "${fixture_root}/sprites-missing.json" \
    >/tmp/gameterm-scene-doctor-verify-warn.out
  grep -q "WARN: sprite asset missing" \
    /tmp/gameterm-scene-doctor-verify-warn.out

  set +e
  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/default.json" \
    --sprites "${fixture_root}/sprites-missing.json" \
    --strict \
    >/tmp/gameterm-scene-doctor-verify-strict.out
  strict_rc=$?
  set -e
  if [[ "${strict_rc}" -eq 0 ]]; then
    echo "expected doctor --strict to fail on warning fixture" >&2
    exit 1
  fi

  echo "doctor: ok"
}

run_cargo_checks() {
  cargo test -p gameterm-visual scene_fixture
  cargo test -p gameterm-visual open_file
  cargo test -p gameterm-visual navigate
  cargo test -p gameterm-visual debug_report
  cargo test -p gameterm-gui overlay::visual
}

run_all() {
  run_static_checks
  run_init_helper_check
  run_author_helper_check
  run_doctor_check
  for fixture in basic navigate invalid sprites missing-sprite; do
    run_fixture_setup_check "${fixture}"
  done
  run_cargo_checks
}

cd "${repo_root}"

case "${mode}" in
  all)
    run_all
    ;;
  basic|navigate|invalid|sprites|missing-sprite)
    run_static_checks
    run_fixture_setup_check "${mode}"
    ;;
  *)
    echo "unknown mode: ${mode}" >&2
    usage >&2
    exit 2
    ;;
esac

echo "Scene Mode verification succeeded."
