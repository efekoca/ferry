# Ferry

PHP projelerini yeni sürümlere güvenle taşıyan masaüstü uygulaması (macOS ve Windows; Linux yol haritasında). Arayüz Türkçe ve İngilizcedir.

Ferry kodu tarar, PHP 7.0'dan 8.5'e kadar kaldırılan ve değişen kullanımları bulur, her bulguyu riskine göre
sınıflandırır (**Otomatik**, **Gözden geçir**, **Elle düzelt**), Rector dönüşümlerini satır satır önizletir ve
onaylanan değişiklikleri uygulamadan önce yedek alır. Böylece her değişiklik tek tıkla geri alınabilir.

## Mimari

```
src/                    React + TypeScript arayüzü (tasarım: docs/DESIGN.md)
src-tauri/              Tauri 2 kabuğu: komutlar, pencere, platform ayarları
crates/ferry-core/    Platformdan bağımsız motor
  lexer, analyzer, rules   Yerleşik uyumluluk tarayıcısı
  rector                   Rector yapılandırması, önizleme, güvenli uygulama
  diff, snapshot           Diff üretimi/uygulaması, geri alma
  deps, version            composer.lock uyumluluğu, Composer kısıtları
  project, toolchain       Framework algılama, PHP/Composer/Rector bulma
```

Rector yalnızca deneme modunda çalışır. Ürettiği diff'ler motor tarafından dosyaya uygulanır, böylece diske yazılan
içerik önizlemede görülenle birebir aynıdır. Önizlemeden sonra değişen bir dosyaya uygulama reddedilir.

## Geliştirme

Gereksinimler: Rust (stable), Node 22, pnpm, PHP 7.4+ ve Composer (otomatik düzeltmeler için).

```bash
pnpm install
pnpm tauri dev        # masaüstü uygulaması
pnpm dev              # yalnızca arayüz, tarayıcıda örnek verilerle
```

## Demo projeleri

`demo/` klasöründe üç örnek proje var: her kuralı tetikleyen düz PHP bir mağaza, WordPress bir blog ve
neredeyse hazır bir Laravel API'si. Değişiklik uygulamadan önce temiz bir kopya oluşturun:

```bash
pnpm demo             # projeleri .demo/ klasörüne sıfırdan kopyalar
```

Ayrıntılar için [demo/README.md](demo/README.md).

## Testler

```bash
pnpm typecheck && pnpm test     # arayüz birim ve akış testleri
cargo test --workspace          # motor, entegrasyon ve IPC sözleşme testleri
```

Gerçek Rector ile uçtan uca testler için Rector'ın kurulu olduğu bir klasör gösterin:

```bash
FERRY_TEST_TOOLS=/yol/tools cargo test --workspace
```

Gerçek pencerede, gerçek backend'le uçtan uca duman testi (yalnızca debug derlemelerde etkin). Betik proje açar,
tarar, gerekirse Rector'ı kurar, önizler, uygular ve geri yükler; ilerlemeyi `[e2e]` satırlarıyla yazar:

```bash
FERRY_E2E=e2e/native-smoke.js FERRY_E2E_ROOT=/yol/php-projesi pnpm tauri dev
```

## CI

GitHub Actions dakikalarını boşa harcamamak için iş yükü kademelidir:

- **Her push ve PR:** Linux'ta arayüz testleri, production build, motor testleri ve gerçek Rector testleri.
- **`main` dalına push:** Ek olarak Windows'ta tüm testler.
- **`v*` etiketi ya da elle çalıştırma:** macOS testleri ve macOS/Windows kurulum paketleri.

Yalnızca Markdown ve `docs/` değişiklikleri CI'ı tetiklemez.

## Yol haritası

- Docker ile iki PHP sürümünde karşılaştırmalı çalışma testi
- "Elle düzelt" bulguları için AI destekli öneriler
- WordPress ve Laravel'e özel kural paketleri
- Linux desteği

## Lisans

[MIT](LICENSE)
