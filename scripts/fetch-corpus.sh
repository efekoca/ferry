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
clone joomla/joomla-cms 3.6.0 joomla
clone OpenMage/magento-mirror 1.9.3.10 magento
clone PrestaShop/PrestaShop 1.6.1.24 prestashop
clone laravel/framework v5.4.0 laravel
echo "Corpus ready in $dest"
