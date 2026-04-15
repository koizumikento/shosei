#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  scripts/generate-scoop-manifest.sh \
    --tag <release-tag> \
    --repo <owner/repo> \
    --windows-sha <sha256> \
    --output <path>
EOF
}

tag=""
repo=""
windows_sha=""
output=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tag)
      tag="${2:-}"
      shift 2
      ;;
    --repo)
      repo="${2:-}"
      shift 2
      ;;
    --windows-sha)
      windows_sha="${2:-}"
      shift 2
      ;;
    --output)
      output="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 3
      ;;
  esac
done

if [[ -z "${tag}" || -z "${repo}" || -z "${windows_sha}" || -z "${output}" ]]; then
  echo "missing required arguments" >&2
  usage >&2
  exit 3
fi

version="${tag#v}"
if [[ "${version}" == "${tag}" ]]; then
  echo "tag must start with 'v': ${tag}" >&2
  exit 3
fi

if ! [[ "${windows_sha}" =~ ^[0-9a-f]{64}$ ]]; then
  echo "invalid Windows SHA256: ${windows_sha}" >&2
  exit 3
fi

mkdir -p "$(dirname "${output}")"

cat > "${output}" <<EOF
{
  "version": "${version}",
  "description": "Rust CLI for Japanese publishing workflows",
  "homepage": "https://github.com/${repo}",
  "license": "MIT",
  "architecture": {
    "64bit": {
      "url": "https://github.com/${repo}/releases/download/${tag}/shosei-${tag}-x86_64-pc-windows-msvc.zip",
      "hash": "${windows_sha}"
    }
  },
  "bin": "shosei.exe",
  "checkver": {
    "github": "https://github.com/${repo}"
  },
  "autoupdate": {
    "architecture": {
      "64bit": {
        "url": "https://github.com/${repo}/releases/download/v\$version/shosei-v\$version-x86_64-pc-windows-msvc.zip"
      }
    }
  }
}
EOF
