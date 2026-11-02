#!/usr/bin/env bash
# Builds one image per PHP version with the same extensions and writes its reflection dump.
# Usage: scripts/php-symbols/dump.sh <output-folder> [versions...]
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${1:?usage: dump.sh <output-folder> [versions...]}"
shift
versions=("$@")
[ ${#versions[@]} -gt 0 ] || versions=(7.0 7.1 7.2 7.3 7.4 8.0 8.1 8.2 8.3 8.4 8.5)
mkdir -p "$out"
for v in "${versions[@]}"; do
  echo "== PHP $v"
  docker build -q --build-arg PHP="$v" -t "ferry-php-symbols:$v" "$here" >/dev/null
  docker run --rm "ferry-php-symbols:$v" > "$out/php-$v.json"
  php -r '$d = json_decode(file_get_contents($argv[1]), true); printf("   %s: %d functions, %d classes, %d constants, %d extensions\n", $d["version"], count($d["functions"]), count($d["classes"]), count($d["constants"]), count($d["extensions"]));' "$out/php-$v.json"
done
