use crate::diff::{self, FileDiff};
use crate::i18n::{Localized, Text};
use crate::rules::FixKind;
use crate::snapshot::fnv1a;
use crate::toolchain::{command, ToolError};
use crate::version::PhpVersion;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const RECTOR_TIMEOUT: Duration = Duration::from_secs(20 * 60);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RectorMode {
    Compat,
    Modernize,
}

const UNVERSIONED_RULES: &[(&str, (u32, u32))] = &[
    ("Rector\\CodingStyle\\Rector\\ArrowFunction\\ArrowFunctionDelegatingCallToFirstClassCallableRector", (8, 1)),
    ("Rector\\CodingStyle\\Rector\\Closure\\ClosureDelegatingCallToFirstClassCallableRector", (8, 1)),
    ("Rector\\CodingStyle\\Rector\\FuncCall\\ClosureFromCallableToFirstClassCallableRector", (8, 1)),
    ("Rector\\CodingStyle\\Rector\\FuncCall\\FunctionFirstClassCallableRector", (8, 1)),
    ("Rector\\TypeDeclaration\\Rector\\ClassMethod\\ReturnNeverTypeRector", (8, 1)),
    ("Rector\\CodingStyle\\Rector\\FuncCall\\ConsistentImplodeRector", (8, 0)),
    ("Rector\\DeadCode\\Rector\\StaticCall\\RemoveParentCallWithoutParentRector", (8, 0)),
];

const MODERNIZATION_RULES: &[&str] = &[
    "Rector\\CodingStyle\\Rector\\ArrowFunction\\ArrowFunctionDelegatingCallToFirstClassCallableRector",
    "Rector\\CodingStyle\\Rector\\Closure\\ClosureDelegatingCallToFirstClassCallableRector",
    "Rector\\CodingStyle\\Rector\\FuncCall\\ClosureFromCallableToFirstClassCallableRector",
    "Rector\\CodingStyle\\Rector\\FuncCall\\FunctionFirstClassCallableRector",
    "Rector\\TypeDeclaration\\Rector\\ClassMethod\\ReturnNeverTypeRector",
    "Rector\\Php52\\Rector\\Property\\VarToPublicPropertyRector",
    "Rector\\Php53\\Rector\\Ternary\\TernaryToElvisRector",
    "Rector\\Php53\\Rector\\FuncCall\\DirNameFileConstantToDirConstantRector",
    "Rector\\Php54\\Rector\\Array_\\LongArrayToShortArrayRector",
    "Rector\\Php55\\Rector\\String_\\StringClassNameToClassConstantRector",
    "Rector\\Php55\\Rector\\FuncCall\\GetCalledClassToStaticClassRector",
    "Rector\\Php55\\Rector\\FuncCall\\GetCalledClassToSelfClassRector",
    "Rector\\Php55\\Rector\\ClassConstFetch\\StaticToSelfOnFinalClassRector",
    "Rector\\Php55\\Rector\\Class_\\ClassConstantToSelfClassRector",
    "Rector\\Php56\\Rector\\FuncCall\\PowToExpRector",
    "Rector\\Php70\\Rector\\Ternary\\TernaryToSpaceshipRector",
    "Rector\\Php70\\Rector\\Ternary\\TernaryToNullCoalescingRector",
    "Rector\\Php70\\Rector\\FuncCall\\MultiDirnameRector",
    "Rector\\Php70\\Rector\\FuncCall\\RandomFunctionRector",
    "Rector\\Php70\\Rector\\If_\\IfToSpaceshipRector",
    "Rector\\Php70\\Rector\\StmtsAwareInterface\\IfIssetToCoalescingRector",
    "Rector\\Php71\\Rector\\List_\\ListToArrayDestructRector",
    "Rector\\Php71\\Rector\\TryCatch\\MultiExceptionCatchRector",
    "Rector\\Php71\\Rector\\BooleanOr\\IsIterableRector",
    "Rector\\Php73\\Rector\\FuncCall\\ArrayKeyFirstLastRector",
    "Rector\\Php73\\Rector\\FuncCall\\ArrayKeysToArrayKeyFirstLastRector",
    "Rector\\Php73\\Rector\\FuncCall\\SetCookieRector",
    "Rector\\Php73\\Rector\\FuncCall\\JsonThrowOnErrorRector",
    "Rector\\Php73\\Rector\\BooleanOr\\IsCountableRector",
    "Rector\\Php74\\Rector\\Closure\\ClosureToArrowFunctionRector",
    "Rector\\Php74\\Rector\\Property\\RestoreDefaultNullToNullableTypePropertyRector",
    "Rector\\Php74\\Rector\\If_\\IfToNullCoalescingAssignRector",
    "Rector\\Php74\\Rector\\Assign\\NullCoalescingOperatorRector",
    "Rector\\Php80\\Rector\\Ternary\\TernaryToNullsafeCoalesceRector",
    "Rector\\Php80\\Rector\\Ternary\\GetDebugTypeRector",
    "Rector\\Php80\\Rector\\FuncCall\\ClassOnObjectRector",
    "Rector\\Php80\\Rector\\Catch_\\RemoveUnusedVariableInCatchRector",
    "Rector\\Php80\\Rector\\Property\\NestedAnnotationToAttributeRector",
    "Rector\\Php80\\Rector\\ClassConstFetch\\ClassOnThisVariableObjectRector",
    "Rector\\Php80\\Rector\\NotIdentical\\StrContainsRector",
    "Rector\\Php80\\Rector\\NotIdentical\\MbStrContainsRector",
    "Rector\\Php80\\Rector\\Identical\\StrStartsWithRector",
    "Rector\\Php80\\Rector\\Identical\\StrEndsWithRector",
    "Rector\\Php80\\Rector\\Switch_\\ChangeSwitchToMatchRector",
    "Rector\\Php80\\Rector\\Class_\\StringableForToStringRector",
    "Rector\\Php80\\Rector\\Class_\\AnnotationToAttributeRector",
    "Rector\\Php80\\Rector\\Class_\\ClassPropertyAssignToConstructorPromotionRector",
    "Rector\\Php81\\Rector\\Property\\ReadOnlyPropertyRector",
    "Rector\\Php81\\Rector\\New_\\MyCLabsConstructorCallToEnumFromRector",
    "Rector\\Php81\\Rector\\Array_\\ArrayToFirstClassCallableRector",
    "Rector\\Php81\\Rector\\MethodCall\\SpatieEnumMethodCallToEnumConstRector",
    "Rector\\Php81\\Rector\\MethodCall\\MyCLabsMethodCallToEnumConstRector",
    "Rector\\Php81\\Rector\\Class_\\MyCLabsClassToEnumRector",
    "Rector\\Php81\\Rector\\Class_\\SpatieEnumClassToEnumRector",
    "Rector\\Php82\\Rector\\Param\\AddSensitiveParameterAttributeRector",
    "Rector\\Php82\\Rector\\Class_\\ReadOnlyClassRector",
    "Rector\\Php83\\Rector\\FuncCall\\DynamicClassConstFetchRector",
    "Rector\\Php83\\Rector\\ClassMethod\\AddOverrideAttributeToOverriddenMethodsRector",
    "Rector\\Php83\\Rector\\BooleanAnd\\JsonValidateRector",
    "Rector\\Php83\\Rector\\Class_\\ReadOnlyAnonymousClassRector",
    "Rector\\Php83\\Rector\\ClassConst\\AddTypeToConstRector",
    "Rector\\Php84\\Rector\\FuncCall\\RoundingModeEnumRector",
    "Rector\\Php84\\Rector\\Foreach_\\ForeachToArrayFindRector",
    "Rector\\Php84\\Rector\\Foreach_\\ForeachToArrayFindKeyRector",
    "Rector\\Php84\\Rector\\Foreach_\\ForeachToArrayAllRector",
    "Rector\\Php84\\Rector\\Foreach_\\ForeachToArrayAnyRector",
    "Rector\\Php84\\Rector\\MethodCall\\NewMethodCallWithoutParenthesesRector",
    "Rector\\Php84\\Rector\\Class_\\DeprecatedAnnotationToDeprecatedAttributeRector",
    "Rector\\Php84\\Rector\\Class_\\PropertyHookRector",
    "Rector\\Php85\\Rector\\Property\\AddOverrideAttributeToOverriddenPropertiesRector",
    "Rector\\Php85\\Rector\\ArrayDimFetch\\ArrayFirstLastRector",
    "Rector\\Php85\\Rector\\Expression\\NestedFuncCallsToPipeOperatorRector",
    "Rector\\Php85\\Rector\\StmtsAwareInterface\\SequentialAssignmentsToPipeOperatorRector",
    "Rector\\Php85\\Rector\\Const_\\ConstAndTraitDeprecatedAttributeRector",
];

const LIKELY_RULES: &[&str] = &[
    "QuoteBarewordKeys",
    "ParenthesizeNestedTernaryRector",
    "ParseStrWithResultArgumentRector",
    "SensitiveDefineRector",
    "SensitiveConstantNameRector",
    "EregToPregMatchRector",
    "Php4ConstructorRector",
    "BreakNotInLoopOrSwitchToReturnRector",
    "ListSwapArrayOrderRector",
    "ThisCallOnStaticMethodToStaticCallRector",
    "StaticCallOnNonStaticToInstanceCallRector",
    "RemoveExtraParametersRector",
    "BinaryOpBetweenNumberAndStringRector",
    "AssignArrayToStringRector",
    "GetClassOnNullRector",
    "StringsAssertNakedRector",
    "CreateFunctionToAnonymousFunctionRector",
    "WhileEachToForeachRector",
    "ListEachRector",
    "ReplaceEachAssignmentWithKeyCurrentRector",
    "MoneyFormatToNumberFormatRector",
    "AddParamBasedOnParentClassMethodRector",
    "OptionalParametersAfterRequiredRector",
    "NullToStrictStringFuncCallArgRector",
    "NullToStrictIntPregSlitFuncCallLimitArgRector",
    "Utf8DecodeEncodeToMbConvertEncodingRector",
    "OrdSingleByteRector",
    "SleepToSerializeRector",
    "WakeupToUnserializeRector",
];

const RULE_TITLES: &[(&str, Text)] = &[
    ("ExplicitNullableParamTypeRector", Text::new("Örtük nullable parametre açıkça yazılır", "Make implicit nullable params explicit")),
    ("VariableInStringInterpolationFixerRector", Text::same("\"${var}\" → \"{$var}\"")),
    ("Utf8DecodeEncodeToMbConvertEncodingRector", Text::same("utf8_encode/decode → mb_convert_encoding")),
    ("WhileEachToForeachRector", Text::same("while (each()) → foreach")),
    ("ListEachRector", Text::new("list() = each() kalıbı kaldırılır", "Remove list() = each()")),
    ("ReplaceEachAssignmentWithKeyCurrentRector", Text::same("each() → key() / current()")),
    ("CreateFunctionToAnonymousFunctionRector", Text::same("create_function() → closure")),
    ("EregToPregMatchRector", Text::same("ereg*() → preg_*()")),
    ("Php4ConstructorRector", Text::new("PHP 4 kurucusu → __construct", "PHP 4 constructor → __construct")),
    ("RemoveGetClassGetParentClassNoArgsRector", Text::same("get_class() → self::class")),
    ("NullToStrictStringFuncCallArgRector", Text::new("Metin parametrelerine null gitmesin", "Stop passing null to string params")),
    ("CurlyToSquareBracketArrayStringRector", Text::same("$x{0} → $x[0]")),
    ("QuoteBarewordKeys", Text::same("$row[id] → $row['id']")),
    ("AssignNewWithoutReference", Text::same("= &new → = new")),
    ("DefaultColonAfterSwitch", Text::same("default; → default:")),
    ("AssignBuiltinResultWithoutReference", Text::same("= &func_get_args() → = func_get_args()")),
    (
        "ParenthesizeNestedTernaryRector",
        Text::new("İç içe ternary paranteze alınır (eski davranış korunur)", "Parenthesize nested ternaries (keeps the old behaviour)"),
    ),
    (
        "ParseStrWithResultArgumentRector",
        Text::new("parse_str() sonucu $result dizisine yazılır", "parse_str() writes into a $result array"),
    ),
    ("SensitiveDefineRector", Text::new("define()'dan üçüncü parametre kaldırılır", "Drop define()'s third argument")),
    ("SensitiveConstantNameRector", Text::new("Sabit adı büyük harfe çevrilir", "Uppercase constant names")),
    ("UnsetCastRector", Text::new("(unset) dönüşümü kaldırılır", "Remove (unset) casts")),
    ("ShellExecFunctionCallOverBackticksRector", Text::new("Ters tırnak → shell_exec()", "Backticks → shell_exec()")),
    ("ColonAfterSwitchCaseRector", Text::same("case …; → case …:")),
    (
        "OptionalParametersAfterRequiredRector",
        Text::new("Sonraki zorunlu parametrelere varsayılan değer eklenir", "Give later required params a default"),
    ),
    ("AddEscapeArgumentRector", Text::new("CSV fonksiyonlarına escape parametresi eklenir", "Add the escape argument to CSV functions")),
    ("RenameFunctionRector", Text::new("Kaldırılan fonksiyon yeni adıyla değiştirilir", "Rename removed functions")),
    ("StringifyStrNeedlesRector", Text::new("Sayı olan aramalar metne çevrilir", "Stringify numeric needles")),
    ("ParenthesizeNestedTernaryRector", Text::new("İç içe ternary'lere parantez eklenir", "Parenthesize nested ternaries")),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppliedRule {
    pub class: String,
    pub name: String,
    pub title: Localized,
    pub since: Option<PhpVersion>,
    pub fix: FixKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub file: String,
    pub rules: Vec<AppliedRule>,
    pub fix: FixKind,
    pub diff: FileDiff,
    #[serde(skip)]
    pub original_hash: u64,
    #[serde(skip)]
    pub new_content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RectorPreview {
    pub target: PhpVersion,
    pub mode: RectorMode,
    pub changes: Vec<FileChange>,
    pub errors: Vec<String>,
    pub duration_ms: u64,
}

#[derive(Debug)]
pub enum RectorError {
    Tool(ToolError),
    Output(String),
    Io(String, std::io::Error),
    Stale(String),
    Cancelled,
    TimedOut(u64),
}

impl From<ToolError> for RectorError {
    fn from(e: ToolError) -> Self {
        RectorError::Tool(e)
    }
}

impl std::fmt::Display for RectorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            RectorError::Tool(e) => e.to_string(),
            RectorError::Output(out) => crate::t!("Rector çıktısı okunamadı: {out}", "Couldn't read Rector's output: {out}"),
            RectorError::Io(path, e) => {
                let reason = crate::i18n::io_error(e);
                crate::t!("Dosya işlemi başarısız ({path}): {reason}", "File operation failed ({path}): {reason}")
            }
            RectorError::Cancelled => crate::t!("Rector durduruldu.", "Rector was stopped."),
            RectorError::TimedOut(minutes) => crate::t!(
                "Rector {minutes} dakikada bitmediği için durduruldu. Projenin bir bölümünü dışarıda bırakmayı ya da PHP'nin bellek ayarlarını kontrol etmeyi deneyin.",
                "Rector was stopped because it didn't finish in {minutes} minutes. Try leaving part of the project out, or check PHP's memory settings."
            ),
            RectorError::Stale(file) => crate::t!(
                "{file} önizlemeden sonra değişmiş. Değişiklikleri yeniden hesaplayın.",
                "{file} changed after the preview. Recalculate the changes."
            ),
        };
        f.write_str(&msg)
    }
}

impl std::error::Error for RectorError {}

pub fn rule_short_name(class: &str) -> &str {
    class.rsplit('\\').next().unwrap_or(class)
}

fn rule_since(class: &str) -> Option<PhpVersion> {
    if let Some((_, (major, minor))) = UNVERSIONED_RULES.iter().find(|(c, _)| *c == class) {
        return Some(PhpVersion::new(*major, *minor));
    }
    let ns = class.split('\\').nth(1)?;
    let digits = ns.strip_prefix("Php")?;
    if digits.len() != 2 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let n: u32 = digits.parse().ok()?;
    Some(PhpVersion::new(n / 10, n % 10))
}

fn humanize(short: &str) -> String {
    let base = short.strip_suffix("Rector").unwrap_or(short);
    let mut out = String::new();
    for (i, c) in base.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            out.push(' ');
        }
        out.push(c);
    }
    out
}

pub fn describe_rule(class: &str) -> AppliedRule {
    let name = rule_short_name(class);
    let title = RULE_TITLES.iter().find(|(n, _)| *n == name).map_or_else(|| Localized::same(humanize(name)), |(_, t)| (*t).into());
    let fix = if LIKELY_RULES.contains(&name) { FixKind::Likely } else { FixKind::Auto };
    AppliedRule { class: class.to_string(), name: name.to_string(), title, since: rule_since(class), fix }
}

fn glob_literal(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '[' => "[[]".to_string(),
            '*' => "[*]".to_string(),
            '?' => "[?]".to_string(),
            c => c.to_string(),
        })
        .collect()
}

fn php_str(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'"))
}

#[derive(Default)]
pub struct Staging {
    dir: PathBuf,
    files: Vec<StagedFile>,
    scope: Option<std::collections::HashSet<String>>,
}

struct StagedFile {
    rel: String,
    fixed: String,
    original: String,
    rules: Vec<AppliedRule>,
}

impl Staging {
    fn prepare(root: &Path, excluded_dirs: &[String], dir: PathBuf) -> Result<Self, RectorError> {
        let _ = std::fs::remove_dir_all(&dir);
        let mut files = Vec::new();
        let mut scope = std::collections::HashSet::new();
        let mut constants = std::collections::HashSet::new();
        let mut candidates = Vec::new();
        for path in crate::project::collect_php_files(root, excluded_dirs) {
            scope.insert(crate::analyzer::relative_path(root, &path));
            let Ok(bytes) = std::fs::read(&path) else { continue };
            let Ok(text) = String::from_utf8(bytes) else { continue };
            constants.extend(crate::analyzer::constant_names(&text));
            if crate::analyzer::fix_legacy_syntax(&text).is_some() {
                candidates.push((path, text));
            }
        }
        for (path, text) in candidates {
            let Some((fixed, applied)) = crate::analyzer::fix_legacy_syntax_with(&text, &constants) else {
                continue;
            };
            let rel = crate::analyzer::relative_path(root, &path);
            let dest = dir.join(&rel);
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| RectorError::Io(parent.display().to_string(), e))?;
            }
            std::fs::write(&dest, &fixed).map_err(|e| RectorError::Io(dest.display().to_string(), e))?;
            files.push(StagedFile { rel, fixed, original: text, rules: applied.into_iter().map(describe_rule).collect() });
        }
        Ok(Staging { dir, files, scope: Some(scope) })
    }

    fn get(&self, rel: &str) -> Option<&StagedFile> {
        self.files.iter().find(|f| f.rel == rel)
    }

    fn in_scope(&self, rel: &str) -> bool {
        self.scope.as_ref().is_none_or(|s| s.contains(rel))
    }
}

fn not_utf8(rel: &str) -> String {
    crate::t!(
        "{rel} — UTF-8 değil; dosya bozulmasın diye atlandı. Önce UTF-8'e dönüştürün.",
        "{rel} — not UTF-8; skipped so the file isn't corrupted. Convert it to UTF-8 first."
    )
}

pub fn build_config(
    root: &Path,
    target: PhpVersion,
    mode: RectorMode,
    excluded_dirs: &[String],
    staging: &Staging,
    cache_dir: &Path,
) -> String {
    let mut paths = vec![php_str(&root.to_string_lossy())];
    let mut bases = vec![root.to_string_lossy().into_owned()];
    if let Ok(real) = std::fs::canonicalize(root) {
        let real = real.to_string_lossy().into_owned();
        bases.push(real.strip_prefix(r"\\?\").map(str::to_string).unwrap_or(real));
    }
    let mut bases: Vec<String> = bases.iter().map(|b| glob_literal(b.replace('\\', "/").trim_end_matches('/'))).collect();
    bases.dedup();
    let mut skip: Vec<String> = bases
        .iter()
        .flat_map(|base| {
            ["vendor", "node_modules"]
                .iter()
                .flat_map(move |d| [format!("{base}/{d}/*"), format!("{base}/*/{d}/*")])
                .chain(excluded_dirs.iter().map(move |d| format!("{base}/{}/*", d.replace('\\', "/").trim_matches('/'))))
        })
        .map(|p| php_str(&p))
        .collect();
    if !staging.files.is_empty() {
        paths.push(php_str(&staging.dir.to_string_lossy()));
        skip.extend(staging.files.iter().flat_map(|f| bases.iter().map(move |base| php_str(&format!("{base}/{}", glob_literal(&f.rel))))));
    }
    if mode == RectorMode::Compat {
        skip.extend(MODERNIZATION_RULES.iter().filter(|c| rule_since(c).is_some_and(|v| v <= target)).map(|c| php_str(c)));
    }
    format!(
        "<?php\n\n// Generated by Ferry.\n\ndeclare(strict_types=1);\n\nuse Rector\\Config\\RectorConfig;\n\nreturn RectorConfig::configure()\n    ->withPaths([{paths}])\n    ->withFileExtensions([{extensions}])\n    ->withSkip([\n        {skip}\n    ])\n    ->withPhpSets({set}: true)\n    ->withPhpVersion({version_id})\n    ->withCache({cache});\n",
        paths = paths.join(", "),
        extensions = crate::project::PHP_EXTENSIONS.iter().map(|e| php_str(e)).collect::<Vec<_>>().join(", "),
        skip = skip.join(",\n        "),
        set = target.compact(),
        version_id = target.major * 10000 + target.minor * 100,
        cache = php_str(&cache_dir.to_string_lossy()),
    )
}

#[derive(Deserialize)]
struct RectorJson {
    #[serde(default)]
    file_diffs: Vec<RectorFileDiff>,
    #[serde(default)]
    errors: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct RectorFileDiff {
    file: String,
    diff: String,
    #[serde(default)]
    applied_rectors: Vec<String>,
}

fn error_message(v: &serde_json::Value) -> String {
    let msg =
        v.get("message").and_then(|m| m.as_str()).map(str::to_string).unwrap_or_else(|| crate::t!("bilinmeyen hata", "unknown error"));
    match (v.get("file").and_then(|f| f.as_str()), v.get("line").and_then(|l| l.as_u64())) {
        (Some(f), Some(l)) => format!("{f}:{l} — {msg}"),
        (Some(f), None) => format!("{f} — {msg}"),
        _ => msg.to_string(),
    }
}

fn find_json(stdout: &str) -> Result<RectorJson, RectorError> {
    let mut first_error = None;
    for (i, _) in stdout.match_indices('{') {
        let mut stream = serde_json::Deserializer::from_str(&stdout[i..]).into_iter::<serde_json::Value>();
        match stream.next() {
            Some(Ok(value)) if value.get("file_diffs").is_some() || value.get("totals").is_some() => {
                return serde_json::from_value(value).map_err(|e| RectorError::Output(e.to_string()));
            }
            Some(Err(e)) if first_error.is_none() => first_error = Some(e.to_string()),
            _ => {}
        }
    }
    let excerpt: String = stdout.trim().chars().take(1200).collect();
    Err(RectorError::Output(match first_error {
        Some(e) => format!("{e}: {excerpt}"),
        None => excerpt,
    }))
}

pub fn parse_output(root: &Path, stdout: &str, staging: &Staging) -> Result<(Vec<FileChange>, Vec<String>), RectorError> {
    let parsed = find_json(stdout)?;

    let mut errors: Vec<String> = parsed.errors.iter().map(error_message).collect();
    let mut changes = Vec::new();
    for fd in parsed.file_diffs {
        let abs = if Path::new(&fd.file).is_absolute() { PathBuf::from(&fd.file) } else { root.join(&fd.file) };
        let staged_rel = crate::analyzer::relative_path(&staging.dir, &abs);
        let staged = staging.get(&staged_rel);
        let rel = match staged {
            Some(_) => staged_rel.clone(),
            None => crate::analyzer::relative_path(root, &abs),
        };
        if staged.is_none() && (staging.get(&rel).is_some() || !staging.in_scope(&rel)) {
            continue;
        }
        if Path::new(&rel).is_absolute() || Path::new(&rel).components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            errors.push(crate::t!("{rel} — proje klasörünün dışında; atlandı.", "{rel} — outside the project folder; skipped."));
            continue;
        }
        let original_text = match staged {
            Some(f) => f.original.clone(),
            None => match std::fs::read(&abs) {
                Ok(bytes) => match String::from_utf8(bytes) {
                    Ok(text) => text,
                    Err(_) => {
                        errors.push(not_utf8(&rel));
                        continue;
                    }
                },
                Err(e) => {
                    let e = crate::i18n::io_error(&e);
                    errors.push(crate::t!("{rel} — okunamadı: {e}", "{rel} — couldn't read: {e}"));
                    continue;
                }
            },
        };
        let new_content = match diff::apply_unified(staged.map_or(original_text.as_str(), |f| f.fixed.as_str()), &fd.diff) {
            Ok(c) => c,
            Err(e) => {
                errors.push(format!("{rel} — {e}"));
                continue;
            }
        };
        let mut rules: Vec<AppliedRule> = staged.map(|f| f.rules.clone()).unwrap_or_default();
        rules.extend(fd.applied_rectors.iter().map(|c| describe_rule(c)));
        rules.dedup_by(|a, b| a.class == b.class);
        let fix = rules.iter().map(|r| r.fix).max().unwrap_or(FixKind::Auto);
        changes.push(FileChange {
            diff: diff::compute(&original_text, &new_content, 3),
            file: rel,
            rules,
            fix,
            original_hash: fnv1a(original_text.as_bytes()),
            new_content,
        });
    }
    for f in &staging.files {
        if changes.iter().any(|c| c.file == f.rel) {
            continue;
        }
        changes.push(FileChange {
            diff: diff::compute(&f.original, &f.fixed, 3),
            file: f.rel.clone(),
            rules: f.rules.clone(),
            fix: f.rules.iter().map(|r| r.fix).max().unwrap_or(FixKind::Auto),
            original_hash: fnv1a(f.original.as_bytes()),
            new_content: f.fixed.clone(),
        });
    }
    changes.sort_by(|a, b| (b.fix, &a.file).cmp(&(a.fix, &b.file)));
    Ok((changes, errors))
}

pub(crate) fn run_cancellable(mut cmd: Command, program: &Path, cancel: &AtomicBool, timeout: Duration) -> Result<Output, RectorError> {
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|source| ToolError::Spawn { program: program.display().to_string(), source })?;
    let read = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        })
    };
    let stdout = read(child.stdout.take().map(|p| Box::new(p) as _));
    let stderr = read(child.stderr.take().map(|p| Box::new(p) as _));
    let started = Instant::now();
    let io = |e| RectorError::Io("rector".into(), e);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(io)? {
            break status;
        }
        let stop = if cancel.load(Ordering::Relaxed) {
            Some(RectorError::Cancelled)
        } else if started.elapsed() >= timeout {
            Some(RectorError::TimedOut(timeout.as_secs().div_ceil(60).max(1)))
        } else {
            None
        };
        if let Some(err) = stop {
            let _ = child.kill();
            let _ = child.wait();
            return Err(err);
        }
        std::thread::sleep(Duration::from_millis(100));
    };
    Ok(Output { status, stdout: stdout.join().unwrap_or_default(), stderr: stderr.join().unwrap_or_default() })
}

#[allow(clippy::too_many_arguments)]
pub fn preview(
    php: &Path,
    rector: &Path,
    root: &Path,
    target: PhpVersion,
    mode: RectorMode,
    excluded_dirs: &[String],
    work_dir: &Path,
    cancel: &AtomicBool,
) -> Result<RectorPreview, RectorError> {
    if !rector.is_file() {
        return Err(ToolError::RectorMissing.into());
    }
    let started = Instant::now();
    let key = format!("{:016x}", fnv1a(root.to_string_lossy().as_bytes()));
    let dir = work_dir.join(&key);
    std::fs::create_dir_all(&dir).map_err(|e| RectorError::Io(dir.display().to_string(), e))?;
    let run = tempfile::Builder::new().prefix("run-").tempdir_in(&dir).map_err(|e| RectorError::Io(dir.display().to_string(), e))?;
    let config_path = run.path().join("rector.php");
    let staging = Staging::prepare(root, excluded_dirs, run.path().join("staged"))?;
    let config = build_config(root, target, mode, excluded_dirs, &staging, &dir.join("cache"));
    std::fs::write(&config_path, config).map_err(|e| RectorError::Io(config_path.display().to_string(), e))?;

    let mut cmd = command(php);
    cmd.arg("-d")
        .arg("memory_limit=-1")
        .arg(rector)
        .args(["process", "--dry-run", "--output-format=json", "--no-progress-bar", "--no-ansi", "--config"])
        .arg(&config_path)
        .current_dir(root);
    let output = run_cancellable(cmd, php, cancel, RECTOR_TIMEOUT)?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let (changes, errors) = parse_output(root, &stdout, &staging).map_err(|e| {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.trim().is_empty() {
            e
        } else {
            RectorError::Output(stderr.trim().chars().take(1200).collect())
        }
    })?;
    Ok(RectorPreview { target, mode, changes, errors, duration_ms: started.elapsed().as_millis() as u64 })
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let meta = std::fs::symlink_metadata(path).ok();
    if meta.as_ref().is_some_and(|m| m.file_type().is_symlink()) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            crate::t!("sembolik bağlantı; hedef dosya elle düzenlenmeli", "it's a symbolic link; edit the target file by hand"),
        ));
    }
    #[cfg(unix)]
    if meta.as_ref().is_some_and(|m| std::os::unix::fs::MetadataExt::nlink(m) > 1) {
        let mut file = std::fs::OpenOptions::new().write(true).truncate(true).open(path)?;
        file.write_all(bytes)?;
        return file.sync_all();
    }
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    tmp.write_all(bytes)?;
    tmp.as_file().sync_all()?;
    let readonly = meta.as_ref().is_some_and(|m| m.permissions().readonly());
    if let Some(meta) = &meta {
        let mut perms = meta.permissions();
        #[allow(clippy::permissions_set_readonly_false)]
        perms.set_readonly(false);
        let _ = tmp.as_file().set_permissions(perms);
    }
    if readonly && cfg!(windows) {
        set_readonly(path, false)?;
    }
    if let Err(e) = tmp.persist(path) {
        if readonly && cfg!(windows) {
            let _ = set_readonly(path, true);
        }
        return Err(e.error);
    }
    if readonly {
        set_readonly(path, true)?;
    }
    Ok(())
}

fn set_readonly(path: &Path, readonly: bool) -> std::io::Result<()> {
    let mut perms = std::fs::metadata(path)?.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
}

pub fn check_fresh(root: &Path, changes: &[&FileChange]) -> Result<(), RectorError> {
    for change in changes {
        let path = root.join(&change.file);
        let current = std::fs::read(&path).map_err(|e| RectorError::Io(change.file.clone(), e))?;
        if fnv1a(&current) != change.original_hash {
            return Err(RectorError::Stale(change.file.clone()));
        }
    }
    Ok(())
}

pub fn write_changes(root: &Path, changes: &[&FileChange], written: &mut Vec<String>) -> Result<(), RectorError> {
    for change in changes {
        let path = root.join(&change.file);
        write_atomic(&path, change.new_content.as_bytes()).map_err(|e| RectorError::Io(change.file.clone(), e))?;
        written.push(change.file.clone());
    }
    Ok(())
}

pub fn apply(root: &Path, changes: &[&FileChange]) -> Result<Vec<String>, RectorError> {
    check_fresh(root, changes)?;
    let mut written = Vec::new();
    write_changes(root, changes, &mut written)?;
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_made_while_rector_runs_make_the_change_stale() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.php"), "<?php\necho $s{0};\n// edited meanwhile\n").unwrap();
        let staging = Staging {
            dir: root.join("staged"),
            files: vec![StagedFile {
                rel: "a.php".into(),
                fixed: "<?php\necho $s[0];\n".into(),
                original: "<?php\necho $s{0};\n".into(),
                rules: Vec::new(),
            }],
            scope: None,
        };
        let (changes, errors) = parse_output(root, r#"{"totals":{"changed_files":0},"file_diffs":[]}"#, &staging).unwrap();
        assert!(errors.is_empty());
        assert!(matches!(check_fresh(root, &[&changes[0]]), Err(RectorError::Stale(_))));
    }

    #[test]
    fn describes_rules() {
        let r = describe_rule("Rector\\Php84\\Rector\\Param\\ExplicitNullableParamTypeRector");
        assert_eq!(r.since, Some(PhpVersion::new(8, 4)));
        assert_eq!(r.fix, FixKind::Auto);
        assert_eq!(r.title.tr, "Örtük nullable parametre açıkça yazılır");
        assert_eq!(r.title.en, "Make implicit nullable params explicit");
        let r = describe_rule("Rector\\Php72\\Rector\\While_\\WhileEachToForeachRector");
        assert_eq!(r.fix, FixKind::Likely);
        let r = describe_rule("Rector\\CodeQuality\\Rector\\Foo\\SomethingNiceRector");
        assert_eq!(r.since, None);
        assert_eq!(r.title, Localized::same("Something Nice"));
    }

    #[test]
    fn config_skips_only_registered_modernization_rules() {
        let root = Path::new("/p/it's");
        let cfg = build_config(root, PhpVersion::new(8, 1), RectorMode::Compat, &["storage".into()], &Staging::default(), Path::new("/c"));
        assert!(cfg.contains("->withPhpSets(php81: true)"));
        assert!(cfg.contains("'/p/it\\'s'"));
        assert!(cfg.contains("StrContainsRector"));
        assert!(cfg.contains("FunctionFirstClassCallableRector"), "modernizations outside PhpXY namespaces are skipped too");
        assert!(cfg.contains("'/p/it\\'s/storage/*'"));
        assert!(cfg.contains("'/p/it\\'s/*/vendor/*'"));
        assert!(!cfg.contains("'*/vendor/*'"));
        assert!(cfg.contains("withFileExtensions(['php', 'phtml', 'inc'"));
        assert!(!cfg.contains("Php84"), "rules newer than the target must not be skipped");
        let modern = build_config(root, PhpVersion::new(8, 4), RectorMode::Modernize, &[], &Staging::default(), Path::new("/c"));
        assert!(!modern.contains("StrContainsRector"));
    }

    #[test]
    fn parses_output_and_applies_safely() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("src/a.php"), "<?php\nfunction f(Foo $a = null) {}\n").unwrap();
        let stdout = r#"[warning] noise
{
    "totals": {"changed_files": 1, "errors": 1},
    "file_diffs": [{
        "file": "src/a.php",
        "diff": "--- Original\n+++ New\n@@ -1,2 +1,2 @@\n <?php\n-function f(Foo $a = null) {}\n+function f(?Foo $a = null) {}\n",
        "applied_rectors": ["Rector\\Php84\\Rector\\Param\\ExplicitNullableParamTypeRector"]
    }],
    "errors": [{"message": "Syntax error", "file": "src/b.php", "line": 3}]
}"#;
        let (changes, errors) = parse_output(root, stdout, &Staging::default()).unwrap();
        assert_eq!(errors, vec!["src/b.php:3 — Syntax error"]);
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].diff.additions, 1);

        let refs: Vec<&FileChange> = changes.iter().collect();
        assert_eq!(apply(root, &refs).unwrap(), vec!["src/a.php"]);
        assert_eq!(std::fs::read_to_string(root.join("src/a.php")).unwrap(), "<?php\nfunction f(?Foo $a = null) {}\n");
        assert!(matches!(apply(root, &refs), Err(RectorError::Stale(_))));
    }

    #[test]
    fn tolerates_noise_around_the_json_and_skips_non_utf8_files() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.php"), b"<?php\n// \xfc\nf();\n").unwrap();
        let stdout = "Array\n(\n{\n    [x] => 1\n)\n{\"totals\": {}, \"file_diffs\": [{\"file\": \"a.php\", \"diff\": \"@@ -3,1 +3,1 @@\\n-f();\\n+g();\\n\", \"applied_rectors\": []}]}\nPHP Deprecated: something {closure}\n";
        let (changes, errors) = parse_output(root, stdout, &Staging::default()).unwrap();
        assert!(changes.is_empty());
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("UTF-8"), "{errors:?}");
        assert!(parse_output(root, "Fatal error in {closure}", &Staging::default()).is_err());
    }

    fn nullable_diff(file: &str) -> String {
        format!(
            r#"{{"file": "{file}", "diff": "@@ -1,2 +1,2 @@\n <?php\n-function f(Foo $a = null) {{}}\n+function f(?Foo $a = null) {{}}\n", "applied_rectors": []}}"#
        )
    }

    #[test]
    fn ignores_files_the_scanner_skips() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let src = "<?php\nfunction f(Foo $a = null) {}\n";
        for f in ["src/a.php", ".ddev/b.php", ".git/hooks/c.php"] {
            std::fs::create_dir_all(root.join(f).parent().unwrap()).unwrap();
            std::fs::write(root.join(f), src).unwrap();
        }
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("src/a.php"), root.join("src/link.php")).unwrap();
        let staging = Staging::prepare(root, &[], root.join(".ferry-staged")).unwrap();
        let diffs: Vec<String> =
            ["src/a.php", ".ddev/b.php", ".git/hooks/c.php", "src/link.php"].iter().map(|f| nullable_diff(f)).collect();
        let stdout = format!(r#"{{"file_diffs": [{}]}}"#, diffs.join(","));
        let (changes, errors) = parse_output(root, &stdout, &staging).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        let files: Vec<&str> = changes.iter().map(|c| c.file.as_str()).collect();
        assert_eq!(files, ["src/a.php"]);
    }

    #[test]
    fn constants_defined_in_other_files_stay_constants() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("config.php"), "<?php define('active', 'actual-key'); const other_key = 1;").unwrap();
        std::fs::write(root.join("a.php"), "<?php\nrequire 'config.php';\necho $a[active] . $a[other_key] . $a[missing];\n").unwrap();
        let staging = Staging::prepare(root, &[], root.join(".ferry-staged")).unwrap();
        let (changes, errors) = parse_output(root, r#"{"file_diffs": []}"#, &staging).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_content, "<?php\nrequire 'config.php';\necho $a[active] . $a[other_key] . $a['missing'];\n");
        assert_eq!(changes[0].fix, FixKind::Likely, "a staged-only file keeps its riskiest rule");
    }

    #[test]
    fn a_staged_file_reported_twice_yields_one_change() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::write(root.join("a.php"), "<?php\necho $s{0};\nfunction f(Foo $a = null) {}\n").unwrap();
        let staging = Staging::prepare(root, &[], root.join("[staged]")).unwrap();
        assert_eq!(staging.files.len(), 1);
        let staged = root.join("[staged]/a.php").to_string_lossy().replace('\\', "/");
        let stdout = format!(
            r#"{{"file_diffs": [
                {{"file": "a.php", "diff": "@@ -1,1 +1,1 @@\n-<?php\n+<?php // original\n", "applied_rectors": []}},
                {{"file": "{staged}", "diff": "@@ -3,1 +3,1 @@\n-function f(Foo $a = null) {{}}\n+function f(?Foo $a = null) {{}}\n", "applied_rectors": []}}
            ]}}"#
        );
        let (changes, errors) = parse_output(root, &stdout, &staging).unwrap();
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].new_content, "<?php\necho $s[0];\nfunction f(?Foo $a = null) {}\n");
    }

    #[test]
    fn staged_originals_are_skipped_with_literal_globs() {
        let root = Path::new(r"C:\Users\me\[old] site");
        let staging = Staging {
            dir: PathBuf::from(r"C:\tmp\staged"),
            files: vec![StagedFile { rel: "src/a[1].php".into(), fixed: String::new(), original: String::new(), rules: Vec::new() }],
            scope: None,
        };
        let cfg = build_config(root, PhpVersion::new(8, 4), RectorMode::Compat, &[], &staging, Path::new("/c"));
        assert!(cfg.contains("'C:/Users/me/[[]old] site/src/a[[]1].php'"), "{cfg}");
    }

    #[test]
    fn writes_keep_read_only_files_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.php");
        std::fs::write(&path, "old").unwrap();
        set_readonly(&path, true).unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        assert!(std::fs::metadata(&path).unwrap().permissions().readonly());
        set_readonly(&path, false).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn writes_refuse_symlinks_and_keep_hard_links() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.php");
        std::fs::write(&target, "old").unwrap();
        let link = dir.path().join("link.php");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        assert!(write_atomic(&link, b"new").is_err());
        assert!(std::fs::symlink_metadata(&link).unwrap().file_type().is_symlink());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "old");

        let hard = dir.path().join("hard.php");
        std::fs::hard_link(&target, &hard).unwrap();
        write_atomic(&hard, b"new").unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "new");
    }

    #[cfg(unix)]
    #[test]
    fn long_runs_can_be_cancelled_or_time_out() {
        let sleep = Path::new("/bin/sleep");
        let mut cmd = Command::new(sleep);
        cmd.arg("30");
        let started = Instant::now();
        let err = run_cancellable(cmd, sleep, &AtomicBool::new(false), Duration::from_millis(300)).unwrap_err();
        assert!(matches!(err, RectorError::TimedOut(1)), "{err}");
        assert!(started.elapsed() < Duration::from_secs(5));

        let cancel = std::sync::Arc::new(AtomicBool::new(false));
        let flag = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(200));
            flag.store(true, Ordering::Relaxed);
        });
        let mut cmd = Command::new(sleep);
        cmd.arg("30");
        assert!(matches!(run_cancellable(cmd, sleep, &cancel, RECTOR_TIMEOUT), Err(RectorError::Cancelled)));

        let mut cmd = Command::new("/bin/sh");
        cmd.args(["-c", "echo out; echo err >&2"]);
        let out = run_cancellable(cmd, sleep, &AtomicBool::new(false), RECTOR_TIMEOUT).unwrap();
        assert_eq!((out.stdout.as_slice(), out.stderr.as_slice()), (&b"out\n"[..], &b"err\n"[..]));
    }
}
