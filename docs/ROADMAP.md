# Roadmap

What Ferry could add to cover a PHP upgrade as completely as possible. The list comes from three research passes in October 2026: PHP core changes from 7.0 to 8.5, everything outside the language that breaks during an upgrade, and the tools people use for upgrades today. Each item names what it would catch, how Ferry could detect it, and a rough size: **S** is up to a week, **M** one to three weeks, **L** more than three.

Items that need verification before they're built are marked *(verify)*.

## 1. Language changes Ferry doesn't detect yet

Function, constant, syntax and literal-argument coverage is strong, and the common silent changes are reviewed, including locale-dependent code for Turkish projects. What's left needs runtime evidence.

### Out of reach for static analysis

String-to-number comparisons with `==` (8.0), `null` passed to built-ins from variables, dynamic properties on objects of unknown classes, autovivification from `false`, undefined variables and keys, and data stored in serialized form. These need runtime evidence; see section 3.

## 2. Outside the language

| Addition | Why | Size |
|---|---|---|
| **Platform checks.** Re-check the MediaWiki matrix and Joomla 6 once they publish firm ranges. | Legacy CMS users are Ferry's core audience | S |

## 3. Workflow features

Competing tools each cover part of an upgrade: PHPCompatibility (detection only; 10.0 still in alpha), Rector (fixes; often changes more than needed), PHPStan and Psalm (types and baselines), Laravel Shift (Laravel only, PR-based, paid), PhpStorm and Qodana, Exakat, Tideways (production deprecations). None combines a readiness score, static scanning, safe fixes, dependency checks, runtime evidence and CI gating in a private desktop app. Ferry has the first four. The missing pieces are all workflow:

| # | Feature | Why | Size |
|---|---|---|---|
| 13 | **Docker comparison**: `ferry-cli compare` runs `php -l` and the test suite in official `php:X.Y-cli` images on the current and target versions. The app has a screen for it, and new messages are added to the findings. Extensions the project uses are built into a local image, and pages and database tables are compared, using a dump, a copy of a local database, or test data from migrations and seeders. Ready-made fixes can be tried in a temporary copy and compared again before they're written | Covers behavior changes; turns "Coming soon" into a real feature | M |
| 14 | **AI help for manual findings**, opt-in per project, own API key or a local model, through the same preview, backup and lint flow | Manual findings are where time goes; the privacy promise needs an explicit opt-in | M–L |
| 15 | Issue creation (GitHub/GitLab), incremental cached scans for very large projects, scanning `vendor/` for library parents, multiple composer roots, more interface languages | Teams, enterprise, reach | M each |

## 4. Benchmarks

Today Ferry is measured against PHP itself in two ways:

- `compile_time_findings_match_php` lints thirteen old projects with `php -l` and compares compile-time findings. With `scripts/fetch-corpus.sh` the corpus covers WordPress, Drupal, CodeIgniter, phpBB, Joomla, Magento, PrestaShop, Laravel, Symfony, CakePHP, Yii, MediaWiki and Moodle.
- `every_deprecated_function_is_reported` and `every_deprecated_method_and_constant_is_reported` ask the PHP binary which built-in symbols are deprecated and check that Ferry reports each one with the same version.

A third, `scripts/runtime-benchmark.sh`, runs a project's PHPUnit tests file by file in Docker on the target PHP and merges the reported deprecations into the scan with `ferry-cli --runtime-log`. First run on PHP 8.5.12 (1 Nov 2026):

| Project | Tests | Deprecations from project code | Already found by the static scan | Missed |
|---|---|---|---|---|
| Monolog 3.5.0 | `tests/Monolog` | 27 | 27 | 0 |
| league/commonmark 2.4.2 | `tests/unit` | 4 | 3 | 1: `ord()` given a multi-byte string |
| Laravel 11.0.0 | `tests/Support` | 73 | 62 | 11: `null` used as an array offset or `array_key_exists()` key through a variable |
| PHPMailer 6.9.1 | `test` | 5 | 5 | 0 |
| Carbon 3.0.0 | `tests` | 7 | 7 | 0 (the suite stops in its bootstrap on 8.5, so only these were reported) |

So 104 of 116 runtime deprecations (90%) were already reported statically. Every miss is an 8.5 deprecation that depends on a runtime value, which is the case runtime log import exists for. Two PHPUnit warnings in Monolog came from the test environment (memory limit, missing git branch) and messages from `vendor/` are counted as outside the project.

Next: add more projects, and run the suites that stop early (Carbon) after fixing their bootstrap.
