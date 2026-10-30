#!/usr/bin/env bash
set -euo pipefail
dest="${1:?usage: scripts/fetch-corpus.sh <folder>}"
mkdir -p "$dest"
clone() {
  [ -d "$dest/$3" ] || git clone -q --depth 1 --branch "$2" "https://github.com/$1.git" "$dest/$3"
}
clone WordPress/WordPress 4.9 wordpress
clone drupal/drupal 7.50 drupal
clone bcit-ci/CodeIgniter 3.0.0 codeigniter
clone phpbb/phpbb release-3.2.0 phpbb
echo "Corpus ready in $dest"
