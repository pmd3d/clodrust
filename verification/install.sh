#!/usr/bin/env bash
set -euo pipefail
repo_dir=$(cd "$(dirname "$0")/.." && pwd)
workspace_dir=$(dirname "$repo_dir")
if [[ ! -d "$HOME/.rustup" && -d "$workspace_dir/.rustup" ]]; then
  export RUSTUP_HOME="${RUSTUP_HOME:-$workspace_dir/.rustup}"
  export CARGO_HOME="${CARGO_HOME:-$workspace_dir/.cargo}"
  export PATH="$CARGO_HOME/bin:$PATH"
fi
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
  echo 'Automatic setup supports x86_64 Linux/WSL. See verification/README.md for other platforms.' >&2
  exit 2
fi
for program in python3 curl unzip sha256sum rustup; do
  command -v "$program" >/dev/null || { echo "Missing prerequisite: $program" >&2; exit 2; }
done
read -r verus_version rust_version checksum < <(python3 - "$repo_dir/verification/toolchain.json" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
print(p['verus_version'], p['rust_version'], p['linux_x86_64_sha256'])
PY
)
install_dir="$repo_dir/.verus"
mkdir -p "$install_dir"
archive="$install_dir/verus-$verus_version-x86-linux.zip"
if [[ ! -f "$archive" ]]; then
  curl --fail --location --retry 2 \
    "https://github.com/verus-lang/verus/releases/download/release%2F$verus_version/verus-$verus_version-x86-linux.zip" \
    -o "$archive.part"
  mv "$archive.part" "$archive"
fi
printf '%s  %s\n' "$checksum" "$archive" | sha256sum --check --status || {
  echo "Checksum mismatch: remove $archive and retry." >&2; exit 1;
}
if [[ ! -x "$install_dir/verus-x86-linux/verus" ]] || ! python3 - "$install_dir/verus-x86-linux/version.json" "$verus_version" <<'PY_VERSION'
import json, sys
try:
    installed = json.load(open(sys.argv[1]))['verus']['version']
except (OSError, KeyError, ValueError):
    sys.exit(1)
sys.exit(0 if installed == sys.argv[2] else 1)
PY_VERSION
then
  unzip -oq "$archive" -d "$install_dir"
fi
rustup toolchain install "$rust_version" --profile minimal --no-self-update
"$install_dir/verus-x86-linux/verus" --version
echo 'Ready. Run: python3 verification/verify.py'
