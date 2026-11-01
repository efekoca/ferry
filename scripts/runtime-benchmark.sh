#!/usr/bin/env bash
# Usage: scripts/runtime-benchmark.sh <project-dir> <test-dir> [target]
set -euo pipefail
project=$(cd "$1" && pwd)
tests=$2
target=${3:-8.5}
out=$(mktemp -d)
work="$out/project"
cp -R "$project" "$work"
docker run --rm -v "$work":/app -w /app composer:2 \
  composer install --no-interaction --no-progress --no-scripts --no-plugins --ignore-platform-reqs > "$out/install.log" 2>&1
docker run --rm --network none -v "$work":/app -w /app "php:$target-cli" sh -c "
  flags='--display-deprecations --display-warnings --no-progress'
  php vendor/bin/phpunit --version | grep -q 'PHPUnit 9' && flags=''
  for f in \$(find '$tests' -name '*Test.php' | sort); do
    timeout 120 php -d error_reporting=-1 -d display_errors=1 -d memory_limit=1G vendor/bin/phpunit \$flags \"\$f\" 2>&1 || true
  done" > "$out/phpunit.log" 2>&1
cargo run -q --release -p ferry-cli -- scan "$work" --target "$target" --format json --fail-on none \
  --runtime-log "$out/phpunit.log" --output "$out/merged.json"
echo "Results in $out"
