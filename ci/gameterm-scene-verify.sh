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
                        sprites, missing-sprite, run-command-targets,
                        layered-mode, vertical-slice, authoring-loop,
                        game-states, chained-transitions, or
                        workspace-agent, multi-agent-coordination, or
                        renpy-demo.
  -h, --help            Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
fixture_root="${repo_root}/ci/fixtures/gameterm-scene"
mode="all"
tmp_paths=()
scene_scripts=(
  gameterm-scene-agent.sh
  gameterm-scene-author.sh
  gameterm-scene-doctor.sh
  gameterm-scene-init.sh
  gameterm-scene-mux-context.sh
  gameterm-scene-patch.sh
  gameterm-scene-process.sh
  gameterm-scene-session.sh
  gameterm-scene-smoke.sh
  gameterm-scene-story.sh
  gameterm-scene-verify.sh
  gameterm-scene-vn-demo.sh
  gameterm-scene-workspace.sh
)
onboarding_required_patterns=(
  'ci/gameterm-scene-workspace.sh inspect --cwd .'
  'ci/gameterm-scene-workspace.sh discover'
  'ci/gameterm-scene-author.sh validate'
  'ci/gameterm-scene-doctor.sh'
  'ci/gameterm-scene-verify.sh --all'
  'ci/gameterm-scene-smoke.sh --launch --scenario workspace-discovery'
  'ci/gameterm-scene-author.sh install-fixture workspace-agent --force'
  'does not run commands, start agents, submit prompts, or overwrite'
)

cleanup() {
  set +u
  for path in "${tmp_paths[@]}"; do
    if [[ -n "${path}" && -d "${path}" ]]; then
      rm -rf "${path}"
    fi
  done
  set -u
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
    layered-mode)
      printf '%s\n' "${fixture_root}/layered-mode.json"
      ;;
    vertical-slice)
      printf '%s\n' "${fixture_root}/vertical-slice.json"
      ;;
    authoring-loop)
      printf '%s\n' "${fixture_root}/authoring-loop.json"
      ;;
    game-states)
      printf '%s\n' "${fixture_root}/game-states.json"
      ;;
    chained-transitions)
      printf '%s\n' "${fixture_root}/chained-transitions.json"
      ;;
    workspace-agent)
      printf '%s\n' "${fixture_root}/workspace-agent.json"
      ;;
    multi-agent-coordination)
      printf '%s\n' "${fixture_root}/multi-agent-coordination.json"
      ;;
    renpy-demo)
      printf '%s\n' "${fixture_root}/renpy-demo.json"
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
  local script
  for script in "${scene_scripts[@]}"; do
    case "${script}" in
      *.py)
        python3 -m py_compile "${repo_root}/ci/${script}"
        ;;
      *)
        bash -n "${repo_root}/ci/${script}"
        ;;
    esac
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
  grep -qx visual-novel <<<"${templates}"
  grep -qx layered-mode <<<"${templates}"
  grep -qx rpg-quest <<<"${templates}"
  grep -qx vertical-slice <<<"${templates}"
  grep -qx workspace-agent <<<"${templates}"
  grep -qx multi-agent-coordination <<<"${templates}"
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
  for template in visual-novel layered-mode rpg-quest vertical-slice workspace-agent multi-agent-coordination; do
    "${repo_root}/ci/gameterm-scene-author.sh" \
      new-template \
      --template "${template}" \
      "${tmp_home}/gameterm/scenes/${template}.json" >/dev/null
    "${repo_root}/ci/gameterm-scene-author.sh" \
      validate "${tmp_home}/gameterm/scenes/${template}.json" >/dev/null
  done
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
    set-variable \
    --key verifier_ready \
    --value-bool true \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-variable \
    --key verifier_count \
    --value-number 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-variable \
    --key verifier_track \
    --value-text authoring \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    clear-variable \
    --key verifier_count \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-layer \
    --layer-id verify \
    --state draft \
    --label Verify \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  cp "${tmp_home}/gameterm/scenes/authored.json" \
    "${tmp_home}/gameterm/scenes/authored-before-failed-mutation.json"
  set +e
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-layer \
    --layer-id verify \
    --state duplicate \
    "${tmp_home}/gameterm/scenes/authored.json" \
    >/tmp/gameterm-scene-author-rollback.out \
    2>/tmp/gameterm-scene-author-rollback.err
  duplicate_layer_rc=$?
  set -e
  if [[ "${duplicate_layer_rc}" -eq 0 ]]; then
    echo "expected duplicate layer mutation to fail" >&2
    exit 1
  fi
  cmp \
    "${tmp_home}/gameterm/scenes/authored-before-failed-mutation.json" \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-layer-transition \
    --layer-id verify \
    --input activate \
    --target-state complete \
    --condition-variable verifier_ready \
    --condition-bool true \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-layer \
    --layer-id verify \
    --state ready \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-mode-input \
    --input other \
    --action run_update_hooks \
    --condition-source inventory_count \
    --condition-variable verify-token \
    --condition-number 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-lifecycle \
    --enter-status "Entered verifier" \
    --update-status "Updated verifier" \
    --exit-status "Exited verifier" \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  jq -e '
    any(.variables[]; .key == "verifier_ready" and .value == {"Bool": true})
    and any(.variables[]; .key == "verifier_track" and .value == {"Text": "authoring"})
    and (all(.variables[]; .key != "verifier_count"))
    and any(.layers[]; .layer_id == "verify"
      and .state == "ready"
      and .label == "Verify"
      and (.transitions | any(.input == "activate"
        and .target_state == "complete"
        and (.conditions | any(.variable == "verifier_ready"
          and .equals == {"Bool": true})))))
    and any(.mode.input_map[]; .input == "other"
      and .action == "run_update_hooks"
      and (.conditions | any(.source == "inventory_count"
        and .variable == "verify-token"
        and .equals == {"Number": 2})))
    and .mode.lifecycle.enter_status == "Entered verifier"
    and .mode.lifecycle.update_status == "Updated verifier"
    and .mode.lifecycle.exit_status == "Exited verifier"
  ' "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-inventory \
    --item-id verify-token \
    --label "Verify Token" \
    --count 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    set-stat \
    --owner-id author \
    --key focus \
    --value-number 3 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    adjust-stat \
    --owner-id author \
    --key focus \
    --amount 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    add-quest \
    --quest-id verify-quest \
    --label "Verify Quest" \
    --stage 1 \
    --journal "Started verification." \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    advance-quest \
    --quest-id verify-quest \
    --stage 2 \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    append-quest-journal \
    --quest-id verify-quest \
    --journal "Advanced verification." \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    complete-quest \
    --quest-id verify-quest \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  jq -e '
    any(.rpg.inventory[]; .item_id == "verify-token"
      and .label == "Verify Token"
      and .count == 2)
    and any(.rpg.stats[]; .owner_id == "author"
      and .key == "focus"
      and .value == {"Number": 5})
    and any(.rpg.quests[]; .quest_id == "verify-quest"
      and .stage == 2
      and .completed == true
      and (.journal | contains("Started verification."))
      and (.journal | contains("Advanced verification.")))
  ' "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    remove-inventory \
    --item-id verify-token \
    "${tmp_home}/gameterm/scenes/authored.json" >/dev/null
  jq -e 'all(.rpg.inventory[]?; .item_id != "verify-token")' \
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
  grep -q "Doctor summary: 0 error(s), 0 warning(s)" \
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
  grep -q "WARN: RunCommand cwd is missing: Run in tab" \
    /tmp/gameterm-scene-doctor-verify-targets.out
  grep -q "WARN: choice policy origin is missing: Run in tab" \
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

run_vn_script_import_check() {
  local tmp_dir
  local scene_output
  local attribution_output
  tmp_dir="$(mktemp -d /tmp/gameterm-scene-vn-script-verify.XXXXXX)"
  tmp_paths+=("${tmp_dir}")
  scene_output="${tmp_dir}/renpy-demo.json"
  attribution_output="${tmp_dir}/renpy-demo-attribution.json"

  cargo run -q -p gameterm-visual --example scene_vn_script_import -- \
    --source "${fixture_root}/renpy-demo-source.rpy" \
    --output "${scene_output}" \
    --attribution "${attribution_output}" \
    --source-dialect rpy \
    --source-title "GameTerm Ren'Py Demo Fixture" \
    --source-version fixture \
    --title "VN Script Demo Import" \
    >/tmp/gameterm-scene-vn-script-verify.out \
    2>/tmp/gameterm-scene-vn-script-verify.err

  jq -e '
    .variables[] | select(.key == "source_dialect" and .value.Text == "rpy")
  ' "${scene_output}" >/dev/null
  jq -e '
    any(.choices[]; .policy.origin == "vn_script_import"
      and .policy.risk == "state_change"
      and .policy.scope == "scene")
    and any(.choices[]; .conditions[]? | .variable == "met_guide")
  ' "${scene_output}" >/dev/null
  jq -e '
    .license_url == "https://www.renpy.org/doc/html/license.html"
    and .source_dialect == "rpy"
    and .source_version == "fixture"
    and (.assets | length) == 0
    and any(.notes[]; contains("does not copy assets"))
  ' "${attribution_output}" >/dev/null
  jq -e '
    any(.sources[]; .id == "tainara_p_female_character_creator"
      and .license == "CC0-1.0"
      and .repo_policy == "allowed_with_provenance")
    and any(.sources[]; .id == "4cher_set4_vn_sprites"
      and .license == "CC-BY-4.0"
      and .repo_policy == "allowed_with_attribution")
    and all(.sources[]; .id != "potat0master_school_mini_pack_1")
  ' "${fixture_root}/renpy-demo-open-assets.json" >/dev/null
  grep -q "non-menu jump is recorded" /tmp/gameterm-scene-vn-script-verify.err
  cargo run -q -p gameterm-visual --example scene_validate -- "${scene_output}" >/dev/null
  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${scene_output}" \
    --sprites "${fixture_root}/sprites.json" >/tmp/gameterm-scene-vn-script-doctor.out
  grep -q "Doctor summary: 0 error(s)" /tmp/gameterm-scene-vn-script-doctor.out

  echo "vn script import: ok"
}

run_vn_asset_intake_check() {
  local tmp_dir
  local output_root
  local sprite_manifest
  local attribution
  local bindings
  tmp_dir="$(mktemp -d /tmp/gameterm-scene-vn-assets-verify.XXXXXX)"
  tmp_paths+=("${tmp_dir}")
  output_root="${tmp_dir}/assets/vn-demo"
  sprite_manifest="${tmp_dir}/sprites.json"
  attribution="${tmp_dir}/vn-demo-attribution.json"
  bindings="${tmp_dir}/vn-demo-bindings.json"

  cargo run -q -p gameterm-visual --example scene_vn_asset_intake -- \
    --catalog "${fixture_root}/renpy-demo-open-assets.json" \
    --source-root "${fixture_root}/vn-asset-source" \
    --output-root "${output_root}" \
    --sprite-manifest "${sprite_manifest}" \
    --attribution "${attribution}" \
    --bindings "${bindings}" \
    --base-manifest "${fixture_root}/sprites.json" \
    >/tmp/gameterm-scene-vn-assets-verify.out \
    2>/tmp/gameterm-scene-vn-assets-verify.err

  jq -e '
    any(.sprites[]; .id == "workspace-map")
    and any(.sprites[]; .id == "vn.character.guide.neutral"
      and (.path | endswith("assets/vn-demo/characters/guide-neutral.png")))
    and any(.sprites[]; .id == "vn.character.guide.happy"
      and (.path | endswith("assets/vn-demo/characters/guide-happy.png")))
  ' "${sprite_manifest}" >/dev/null
  jq -e '
    .characters.guide.expressions.neutral == "vn.character.guide.neutral"
    and .characters.guide.expressions.happy == "vn.character.guide.happy"
  ' "${bindings}" >/dev/null
  jq -e '
    .generated_by == "scene_vn_asset_intake"
    and any(.sources[]; .id == "4cher_set4_vn_sprites"
      and .repo_policy == "allowed_with_attribution"
      and (.used_assets | length) == 2)
    and any(.warnings[]; contains("requires sprite composition"))
    and any(.warnings[]; contains("AI-assisted source skipped"))
  ' "${attribution}" >/dev/null
  test -f "${output_root}/characters/guide-neutral.png"
  test -f "${output_root}/characters/guide-happy.png"
  grep -q "AI-assisted source skipped" /tmp/gameterm-scene-vn-assets-verify.err

  "${repo_root}/ci/gameterm-scene-doctor.sh" \
    --scene "${fixture_root}/renpy-demo.json" \
    --sprites "${sprite_manifest}" >/tmp/gameterm-scene-vn-assets-doctor.out
  grep -q "Doctor summary: 0 error(s)" /tmp/gameterm-scene-vn-assets-doctor.out

  echo "vn asset intake: ok"
}

run_vn_demo_install_check() {
  local tmp_dir
  local generated_dir
  local config_home
  local installed_dir
  local overwrite_rc
  tmp_dir="$(mktemp -d /tmp/gameterm-scene-vn-demo-verify.XXXXXX)"
  tmp_paths+=("${tmp_dir}")
  generated_dir="${tmp_dir}/generated"
  config_home="${tmp_dir}/config"
  installed_dir="${config_home}/gameterm/scenes"

  "${repo_root}/ci/gameterm-scene-vn-demo.sh" generate \
    --output-dir "${generated_dir}" \
    --asset-source-root "${fixture_root}/vn-asset-source" \
    --force \
    >/tmp/gameterm-scene-vn-demo-generate.out \
    2>/tmp/gameterm-scene-vn-demo-generate.err

  cargo run -q -p gameterm-visual --example scene_validate -- \
    "${generated_dir}/default.json" >/dev/null
  jq -e '
    .background == "workspace-map"
    and any(.entities[]; .id == "vn-script-narrator"
      and .sprite == "vn.character.guide.neutral")
  ' "${generated_dir}/default.json" >/dev/null
  jq -e '
    any(.sprites[]; .id == "workspace-map")
    and any(.sprites[]; .id == "vn.character.guide.neutral")
  ' "${generated_dir}/sprites.json" >/dev/null
  jq -e '
    .characters.guide.expressions.neutral == "vn.character.guide.neutral"
  ' "${generated_dir}/vn-demo-bindings.json" >/dev/null
  jq -e '.source_dialect == "rpy"' \
    "${generated_dir}/vn-demo-script-attribution.json" >/dev/null
  jq -e '.generated_by == "scene_vn_asset_intake"' \
    "${generated_dir}/vn-demo-asset-attribution.json" >/dev/null
  test -f "${generated_dir}/assets/vn-demo/characters/guide-neutral.png"
  grep -q "AI-assisted source skipped" /tmp/gameterm-scene-vn-demo-generate.err

  "${repo_root}/ci/gameterm-scene-vn-demo.sh" doctor \
    --output-dir "${generated_dir}" \
    >/tmp/gameterm-scene-vn-demo-doctor.out
  grep -q "Doctor summary: 0 error(s)" /tmp/gameterm-scene-vn-demo-doctor.out

  "${repo_root}/ci/gameterm-scene-vn-demo.sh" install \
    --config-home "${config_home}" \
    --asset-source-root "${fixture_root}/vn-asset-source" \
    --force \
    >/tmp/gameterm-scene-vn-demo-install.out \
    2>/tmp/gameterm-scene-vn-demo-install.err
  cargo run -q -p gameterm-visual --example scene_validate -- \
    "${installed_dir}/default.json" >/dev/null

  cp "${installed_dir}/default.json" "${tmp_dir}/default.before.json"
  set +e
  "${repo_root}/ci/gameterm-scene-vn-demo.sh" install \
    --config-home "${config_home}" \
    --asset-source-root "${fixture_root}/vn-asset-source" \
    >/tmp/gameterm-scene-vn-demo-overwrite.out \
    2>/tmp/gameterm-scene-vn-demo-overwrite.err
  overwrite_rc=$?
  set -e
  if [[ "${overwrite_rc}" -eq 0 ]]; then
    echo "expected vn demo install to refuse overwrite without --force" >&2
    exit 1
  fi
  cmp -s "${tmp_dir}/default.before.json" "${installed_dir}/default.json"
  grep -q "refusing to overwrite existing file without --force" \
    /tmp/gameterm-scene-vn-demo-overwrite.err

  echo "vn demo install: ok"
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
    and .process_state == {
      "entity_id": "project-harness",
      "phase": "succeeded",
      "command": "true",
      "exit_code": 0,
      "message": "Process succeeded"
    }
    and any(.updates[]; .entity_id == "project-harness"
      and (.state_flags == ["succeeded"])
      and (.metadata | any(.[0] == "exit_code" and .[1] == "0")))
  ' "${tmp_home}/patches/process.json" >/dev/null

  echo "scene patch: ok"
}

run_agent_check() {
  local tmp_home planning_patch blocked_patch complete_patch waiting_patch cancelled_patch multi_patch inbox
  tmp_home="$(mktemp -d /tmp/gameterm-scene-agent-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  planning_patch="${tmp_home}/patches/agent-planning.json"
  blocked_patch="${tmp_home}/patches/agent-blocked.json"
  complete_patch="${tmp_home}/patches/agent-complete.json"
  waiting_patch="${tmp_home}/patches/agent-waiting.json"
  cancelled_patch="${tmp_home}/patches/agent-cancelled.json"
  multi_patch="${tmp_home}/patches/agent-multi.json"
  inbox="${tmp_home}/inbox/scene-patch.json"

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase planning \
    --command "plan visual slice" \
    --message "Planning visual slice" \
    --patch "${planning_patch}" \
    --select >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    validate \
    --scene "${fixture_root}/default.json" \
    --patch "${planning_patch}" >/dev/null
  jq -e '
    .selected_entity_id == "project-harness"
    and .status == "Agent planning: Planning visual slice"
    and .process_state == {
      "entity_id": "project-harness",
      "phase": "queued",
      "command": "plan visual slice",
      "message": "Planning visual slice"
    }
    and (.variables | any(.key == "agent_phase" and .value == {"Text": "planning"}))
    and any(.updates[]; .entity_id == "project-harness"
      and (.state_flags == ["agent", "agent_planning"])
      and (.metadata | any(.[0] == "agent_phase" and .[1] == "planning")))
  ' "${planning_patch}" >/dev/null

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase blocked \
    --message "Waiting on credentials" \
    --patch "${blocked_patch}" \
    --inbox "${inbox}" >/dev/null
  cmp "${blocked_patch}" "${inbox}"
  jq -e '
    .process_state.phase == "blocked"
    and .process_state.message == "Waiting on credentials"
    and (.variables | any(.key == "agent_process_phase" and .value == {"Text": "blocked"}))
  ' "${blocked_patch}" >/dev/null

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase waiting \
    --message "Waiting on user input" \
    --patch "${waiting_patch}" >/dev/null
  jq -e '
    .process_state.phase == "blocked"
    and (.variables | any(.key == "agent_phase" and .value == {"Text": "waiting"}))
    and any(.updates[]; .entity_id == "project-harness"
      and (.state_flags == ["agent", "agent_waiting"]))
  ' "${waiting_patch}" >/dev/null

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase complete \
    --message "Finished visual slice" \
    --patch "${complete_patch}" >/dev/null
  jq -e '
    .process_state.phase == "succeeded"
    and .process_state.exit_code == 0
    and (.variables | any(.key == "agent_phase" and .value == {"Text": "completed"}))
    and any(.updates[]; .entity_id == "project-harness"
      and (.state_flags == ["agent", "agent_completed"]))
  ' "${complete_patch}" >/dev/null

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase cancelled \
    --message "Cancelled by user" \
    --patch "${cancelled_patch}" >/dev/null
  jq -e '
    .process_state.phase == "failed"
    and .process_state.exit_code == 130
    and (.variables | any(.key == "agent_phase" and .value == {"Text": "cancelled"}))
  ' "${cancelled_patch}" >/dev/null

  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id agent-audit \
    --task-id task-review \
    --blocked-by task-build \
    --phase blocked \
    --message "Waiting for build output" \
    --patch "${multi_patch}" \
    --select >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" \
    validate \
    --scene "${fixture_root}/multi-agent-coordination.json" \
    --patch "${multi_patch}" >/dev/null
  jq -e '
    .selected_entity_id == "agent-audit"
    and (.status | contains("agent-audit blocked for task-review"))
    and (.variables | any(.key == "active_agent_id" and .value == {"Text": "agent-audit"}))
    and (.variables | any(.key == "active_task_id" and .value == {"Text": "task-review"}))
    and (.variables | any(.key == "agent_blocked_by" and .value == {"Text": "task-build"}))
    and any(.updates[]; .entity_id == "agent-audit"
      and (.metadata | any(.[0] == "agent_task_id" and .[1] == "task-review"))
      and (.metadata | any(.[0] == "blocked_by" and .[1] == "task-build")))
  ' "${multi_patch}" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-agent.sh" \
    status \
    --entity-id project-harness \
    --phase paused \
    --patch "${tmp_home}/patches/bad.json" \
    >/tmp/gameterm-scene-agent-bad.out \
    2>/tmp/gameterm-scene-agent-bad.err
  agent_rc=$?
  set -e
  if [[ "${agent_rc}" -eq 0 ]]; then
    echo "expected agent helper to reject unknown phase" >&2
    exit 1
  fi
  grep -q -- '--phase must be idle, planning, running, waiting, blocked, complete, completed, failed, or cancelled' \
    /tmp/gameterm-scene-agent-bad.err

  echo "agent helper: ok"
}

run_workspace_discovery_check() {
  local tmp_home git_scene pane_scene non_git_dir non_git_scene empty_dir empty_scene patch_path pane_patch_path brief_path install_home
  tmp_home="$(mktemp -d /tmp/gameterm-scene-workspace-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  git_scene="${tmp_home}/git-workspace.json"
  pane_scene="${tmp_home}/pane-workspace.json"
  non_git_dir="${tmp_home}/non-git"
  non_git_scene="${tmp_home}/non-git-workspace.json"
  empty_dir="${tmp_home}/empty-workspace"
  empty_scene="${tmp_home}/empty-workspace.json"
  patch_path="${tmp_home}/workspace.patch.json"
  pane_patch_path="${tmp_home}/workspace-pane.patch.json"
  brief_path="${tmp_home}/task-brief.json"
  install_home="${tmp_home}/config"

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    inspect \
    --cwd "${repo_root}" >/tmp/gameterm-scene-workspace-inspect.out
  grep -q '^root_dir=' /tmp/gameterm-scene-workspace-inspect.out
  grep -q '^repo_status=' /tmp/gameterm-scene-workspace-inspect.out

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${repo_root}" \
    --brief-output "${brief_path}" \
    --scene-output "${git_scene}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" validate "${git_scene}" >/dev/null
  jq -e '
    .brief_version == 1
    and .workspace_root == "'"${repo_root}"'"
    and (.context_files | length > 0)
    and (.constraints | index("do not run commands automatically"))
    and (.constraints | index("do not start agents automatically"))
  ' "${brief_path}" >/dev/null
  set +e
  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    brief \
    --cwd "${repo_root}" \
    --brief-output "${brief_path}" \
    >/tmp/gameterm-scene-brief-overwrite.out \
    2>/tmp/gameterm-scene-brief-overwrite.err
  brief_rc=$?
  set -e
  if [[ "${brief_rc}" -eq 0 ]]; then
    echo "expected task brief overwrite protection to fail" >&2
    exit 1
  fi
  jq -e '
    (.entities | map(select(.visible // true)) | length) as $visible_count
    | (.entities
      | map(select(.visible // true) | "\(.position.x),\(.position.y)")
      | unique
      | length) == $visible_count
    and any(.entities[]; .id == "discovered-workspace" and .position == {"x": 2, "y": 2})
    and any(.entities[]; .id == "discovered-project" and .position == {"x": 5, "y": 2})
    and any(.entities[]; .id == "discovered-pane" and .position == {"x": 11, "y": 2})
    and any(.entities[]; .id == "discovered-process" and .position == {"x": 12, "y": 5})
    and all(.entities[] | select((.metadata // []) | any(.[0] == "entity_type" and .[1] == "file")); .position.y >= 6)
    and
    .mode.mode_id == "workspace"
    and any(.entities[]; .id == "discovered-workspace"
      and .kind == "Project"
      and (.metadata | any(.[0] == "repo_status")))
    and any(.entities[]; .id == "discovered-project"
      and (.metadata | any(.[0] == "language" and .[1] == "rust")))
    and any(.entities[]; .id == "discovered-pane"
      and (.metadata | any(.[0] == "context" and .[1] == "absent")))
    and any(.entities[]; .id == "discovered-process")
    and any(.rpg.relationships[]; .source_id == "discovered-project"
      and .target_id == "file-0"
      and .kind == "includes"
      and (.metadata | any(.[0] == "source" and .[1] == "workspace-discovery")))
    and any(.rpg.relationships[]; .source_id == "discovered-task"
      and .target_id == "file-0"
      and .kind == "references")
    and any(.entities[]; .id == "task-brief"
      and (.metadata | any(.[0] == "path" and .[1] == "'"${brief_path}"'")))
    and any(.choices[]; .label == "Open task brief")
    and any(.rpg.relationships[]; .source_id == "discovered-task"
      and .target_id == "task-brief"
      and .kind == "described_by")
    and any(.choices[]; .label == "Run verification")
    and any(.choices[]; .label == "Run verification"
      and .policy.origin == "workspace_discovery"
      and .policy.risk == "command"
      and .policy.scope == "workspace"
      and .policy.requires_confirmation == true)
    and all(.choices[]; .policy.origin == "workspace_discovery")
  ' "${git_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${repo_root}" \
    --pane-id 231 \
    --mux-window-id 7 \
    --pane-cwd "${repo_root}" \
    --foreground-process-name zsh \
    --foreground-process-path /bin/zsh \
    --pane-progress None \
    --scene-output "${pane_scene}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" validate "${pane_scene}" >/dev/null
  jq -e '
    any(.variables[]; .key == "pane_context" and .value == {"Text": "provided"})
    and any(.variables[]; .key == "active_pane_id" and .value == {"Number": 231})
    and any(.variables[]; .key == "process_phase" and .value == {"Text": "running"})
    and any(.entities[]; .id == "discovered-pane"
      and (.metadata | any(.[0] == "pane_id" and .[1] == "231"))
      and (.metadata | any(.[0] == "mux_window_id" and .[1] == "7")))
    and any(.entities[]; .id == "discovered-process"
      and .label == "zsh"
      and (.state_flags | index("running"))
      and (.metadata | any(.[0] == "foreground_process_path" and .[1] == "/bin/zsh")))
    and any(.rpg.relationships[]; .source_id == "discovered-pane"
      and .target_id == "discovered-process"
      and .kind == "observes"
      and (.metadata | any(.[0] == "source" and .[1] == "pane-metadata")))
  ' "${pane_scene}" >/dev/null

  mkdir -p "${non_git_dir}"
  printf '# Temporary workspace\n' >"${non_git_dir}/README.md"
  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${non_git_dir}" \
    --scene-output "${non_git_scene}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" validate "${non_git_scene}" >/dev/null
  jq -e '
    any(.variables[]; .key == "repo_status" and .value == {"Text": "not_git"})
    and any(.choices[]; .label == "Open README.md")
  ' "${non_git_scene}" >/dev/null

  mkdir -p "${empty_dir}"
  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${empty_dir}" \
    --scene-output "${empty_scene}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" validate "${empty_scene}" >/dev/null
  jq -e '
    any(.variables[]; .key == "repo_status" and .value == {"Text": "not_git"})
    and any(.variables[]; .key == "discovered_file_count" and .value == {"Number": 0})
    and all(.entities[]; ((.metadata // []) | any(.[0] == "entity_type" and .[1] == "file")) | not)
  ' "${empty_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    patch \
    --cwd "${repo_root}" \
    --patch-output "${patch_path}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" validate \
    --scene "${fixture_root}/workspace-agent.json" \
    --patch "${patch_path}" >/dev/null
  jq -e '
    .selected_entity_id == "workspace-gameterm"
    and (.variables | any(.key == "repo_status"))
    and any(.updates[]; .entity_id == "workspace-gameterm")
  ' "${patch_path}" >/dev/null

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    patch \
    --cwd "${repo_root}" \
    --pane-id 231 \
    --mux-window-id 7 \
    --pane-cwd "${repo_root}" \
    --foreground-process-name zsh \
    --foreground-process-path /bin/zsh \
    --pane-progress None \
    --patch-output "${pane_patch_path}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" validate \
    --scene "${fixture_root}/workspace-agent.json" \
    --patch "${pane_patch_path}" >/dev/null
  jq -e '
    .process_state.entity_id == "scene-verify-process"
    and .process_state.phase == "running"
    and .process_state.command == "zsh"
    and (.variables | any(.key == "active_pane_id" and .value == {"Number": 231}))
    and any(.updates[]; .entity_id == "workspace-gameterm"
      and (.metadata | any(.[0] == "pane_cwd")))
    and any(.updates[]; .entity_id == "scene-verify-process"
      and .label == "zsh"
      and (.metadata | any(.[0] == "pane_progress" and .[1] == "None")))
  ' "${pane_patch_path}" >/dev/null

  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${repo_root}" \
    --install \
    --config-home "${install_home}" >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" \
    validate "${install_home}/gameterm/scenes/default.json" >/dev/null
  cp "${install_home}/gameterm/scenes/default.json" \
    "${tmp_home}/installed-before-overwrite.json"
  set +e
  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${repo_root}" \
    --install \
    --config-home "${install_home}" \
    >/tmp/gameterm-scene-workspace-overwrite.out \
    2>/tmp/gameterm-scene-workspace-overwrite.err
  overwrite_rc=$?
  set -e
  if [[ "${overwrite_rc}" -eq 0 ]]; then
    echo "expected workspace discovery install overwrite protection to fail" >&2
    exit 1
  fi
  cmp \
    "${tmp_home}/installed-before-overwrite.json" \
    "${install_home}/gameterm/scenes/default.json" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-workspace.sh" \
    discover \
    --cwd "${repo_root}" \
    --open missing-workspace-file.md \
    --strict \
    --scene-output "${tmp_home}/strict.json" \
    >/tmp/gameterm-scene-workspace-strict.out \
    2>/tmp/gameterm-scene-workspace-strict.err
  strict_rc=$?
  set -e
  if [[ "${strict_rc}" -eq 0 ]]; then
    echo "expected workspace discovery strict missing file to fail" >&2
    exit 1
  fi
  grep -q "important file does not exist" \
    /tmp/gameterm-scene-workspace-strict.err

  echo "workspace discovery: ok"
}

run_mux_context_check() {
  local tmp_home active_scene explicit_scene fallback_scene patch_path
  tmp_home="$(mktemp -d /tmp/gameterm-scene-mux-context-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  active_scene="${tmp_home}/active-mux-workspace.json"
  explicit_scene="${tmp_home}/explicit-cwd-mux-workspace.json"
  fallback_scene="${tmp_home}/fallback-workspace.json"
  patch_path="${tmp_home}/active-mux-workspace.patch.json"

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --fixture-context "${fixture_root}/mux-context-active.json" \
    >"${tmp_home}/active-context.json"
  jq -e '
    .source == "fixture"
    and .available == true
    and .pane_id == 231
    and .mux_window_id == 7
    and .pane_cwd == "'"${repo_root}"'"
    and .foreground_process_name == "zsh"
  ' "${tmp_home}/active-context.json" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --fixture-context "${fixture_root}/mux-context-active.json" \
    --format args \
    >"${tmp_home}/active-context.args"
  grep -q -- '--pane-id 231' "${tmp_home}/active-context.args"
  grep -q -- '--pane-cwd' "${tmp_home}/active-context.args"

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --cli-list-json "${fixture_root}/mux-list-active.json" \
    >"${tmp_home}/cli-list-context.json"
  jq -e '
    .source == "gameterm-cli"
    and .available == true
    and .pane_id == 231
    and .mux_window_id == 7
    and .pane_cwd == "'"${repo_root}"'"
  ' "${tmp_home}/cli-list-context.json" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --gameterm-bin /tmp/gameterm-scene-missing-gameterm-bin \
    --allow-missing \
    >"${tmp_home}/missing-cli-context.json"
  jq -e '
    .source == "gameterm-cli"
    and .available == false
    and (.warnings | length == 1)
  ' "${tmp_home}/missing-cli-context.json" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    discover \
    --fixture-context "${fixture_root}/mux-context-active.json" \
    --scene-output "${active_scene}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-author.sh" validate "${active_scene}" >/dev/null
  jq -e '
    any(.variables[]; .key == "pane_context" and .value == {"Text": "provided"})
    and any(.variables[]; .key == "discovery_source" and .value == {"Text": "pane_cwd"})
    and any(.variables[]; .key == "active_pane_id" and .value == {"Number": 231})
    and any(.variables[]; .key == "active_mux_window_id" and .value == {"Number": 7})
    and any(.variables[]; .key == "process_phase" and .value == {"Text": "running"})
    and any(.entities[]; .id == "discovered-pane"
      and (.metadata | any(.[0] == "cwd" and .[1] == "'"${repo_root}"'"))
      and (.metadata | any(.[0] == "progress" and .[1] == "None")))
    and any(.entities[]; .id == "discovered-process"
      and .label == "zsh"
      and (.metadata | any(.[0] == "foreground_process_path" and .[1] == "/bin/zsh")))
  ' "${active_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    discover \
    --fixture-context "${fixture_root}/mux-context-active.json" \
    --cwd "${repo_root}" \
    --scene-output "${explicit_scene}" \
    --force >/dev/null
  jq -e '
    any(.variables[]; .key == "discovery_source" and .value == {"Text": "cwd_with_pane_metadata"})
    and any(.variables[]; .key == "pane_context" and .value == {"Text": "provided"})
  ' "${explicit_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    patch \
    --fixture-context "${fixture_root}/mux-context-active.json" \
    --patch-output "${patch_path}" \
    --force >/dev/null
  "${repo_root}/ci/gameterm-scene-patch.sh" validate \
    --scene "${fixture_root}/workspace-agent.json" \
    --patch "${patch_path}" >/dev/null
  jq -e '
    .process_state.entity_id == "scene-verify-process"
    and .process_state.phase == "running"
    and .process_state.command == "zsh"
    and (.variables | any(.key == "active_pane_id" and .value == {"Number": 231}))
    and any(.updates[]; .entity_id == "workspace-gameterm"
      and (.metadata | any(.[0] == "pane_cwd" and .[1] == "'"${repo_root}"'")))
  ' "${patch_path}" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    discover \
    --fixture-context "${fixture_root}/mux-context-missing.json" \
    --allow-missing \
    --cwd "${repo_root}" \
    --scene-output "${fallback_scene}" \
    --force >/dev/null
  jq -e '
    any(.variables[]; .key == "pane_context" and .value == {"Text": "absent"})
    and any(.variables[]; .key == "discovery_source" and .value == {"Text": "cwd"})
  ' "${fallback_scene}" >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    doctor \
    --fixture-context "${fixture_root}/mux-context-active.json" >/dev/null
  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    doctor \
    --fixture-context "${fixture_root}/mux-context-missing.json" \
    --allow-missing >/dev/null

  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --pane-id 231 \
    --mux-window-id 7 \
    --pane-cwd "${repo_root}" \
    --foreground-process-name zsh \
    --foreground-process-path /bin/zsh \
    --pane-progress None \
    >"${tmp_home}/caller-context.json"
  jq -e '
    .source == "caller"
    and .available == true
    and .pane_id == 231
    and .mux_window_id == 7
  ' "${tmp_home}/caller-context.json" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --fixture-context "${fixture_root}/mux-context-invalid-pane.json" \
    >/tmp/gameterm-scene-mux-invalid-pane.out \
    2>/tmp/gameterm-scene-mux-invalid-pane.err
  invalid_pane_rc=$?
  set -e
  if [[ "${invalid_pane_rc}" -eq 0 ]]; then
    echo "expected mux context helper to reject invalid pane id" >&2
    exit 1
  fi
  grep -q "pane_id must be a non-negative integer" \
    /tmp/gameterm-scene-mux-invalid-pane.err

  set +e
  "${repo_root}/ci/gameterm-scene-mux-context.sh" \
    collect \
    --fixture-context "${fixture_root}/mux-context-invalid-cwd.json" \
    >/tmp/gameterm-scene-mux-invalid-cwd.out \
    2>/tmp/gameterm-scene-mux-invalid-cwd.err
  invalid_cwd_rc=$?
  set -e
  if [[ "${invalid_cwd_rc}" -eq 0 ]]; then
    echo "expected mux context helper to reject invalid pane cwd" >&2
    exit 1
  fi
  grep -q "pane cwd does not exist or is not a directory" \
    /tmp/gameterm-scene-mux-invalid-cwd.err

  echo "mux context helper: ok"
}

run_story_state_check() {
  local tmp_home
  local scene_path
  local state_path
  local imported_path
  tmp_home="$(mktemp -d /tmp/gameterm-scene-story-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  scene_path="${tmp_home}/rpg-quest.json"
  state_path="${tmp_home}/story.json"
  imported_path="${tmp_home}/story-imported.json"

  "${repo_root}/ci/gameterm-scene-author.sh" \
    new-template \
    --template rpg-quest \
    "${scene_path}" >/dev/null

  "${repo_root}/ci/gameterm-scene-story.sh" \
    export \
    "${scene_path}" \
    "${state_path}" >/dev/null
  jq empty "${state_path}"
  "${repo_root}/ci/gameterm-scene-story.sh" validate "${state_path}" >/dev/null
  "${repo_root}/ci/gameterm-scene-story.sh" inspect "${state_path}" | grep -qx "quests=1"
  "${repo_root}/ci/gameterm-scene-story.sh" \
    import \
    "${scene_path}" \
    "${state_path}" \
    "${imported_path}" >/dev/null
  cmp "${state_path}" "${imported_path}" >/dev/null
  echo "story state: ok"
}

run_workspace_session_check() {
  local tmp_home scene_path session_path restored_path before_path
  tmp_home="$(mktemp -d /tmp/gameterm-scene-session-verify.XXXXXX)"
  tmp_paths+=("${tmp_home}")
  scene_path="${tmp_home}/rpg-quest.json"
  session_path="${tmp_home}/workspace.session.json"
  restored_path="${tmp_home}/restored.story.json"
  before_path="${tmp_home}/scene-before.json"

  "${repo_root}/ci/gameterm-scene-author.sh" \
    new-template \
    --template rpg-quest \
    "${scene_path}" >/dev/null
  cp "${scene_path}" "${before_path}"

  "${repo_root}/ci/gameterm-scene-session.sh" \
    save \
    --scene "${scene_path}" \
    --workspace-root "${repo_root}" \
    --output "${session_path}" >/dev/null
  jq -e '
    .workspace_session_version == 1
    and .workspace_root == "'"${repo_root}"'"
    and .story_state.story_state_version == 1
    and (.story_state.rpg.quests | length) == 1
  ' "${session_path}" >/dev/null
  "${repo_root}/ci/gameterm-scene-session.sh" validate \
    --session "${session_path}" >/dev/null
  "${repo_root}/ci/gameterm-scene-session.sh" inspect \
    --session "${session_path}" | grep -qx "quests=1"

  "${repo_root}/ci/gameterm-scene-session.sh" \
    restore \
    --scene "${scene_path}" \
    --session "${session_path}" \
    --output "${restored_path}" >/dev/null
  jq -e '.story_state_version == 1 and (.rpg.quests | length) == 1' \
    "${restored_path}" >/dev/null
  cmp "${scene_path}" "${before_path}" >/dev/null

  set +e
  "${repo_root}/ci/gameterm-scene-session.sh" \
    save \
    --scene "${scene_path}" \
    --workspace-root "${repo_root}" \
    --output "${session_path}" \
    >/tmp/gameterm-scene-session-overwrite.out \
    2>/tmp/gameterm-scene-session-overwrite.err
  overwrite_rc=$?
  set -e
  if [[ "${overwrite_rc}" -eq 0 ]]; then
    echo "expected workspace session overwrite protection to fail" >&2
    exit 1
  fi

  echo "workspace session: ok"
}

run_onboarding_check() {
  local doc="${repo_root}/docs/gameterm-scene-onboarding.md"
  local pattern
  test -f "${doc}"
  for pattern in "${onboarding_required_patterns[@]}"; do
    grep -q "${pattern}" "${doc}"
  done
  grep -q 'gameterm-scene-onboarding.md' \
    "${repo_root}/docs/gameterm-scene-mode.md"
  echo "onboarding: ok"
}

run_smoke_asset_check() {
  local scenarios
  "${repo_root}/ci/gameterm-scene-smoke.sh" --check-assets >/dev/null
  scenarios="$("${repo_root}/ci/gameterm-scene-smoke.sh" --list-scenarios)"
  for scenario in \
    renderer-rows \
    guarded-input \
    run-command-targets \
    overlay-cleanup \
    vertical-slice \
    workspace-agent \
    workspace-discovery \
    agent-lifecycle \
    authoring-loop \
    patch-inbox \
    mux-patch \
    process-state
  do
    grep -qx "${scenario}" <<<"${scenarios}"
    "${repo_root}/ci/gameterm-scene-smoke.sh" \
      --describe-scenario "${scenario}" | grep -q "Expected status:"
  done
  "${repo_root}/ci/gameterm-scene-smoke.sh" \
    --describe-scenario guarded-input | grep -q "Layer story transitioned"
  "${repo_root}/ci/gameterm-scene-smoke.sh" \
    --describe-scenario authoring-loop | grep -q "Story state imported"
  "${repo_root}/ci/gameterm-scene-smoke.sh" \
    --describe-scenario process-state | grep -q "typed process state"
  "${repo_root}/ci/gameterm-scene-smoke.sh" \
    --describe-scenario workspace-agent | grep -q "Agent/Workspace"
  "${repo_root}/ci/gameterm-scene-smoke.sh" \
    --describe-scenario workspace-discovery | grep -q "generated workspace"
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
  run_vn_script_import_check
  run_vn_asset_intake_check
  run_vn_demo_install_check
  run_patch_check
  run_agent_check
  run_workspace_discovery_check
  run_mux_context_check
  run_story_state_check
  run_workspace_session_check
  run_onboarding_check
  run_smoke_asset_check
  for fixture in basic navigate invalid sprites missing-sprite run-command-targets layered-mode vertical-slice authoring-loop game-states chained-transitions workspace-agent multi-agent-coordination renpy-demo; do
    run_fixture_setup_check "${fixture}"
  done
  run_cargo_checks
}

cd "${repo_root}"

case "${mode}" in
  all)
    run_all
    ;;
  basic|navigate|invalid|sprites|missing-sprite|run-command-targets|layered-mode|vertical-slice|authoring-loop|game-states|chained-transitions|workspace-agent|multi-agent-coordination|renpy-demo)
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
