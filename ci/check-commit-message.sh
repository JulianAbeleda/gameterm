#!/usr/bin/env bash
set -euo pipefail

allowed_prefix_re='identity|repo|build|ci|release|docs|config|lua|mux|term|render|visual|window|pty|ssh|test'
subject_re="^\\[(${allowed_prefix_re})\\] (NFC - )?[[:lower:][:digit:]].+"

usage() {
  cat >&2 <<'USAGE'
usage:
  ci/check-commit-message.sh <commit-msg-file>
  ci/check-commit-message.sh <git-revision-range>

Examples:
  ci/check-commit-message.sh .git/COMMIT_EDITMSG
  ci/check-commit-message.sh origin/main..HEAD

Rules:
  [subsystem] concise summary
  [subsystem] NFC - concise non-functional summary
USAGE
}

is_allowed_subject() {
  local subject="$1"

  case "$subject" in
    Merge\ *|Revert\ \"*|\[pre-commit.ci\]\ *) return 0 ;;
  esac

  [[ "$subject" =~ $subject_re ]]
}

check_subject() {
  local source="$1"
  local subject="$2"

  if is_allowed_subject "$subject"; then
    return 0
  fi

  cat >&2 <<EOF
invalid commit subject in $source:
  $subject

Expected one of:
  [visual] add Scene runtime guard
  [visual] NFC - split Scene runtime helpers
  [docs] update Scene handoff

Allowed prefixes:
  ${allowed_prefix_re//|/, }
EOF
  return 1
}

check_message_file() {
  local message_file="$1"
  local subject

  subject="$(sed -n '1p' "$message_file")"
  check_subject "$message_file" "$subject"
}

check_range() {
  local range="$1"
  local failed=0
  local commits

  commits="$(git rev-list --reverse "$range")"
  if [[ -z "$commits" ]]; then
    echo "no commits to check for range $range"
    return 0
  fi

  while IFS= read -r commit; do
    local subject
    subject="$(git log --format=%s -n 1 "$commit")"
    if ! check_subject "$commit" "$subject"; then
      failed=1
    fi
  done <<<"$commits"

  return "$failed"
}

main() {
  if [[ "$#" -ne 1 ]]; then
    usage
    exit 2
  fi

  if [[ -f "$1" ]]; then
    check_message_file "$1"
    return
  fi

  check_range "$1"
}

main "$@"
