#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
DEMO_DIR="${1:-/Users/mcx/Downloads/ShelfyOrdenDemo}"
SOURCE_A="$DEMO_DIR/source-a"
SOURCE_B="$DEMO_DIR/source-b"
SOURCE_C="$DEMO_DIR/source-c"
DESTINATION="$DEMO_DIR/collected"
CONFIG="$DEMO_DIR/multi-source-move.yaml"

# This script owns the demo directory: reset it so every run is reproducible.
rm -rf "$DEMO_DIR"
mkdir -p "$SOURCE_A/nested" "$SOURCE_B" "$SOURCE_C/nested/deeper" "$DESTINATION"

printf 'alpha text\n' > "$SOURCE_A/alpha.txt"
printf '{"source":"a"}\n' > "$SOURCE_A/nested/alpha.json"
printf 'beta text\n' > "$SOURCE_B/beta.txt"
printf 'beta markdown\n' > "$SOURCE_B/beta.md"
printf 'gamma csv\n1,2\n' > "$SOURCE_C/gamma.csv"
printf 'gamma log\n' > "$SOURCE_C/nested/deeper/gamma.log"

cat > "$CONFIG" <<YAML
rules:
  - name: "Collect every file from multiple sources"
    targets: files
    locations:
      - "$SOURCE_A"
      - "$SOURCE_B"
      - "$SOURCE_C"
    subfolders: true
    actions:
      - move: "$DESTINATION/"
YAML

expected=6
before_count="$(find "$SOURCE_A" "$SOURCE_B" "$SOURCE_C" -type f | wc -l | tr -d ' ')"
if [[ "$before_count" != "$expected" ]]; then
  echo "FAIL: expected $expected fixture files, found $before_count" >&2
  exit 1
fi

cargo run --quiet --manifest-path "$PROJECT_DIR/src-tauri/Cargo.toml" -- \
  --cli orden check "$CONFIG"
cargo run --quiet --manifest-path "$PROJECT_DIR/src-tauri/Cargo.toml" -- \
  --cli orden run "$CONFIG"

remaining_count="$(find "$SOURCE_A" "$SOURCE_B" "$SOURCE_C" -type f | wc -l | tr -d ' ')"
collected_count="$(find "$DESTINATION" -maxdepth 1 -type f | wc -l | tr -d ' ')"

if [[ "$remaining_count" != "0" ]]; then
  echo "FAIL: $remaining_count files remain in source folders" >&2
  exit 1
fi
if [[ "$collected_count" != "$expected" ]]; then
  echo "FAIL: expected $expected files in $DESTINATION, found $collected_count" >&2
  exit 1
fi

for file in alpha.txt alpha.json beta.txt beta.md gamma.csv gamma.log; do
  if [[ ! -f "$DESTINATION/$file" ]]; then
    echo "FAIL: missing collected file $file" >&2
    exit 1
  fi
done

echo "PASS: moved all $expected files from 3 source folders into $DESTINATION"
echo "Config: $CONFIG"
find "$DESTINATION" -maxdepth 1 -type f -print | sort
