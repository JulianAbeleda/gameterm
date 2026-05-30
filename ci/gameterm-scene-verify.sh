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
                        sprites, missing-sprite, or run-command-targets.
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
    run-command-targets)
      printf '%s\n' "${fixture_root}/run-command-targets.json"
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
    "${repo_root}/ci/gameterm-scene-patch.sh" \
    "${repo_root}/ci/gameterm-scene-process.sh" \
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
  templates="$("${repo_root}/ci/gameterm-scene-author.sh" list-templates)"
  grep -qx agent-workflow <<<"${templates}"
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
    new-template \
    --template agent-workflow \
    "${tmp_home}/gameterm/scenes/template.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${tmp_home}/gameterm/scenes/template.json" >/dev/null
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
    --target split_down \
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
  grep -q "OK: RunCommand target is valid: Run visual check -> tab" \
    /tmp/gameterm-scene-doctor-verify-ok.out

  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/run-command-targets.json" \
    --sprites "${fixture_root}/sprites.json" \
    >/tmp/gameterm-scene-doctor-verify-targets.out
  grep -q "OK: RunCommand target is valid: Run in tab -> tab" \
    /tmp/gameterm-scene-doctor-verify-targets.out
  grep -q "OK: RunCommand target is valid: Run in right split -> split_right" \
    /tmp/gameterm-scene-doctor-verify-targets.out
  grep -q "OK: RunCommand target is valid: Run in down split -> split_down" \
    /tmp/gameterm-scene-doctor-verify-targets.out

  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/default.json" \
    --sprites "${fixture_root}/sprites-missing.json" \
    >/tmp/gameterm-scene-doctor-verify-warn.out
  grep -q "WARN: sprite asset missing" \
    /tmp/gameterm-scene-doctor-verify-warn.out
  grep -q "SUGGEST: create" \
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

run_patch_check() {
  local tmp_home authored_patch inbox
  local exported_scene hidden_patch
  tmp_home="$(mktemp -d /tmp/gameterm-scene-patch-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  authored_patch="${tmp_home}/patches/project.json"
  hidden_patch="${tmp_home}/patches/hidden.json"
  inbox="${tmp_home}/inbox/scene-patch.json"
  exported_scene="${tmp_home}/exported/default.json"

  cargo run -q -p gameterm-visual --example scene_patch_apply -- \
    "${fixture_root}/default.json" \
    "${fixture_root}/patch-status.json" \
    >/tmp/gameterm-scene-patch-verify-ok.out
  grep -q "status=Fixture patch applied" \
    /tmp/gameterm-scene-patch-verify-ok.out
  grep -q "metadata.status=patched" \
    /tmp/gameterm-scene-patch-verify-ok.out

  set +e
  cargo run -q -p gameterm-visual --example scene_patch_apply -- \
    "${fixture_root}/default.json" \
    "${fixture_root}/patch-unknown-entity.json" \
    >/tmp/gameterm-scene-patch-verify-bad.out \
    2>/tmp/gameterm-scene-patch-verify-bad.err
  patch_rc=$?
  set -e
  if [[ "${patch_rc}" -eq 0 ]]; then
    echo "expected scene patch apply to reject unknown entity" >&2
    exit 1
  fi
  grep -q 'unknown entity id `missing-entity`' \
    /tmp/gameterm-scene-patch-verify-bad.err

  "${repo_root}/ci/gameterm-scene-patch.sh" \
    set-entity \
    --output "${authored_patch}" \
    --entity-id project-harness \
    --status "Authored patch applied" \
    --label "Harness Verified" \
    --position 5,3 \
    --sprite project_core \
    --select-entity-id project-harness \
    --visible \
    --flag loaded \
    --flag authored \
    --metadata fixture=default \
    --metadata source=verify >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    validate \
    --scene "${fixture_root}/default.json" \
    --patch "${authored_patch}" >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    write-inbox \
    --inbox "${inbox}" \
    --patch "${authored_patch}" >/dev/null
  cmp "${authored_patch}" "${inbox}"
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    export-scene \
    --scene "${fixture_root}/default.json" \
    --patch "${authored_patch}" \
    --output "${exported_scene}" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${exported_scene}" >/dev/null
  jq -e '
    .selected_entity_id == "project-harness"
  ' "${authored_patch}" >/dev/null
  jq -e '
    any(.entities[]; .id == "project-harness"
      and .label == "Harness Verified"
      and .position == {"x": 5, "y": 3}
      and .sprite == "project_core"
      and (.state_flags == ["loaded", "authored"])
      and (.metadata | any(.[0] == "source" and .[1] == "verify")))
  ' "${exported_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-patch.sh" \
    set-entity \
    --output "${hidden_patch}" \
    --entity-id project-harness \
    --status "Hidden patch applied" \
    --hidden >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    validate \
    --scene "${fixture_root}/default.json" \
    --patch "${hidden_patch}" >/dev/null
  jq -e '
    any(.updates[]; .entity_id == "project-harness" and .visible == false)
  ' "${hidden_patch}" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    set-entity \
    --output "${tmp_home}/patches/conflict.json" \
    --entity-id project-harness \
    --status "Conflicting visibility" \
    --visible \
    --hidden >/tmp/gameterm-scene-patch-conflict.out \
    2>/tmp/gameterm-scene-patch-conflict.err
  patch_rc=$?
  set -e
  if [[ "${patch_rc}" -eq 0 ]]; then
    echo "expected scene patch authoring to reject conflicting visibility flags" >&2
    exit 1
  fi
  grep -q -- '--visible and --hidden are mutually exclusive' \
    /tmp/gameterm-scene-patch-conflict.err

  "${repo_root}/ci/gameterm-scene-process.sh" \
    --entity-id project-harness \
    --patch "${tmp_home}/patches/process.json" \
    --inbox "${inbox}" \
    --select \
    -- \
    true >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    validate \
    --scene "${fixture_root}/default.json" \
    --patch "${tmp_home}/patches/process.json" >/dev/null
  jq -e '
    .selected_entity_id == "project-harness"
    and .status == "Process succeeded: true"
    and any(.updates[]; .entity_id == "project-harness"
      and (.state_flags == ["succeeded"])
      and (.metadata | any(.[0] == "exit_code" and .[1] == "0")))
  ' "${tmp_home}/patches/process.json" >/dev/null

  echo "scene patch: ok"
}

run_smoke_asset_check() {
  "${repo_root}/ci/gameterm-scene-smoke.sh" --check-assets >/dev/null
  echo "smoke assets: ok"
}

run_cargo_checks() {
  cargo test -p gameterm-visual scene_fixture
  cargo test -p gameterm-visual open_file
  cargo test -p gameterm-visual navigate
  cargo test -p gameterm-visual debug_report
  cargo test -p gameterm-visual scene_patch
  cargo test -p gameterm-visual render::tests
  cargo test -p gameterm-gui overlay::visual
  cargo test -p gameterm-gui visual_quad
  cargo test -p gameterm-gui render::tests
}

run_all() {
  run_static_checks
  run_init_helper_check
  run_author_helper_check
  run_doctor_check
  run_patch_check
  run_smoke_asset_check
  for fixture in basic navigate invalid sprites missing-sprite run-command-targets; do
    run_fixture_setup_check "${fixture}"
  done
  run_cargo_checks
}

cd "${repo_root}"

case "${mode}" in
  all)
    run_all
    ;;
  basic|navigate|invalid|sprites|missing-sprite|run-command-targets)
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
