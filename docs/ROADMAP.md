# Roadmap

What Ferry could add to cover a PHP upgrade as completely as possible. The list comes from three research passes in October 2026: PHP core changes from 7.0 to 8.5, everything outside the language that breaks during an upgrade, and the tools people use for upgrades today. Each item names what it would catch, how Ferry could detect it, and a rough size: **S** is up to a week, **M** one to three weeks, **L** more than three.

Items that need verification before they're built are marked *(verify)*.

## 1. Language changes Ferry doesn't detect yet

Ferry covers roughly half of the PHP 7.0–8.5 change catalog. Function, constant and syntax coverage is strong. The gaps are mostly runtime errors that can be found statically and are common in PHP 5.x and 7.x code. Most of them can be built on the project index (`index.rs`, which already knows parents, traits, static methods and signatures) and the existing guard parsing (`guards.rs`).

### Highest value

| Change | Version | Detection | Size |
|---|---|---|---|
| `fn` and `match` as class names; `use ... as Match` | 7.4, 8.0 | Extend `RESERVED_CLASS_NAMES` | S |

### Cheap additions to existing tables

- Argument-form deprecations from 8.1 to 8.5: `pg_*` without a connection, `imagepolygon` with four arguments, `ldap_connect($host, $port)`, one-argument `ReflectionProperty::setValue()`, `new ReflectionMethod('A::b')`, two-argument `stream_context_set_option()`, the ISO string `DatePeriod` constructor and others.
- Signatures removed in 8.0: one-argument `mb_parse_str`, the encoding as the third argument of `mb_strrpos`, `vsprintf` with a non-array, `spl_autoload_register(..., false)` and others.
- Removed symbols: the DOM classes removed in 8.0, `SplFixedArray`'s iterator methods, the `MYSQLI_*` constants removed in 8.4, `SID` (8.4), `NIL` (8.1), the `PDO::pgsql*` and `PDO::sqlite*` methods (8.5).
- `DEPRECATED_INI`: the 7.x removals, `allow_url_include` (7.4), the remaining 8.4 session keys, `intl.error_level` and `report_memleaks` (8.5).
- `new FilesystemIterator($path, $flags)` without `SKIP_DOTS` (8.2).
- Literal arguments that now throw `ValueError`: `explode('', $s)`, `str_repeat($s, -1)`, `range(1, 10, 0)` and others.

### Silent behavior changes worth a review-level finding

- The default flags of `htmlspecialchars` and related functions (8.1).
- Static variables in inherited methods are shared between subclasses (8.1), which breaks per-class singletons.
- An integer needle in `strpos` and related functions now means the digits, not a character code (8.0).
- PDO (8.0) and mysqli (8.1) throw exceptions by default; code that checks `=== false` stops working.
- Locale-dependent code: since 8.0 floats are converted to strings without the locale, and since 8.2 `strtolower` and related functions are ASCII-only. This matters for Turkish (`tr_TR`) projects.
- Sort callbacks that return a bool (8.0 deprecation, plus stable sorting).
- Literal regex patterns that PCRE2 rejects (`[\w-.]` in 7.3, invalid escapes in 8.0, `{,3}` in 8.4).

### Out of reach for static analysis

String-to-number comparisons with `==` (8.0), `null` passed to built-ins from variables, dynamic properties on objects of unknown classes, autovivification from `false`, undefined variables and keys, and data stored in serialized form. These need runtime evidence; see section 3.

## 2. Outside the language

| Addition | Why | Size |
|---|---|---|
| **Composer platform pitfalls.** `config.platform.ext-*` pinned below the target, `platform-check` disabled, a Composer 1 lock file (no `plugin-api-version`), plugins that need `composer-plugin-api ^1` (`config.platform.php` is done). | Each one makes Composer install packages for the wrong platform | S |
| **More version pins and a bump action.** DDEV, Lando, Platform.sh and versioned PHP-FPM socket paths, plus a button that updates pins to the target (Dockerfile, Compose, CI files, `.tool-versions` and `.php-version` are reported already). | The upgrade isn't done until these change | S |
| **More platforms.** Joomla 3/4/5, Moodle (its own `admin/environment.xml`), MediaWiki, phpBB, CodeIgniter, Yii, CakePHP, OpenCart. Keep the support ranges in a data file with a source URL and a verified date for each row. *(verify each range from official pages; Drupal 7, Magento and PrestaShop are already done)* | Legacy CMS users are Ferry's core audience | M |
| **WordPress plugins and themes.** Read plugin and theme headers (`Version`, `Requires PHP`) and, opt-in, query api.wordpress.org for the latest version, its PHP requirement and whether the plugin was closed. Say "update first" or "fix it yourself" for each. | Most breakage after a host's forced PHP upgrade comes from plugins. WP Engine's checker is unmaintained, stuck at PHP 8.0, and rated 2.7/5 | M |
| **Polyfill cleanup.** `symfony/polyfill-php*` and `random_compat` become no-ops once the target passes their version. | Smaller dependency tree | S |
| **Hosting probe.** A generated one-file `ferry-probe.php` the user uploads to the server; its JSON output (PHP version, SAPI, extensions, `disable_functions`, ini values) is imported into Ferry. | Covers shared hosting, which Ferry can't reach | S |

## 3. Workflow features

Competing tools each cover part of an upgrade: PHPCompatibility (detection only; 10.0 still in alpha), Rector (fixes; often changes more than needed), PHPStan and Psalm (types and baselines), Laravel Shift (Laravel only, PR-based, paid), PhpStorm and Qodana, Exakat, Tideways (production deprecations). None combines a readiness score, static scanning, safe fixes, dependency checks, runtime evidence and CI gating in a private desktop app. Ferry has the first four. The missing pieces are all workflow:

| # | Feature | Why | Size |
|---|---|---|---|
| 1 | **Project config (`.ferry/`)**: target, excluded paths, ignores, baseline, editor, test command; committed to git | Foundation for most items below | S |
| 3 | **Custom editor command or URL** for editors outside the built-in list (VS Code, Cursor, PhpStorm, Sublime Text and Zed are done) | Covers Neovim, Emacs and in-house setups | S |
| 5 | **JUnit, Checkstyle and CSV output** with stable fingerprints | GitLab, Jenkins, spreadsheets | S |
| 7 | **Import runtime deprecations**: PHP error logs, PHPUnit and Symfony bridge output, Monolog JSON. Merge them with static findings, mark matches "confirmed at runtime" with a hit count, and separate own code from vendor code | No tool merges runtime evidence with static findings; it covers what static analysis can't see (section 1) | M |
| 8 | **CSV with a `git blame` owner column** | Assign work across a team | S |
| 9 | **Git integration**: refuse to apply on a dirty tree, optional branch per batch, generated commit message listing the Rector rules | Small, reviewable commits | M |
| 10 | **Apply by rule or by hunk, and Rector level stepping** (`withPhpLevel`) | Answers "Rector changed too much" | S–M |
| 11 | **Lint and run tests with a local PHP of the target version** (Herd, Homebrew, phpenv are already detected) | Most of "Live test" without Docker | M |
| 12 | **Compare targets side by side**: score, breaking findings and blocking packages for 8.2–8.5 | Helps choose between 8.3 and going straight to 8.5 | S–M |
| 13 | **Docker "Live test"**: official `php:X.Y-cli` images with the project's extensions, `php -l`, then the test suite on the current and target versions, feeding item 7 | Covers behavior changes; turns "Coming soon" into a real feature | L |
| 14 | **AI help for manual findings**, opt-in per project, own API key or a local model, through the same preview, backup and lint flow | Manual findings are where time goes; the privacy promise needs an explicit opt-in | M–L |
| 15 | Issue creation (GitHub/GitLab), incremental cached scans for very large projects, scanning `vendor/` for library parents, multiple composer roots, more interface languages | Teams, enterprise, reach | M each |

## 4. Benchmarks

Today Ferry is measured against PHP itself in two ways:

- `compile_time_findings_match_php` lints thirteen old projects with `php -l` and compares compile-time findings. With `scripts/fetch-corpus.sh` the corpus covers WordPress, Drupal, CodeIgniter, phpBB, Joomla, Magento, PrestaShop, Laravel, Symfony, CakePHP, Yii, MediaWiki and Moodle.
- `every_deprecated_function_is_reported` and `every_deprecated_method_and_constant_is_reported` ask the PHP binary which built-in symbols are deprecated and check that Ferry reports each one with the same version.

Still missing is a benchmark for runtime findings (`null` to built-ins, dynamic properties, deprecated call forms). The plan: run the test suites of a few well-known projects released before PHP 8.4, such as Monolog 3.5, Carbon 3.0, CommonMark 2.4 and Laravel 11.0, on PHP 8.5 with PHPUnit's `--display-deprecations`, and compare the reported file and line pairs with Ferry's findings. This needs `composer install` and executes those projects' code, so it should run in a container or with explicit approval.
