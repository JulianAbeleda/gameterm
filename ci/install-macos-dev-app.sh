#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: ci/install-macos-dev-app.sh [OPTIONS]

Builds and installs a local macOS GameTerm.app for development.

Options:
  --install-dir PATH  Directory that will receive GameTerm.app.
                      Default: ~/Applications
  --target-dir PATH   Cargo target directory. Default: target
  --release           Install release binaries instead of debug binaries.
  --no-build          Do not run cargo build before installing.
  --open              Open the installed app after installation.
  --restart           Quit the installed app before replacing it, then reopen it.
  -h, --help          Show this help.
EOF
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
install_dir="${HOME}/Applications"
target_dir="${repo_root}/target"
profile="debug"
build=1
open_app=0
restart_app=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --install-dir)
      install_dir="$2"
      shift 2
      ;;
    --target-dir)
      target_dir="$2"
      shift 2
      ;;
    --release)
      profile="release"
      shift
      ;;
    --no-build)
      build=0
      shift
      ;;
    --open)
      open_app=1
      shift
      ;;
    --restart)
      restart_app=1
      open_app=1
      shift
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

case "${OSTYPE:-}" in
  darwin*) ;;
  *)
    echo "This installer only supports macOS." >&2
    exit 1
    ;;
esac

if [[ "${build}" -eq 1 ]]; then
  build_args=(build -p gameterm-gui -p gameterm -p gameterm-mux-server -p strip-ansi-escapes)
  if [[ "${profile}" == "release" ]]; then
    build_args+=(--release)
  fi
  (cd "${repo_root}" && cargo "${build_args[@]}")
fi

bin_dir="${target_dir}/${profile}"
app_template="${repo_root}/assets/macos/GameTerm.app"
app_path="${install_dir}/GameTerm.app"
macos_dir="${app_path}/Contents/MacOS"
resources_dir="${app_path}/Contents/Resources"

for bin in gameterm gameterm-mux-server gameterm-gui strip-ansi-escapes; do
  if [[ ! -x "${bin_dir}/${bin}" ]]; then
    echo "missing executable: ${bin_dir}/${bin}" >&2
    echo "Run without --no-build, or build ${bin} first." >&2
    exit 1
  fi
done

if [[ "${restart_app}" -eq 1 ]]; then
  pkill -f "${app_path}/Contents/MacOS/gameterm-gui" >/dev/null 2>&1 || true
  sleep 0.3
fi

rm -rf "${app_path}"
mkdir -p "${install_dir}"
cp -R "${app_template}" "${app_path}"

# The template carries optional ANGLE dylibs at the bundle root. The release
# package omits them, and the default macOS renderer does not require them.
rm -f "${app_path}"/*.dylib

mkdir -p "${macos_dir}" "${resources_dir}"
cp -R "${repo_root}/assets/shell-integration/." "${resources_dir}/"
cp -R "${repo_root}/assets/shell-completion" "${resources_dir}/"

if command -v tic >/dev/null 2>&1; then
  mkdir -p "${resources_dir}/terminfo"
  tic -xe gameterm -o "${resources_dir}/terminfo" "${repo_root}/termwiz/data/gameterm.terminfo"
else
  echo "warning: tic not found; skipping bundled terminfo" >&2
fi

for bin in gameterm gameterm-mux-server gameterm-gui strip-ansi-escapes; do
  cp "${bin_dir}/${bin}" "${macos_dir}/${bin}"
  chmod +x "${macos_dir}/${bin}"
done

for bin in gameterm gameterm-mux-server gameterm-gui strip-ansi-escapes; do
  if ! cmp -s "${bin_dir}/${bin}" "${macos_dir}/${bin}"; then
    echo "installed app binary does not match build output: ${bin}" >&2
    exit 1
  fi
done

# Ad-hoc signing keeps local Gatekeeper checks predictable without requiring a
# developer identity.
codesign --force --deep --sign - "${app_path}" >/dev/null

if [[ -x /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister ]]; then
  /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
    -f "${app_path}" >/dev/null 2>&1 || true
fi

echo "Installed ${app_path} from ${bin_dir}"

if [[ "${open_app}" -eq 1 ]]; then
  open "${app_path}"
fi
