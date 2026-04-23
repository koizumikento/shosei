#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
  cat <<'HELP'
Usage: scripts/validate-real-kindle-previewer.sh [WORKDIR]

Creates a minimal Kindle-enabled shosei project, enables validation.kindle_previewer,
and runs `shosei validate --json` against the real local toolchain.

This hook is intentionally not required by CI. It is for release operators or
maintainers who have the real proprietary Kindle Previewer installed and want
device-oriented conversion evidence before Kindle handoff.
HELP
  exit 0
fi

cleanup_dir=""
if [ -n "${1:-}" ]; then
  workdir="$1"
  mkdir -p "$workdir"
else
  workdir="$(mktemp -d "${TMPDIR:-/tmp}/shosei-real-kindle-previewer.XXXXXX")"
  cleanup_dir="$workdir"
fi

mkdir -p "$workdir/manuscript"
cat > "$workdir/manuscript/01.md" <<'MARKDOWN'
# Chapter 1

This is a minimal Kindle Previewer validation fixture.
MARKDOWN

cat > "$workdir/book.yml" <<'YAML'
project:
  type: novel
  vcs: git
book:
  title: "Kindle Previewer Evidence"
  authors:
    - "Shosei"
  reading_direction: ltr
layout:
  binding: left
manuscript:
  chapters:
    - manuscript/01.md
outputs:
  kindle:
    enabled: true
    target: kindle-ja
validation:
  strict: true
  epubcheck: false
  kindle_previewer: true
git:
  lfs: false
YAML

stdout_path="$workdir/validate-stdout.json"
stderr_path="$workdir/validate-stderr.log"

set +e
(
  cd "$repo_root"
  cargo run -q -p shosei-cli --bin shosei -- validate --json --path "$workdir"
) >"$stdout_path" 2>"$stderr_path"
status=$?
set -e

if [ "$status" -eq 0 ] \
  && grep -Eq '"name"[[:space:]]*:[[:space:]]*"kindle-previewer"' "$stdout_path" \
  && grep -Eq '"status"[[:space:]]*:[[:space:]]*"passed"' "$stdout_path"; then
  echo "Kindle Previewer real-tool validation passed."
  echo "Workspace: $workdir"
  echo "Report: $workdir/dist/reports/default-validate.json"
  echo "Validator log: $workdir/dist/logs/default-kindle-previewer-validate.log"
  [ -z "$cleanup_dir" ] || echo "Temporary workspace retained for evidence."
  exit 0
fi

if grep -Eq '"name"[[:space:]]*:[[:space:]]*"kindle-previewer"' "$stdout_path" \
  && grep -Eq '"status"[[:space:]]*:[[:space:]]*"missing-tool"' "$stdout_path"; then
  echo "Kindle Previewer was not found by shosei doctor/validate." >&2
  echo "Install Kindle Previewer on this host or run this hook on a supported macOS/Windows machine." >&2
  echo "Workspace retained for inspection: $workdir" >&2
  exit 2
fi

echo "Kindle Previewer real-tool validation did not pass." >&2
echo "Exit status: $status" >&2
echo "Workspace retained for inspection: $workdir" >&2
echo "stdout: $stdout_path" >&2
echo "stderr: $stderr_path" >&2
exit 1
