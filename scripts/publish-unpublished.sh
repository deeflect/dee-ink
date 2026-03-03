#!/usr/bin/env bash
# Publish all unpublished dee-* crates with basic rate-limit aware retries.
#
# Usage:
#   bash scripts/publish-unpublished.sh [--dry-run] [--max-retries N] [--base-delay SEC]

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CRATES_DIR="$REPO_ROOT/crates"

DRY_RUN=0
MAX_RETRIES=8
BASE_DELAY=15

usage() {
  cat <<'EOF'
Usage: bash scripts/publish-unpublished.sh [options]

Options:
  --dry-run           Show what would be published without calling cargo publish
  --max-retries N     Max retries for rate limits/transient failures (default: 8)
  --base-delay SEC    Initial retry delay in seconds (default: 15)
  -h, --help          Show this help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --max-retries)
      MAX_RETRIES="${2:-}"
      shift 2
      ;;
    --base-delay)
      BASE_DELAY="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo is required but not found in PATH." >&2
  exit 1
fi

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required but not found in PATH." >&2
  exit 1
fi

if ! [[ "$MAX_RETRIES" =~ ^[0-9]+$ ]] || ! [[ "$BASE_DELAY" =~ ^[0-9]+$ ]]; then
  echo "--max-retries and --base-delay must be non-negative integers." >&2
  exit 1
fi

is_retryable_error() {
  local file="$1"
  grep -qiE "rate.?limit|too many requests|429|timed? out|timeout|connection reset|connection refused|503|502|504|service unavailable|temporary failure" "$file"
}

extract_retry_after() {
  local file="$1"
  local sec
  sec="$(grep -Eio 'retry[- ]?after[^0-9]*[0-9]+' "$file" | grep -Eo '[0-9]+' | head -n1 || true)"
  if [[ -n "$sec" ]]; then
    echo "$sec"
  else
    echo ""
  fi
}

crates_io_has_version() {
  local crate="$1"
  local version="$2"
  local tries=0
  local delay="$BASE_DELAY"

  while true; do
    local code
    code="$(curl -sS -o /dev/null -w '%{http_code}' "https://crates.io/api/v1/crates/${crate}/${version}" || echo "000")"
    case "$code" in
      200) return 0 ;;
      404) return 1 ;;
      429|500|502|503|504|000)
        if (( tries >= MAX_RETRIES )); then
          echo "Failed checking crates.io for ${crate}@${version} after ${MAX_RETRIES} retries (last status: $code)." >&2
          return 2
        fi
        local jitter=$((RANDOM % 7))
        local wait_sec=$((delay + jitter))
        echo "Check ${crate}@${version}: crates.io status $code, retrying in ${wait_sec}s..." >&2
        sleep "$wait_sec"
        tries=$((tries + 1))
        delay=$((delay * 2))
        if (( delay > 300 )); then delay=300; fi
        ;;
      *)
        echo "Unexpected crates.io status $code for ${crate}@${version}." >&2
        return 2
        ;;
    esac
  done
}

publish_crate() {
  local crate="$1"
  local attempt=0
  local delay="$BASE_DELAY"
  local log_file
  log_file="$(mktemp)"
  trap 'rm -f "$log_file"' RETURN

  while true; do
    if (( DRY_RUN == 1 )); then
      echo "[DRY-RUN] cargo publish -p $crate --locked"
      return 0
    fi

    echo "Publishing $crate (attempt $((attempt + 1)))..."
    if cargo publish -p "$crate" --locked 2>&1 | tee "$log_file"; then
      echo "Published $crate"
      return 0
    fi

    if grep -qiE "already uploaded|already exists|previously published" "$log_file"; then
      echo "$crate already published while running; skipping."
      return 0
    fi

    if ! is_retryable_error "$log_file"; then
      echo "Non-retryable publish failure for $crate" >&2
      return 1
    fi

    if (( attempt >= MAX_RETRIES )); then
      echo "Hit retry limit publishing $crate" >&2
      return 1
    fi

    local retry_after
    retry_after="$(extract_retry_after "$log_file")"
    local jitter=$((RANDOM % 7))
    local wait_sec
    if [[ -n "$retry_after" ]]; then
      wait_sec=$((retry_after + jitter))
    else
      wait_sec=$((delay + jitter))
    fi

    echo "Retryable error for $crate. Sleeping ${wait_sec}s..."
    sleep "$wait_sec"
    attempt=$((attempt + 1))
    delay=$((delay * 2))
    if (( delay > 300 )); then delay=300; fi
  done
}

mapfile -t manifests < <(find "$CRATES_DIR" -maxdepth 2 -type f -name Cargo.toml | sort)

to_publish=()
already_published=0
skipped_non_publishable=0

for manifest in "${manifests[@]}"; do
  crate_name="$(sed -n 's/^name = "\(.*\)"/\1/p' "$manifest" | head -n1)"
  version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$manifest" | head -n1)"
  publish_field="$(sed -n 's/^publish = \(.*\)$/\1/p' "$manifest" | head -n1)"

  if [[ -z "$crate_name" || -z "$version" ]]; then
    echo "Skipping malformed manifest: $manifest" >&2
    continue
  fi

  if [[ "$publish_field" == "false" ]]; then
    echo "Skip $crate_name@$version (publish = false)"
    skipped_non_publishable=$((skipped_non_publishable + 1))
    continue
  fi

  if crates_io_has_version "$crate_name" "$version"; then
    echo "Skip $crate_name@$version (already on crates.io)"
    already_published=$((already_published + 1))
  else
    to_publish+=("$crate_name")
  fi
done

echo
echo "Crates queued for publish: ${#to_publish[@]}"
for crate in "${to_publish[@]}"; do
  echo "  - $crate"
done
echo

published_now=0
for crate in "${to_publish[@]}"; do
  publish_crate "$crate"
  published_now=$((published_now + 1))
done

echo
echo "Done."
echo "  published now: $published_now"
echo "  already published: $already_published"
echo "  publish=false skipped: $skipped_non_publishable"
