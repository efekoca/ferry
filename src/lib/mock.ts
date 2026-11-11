import type { Api } from "./api";
import type { Localized } from "../i18n";
import type { CheckSummary, DockerComparison, DockerStep, DockerSummary, HistoryEntry, ProjectConfig, ScanCounts,
  CheckRun,
  DependencyReport,
  DockerGap,
  SnapshotConflict,
  DiffLine,
  FileChange,
  Finding,
  FixKind,
  ProjectInfo,
  ScanProgress,
  IgnoreEntry,
  WpLatest,
  ScanResult,
  Settings,
  Severity,
  Snapshot,
  Summary,
  ToolchainStatus,
} from "./types";

export const MOCK_ROOT = "/Users/demo/Projeler/kuzey-magaza";

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const L = (tr: string, en: string): Localized => ({ tr, en });
const same = (both: string): Localized => ({ tr: both, en: both });

const versionLte = (a: string, b: string) => {
  const [am, an] = a.split(".").map(Number);
  const [bm, bn] = b.split(".").map(Number);
  return am < bm || (am === bm && an <= bn);
};

const MYSQL = L(
  "ext/mysql PHP 7.0'da kaldırıldı. Veritabanı katmanının mysqli ya da PDO ile yeniden yazılması gerekiyor.",
  "ext/mysql was removed in PHP 7.0. The database layer needs to be rewritten with mysqli or PDO.",
);
const MYSQL_REPL = L("mysqli_* ya da PDO", "mysqli_* or PDO");
const DOLLAR_BRACE = L("Metin içinde ${...} yazımı PHP 8.2'de kullanımdan kalktı.", "${...} interpolation in strings is deprecated in PHP 8.2.");
const DOLLAR_BRACE_TITLE = L("\"${var}\" yazımı", "\"${var}\" interpolation");
const NULLABLE = L(
  "Varsayılan değeri null olan tipli parametreler açıkça nullable (?Tip) yazılmalı.",
  "Typed parameters that default to null must be declared nullable (?Type).",
);
const NULLABLE_TITLE = L("Örtük nullable parametre", "Implicitly nullable parameter");
const NULLABLE_REPL = L("?Tip $param = null", "?Type $param = null");

type Seed = Omit<Finding, "id" | "column" | "runtimeHits">;

const SEEDS: Seed[] = [
  { file: "wp-includes/version.php", line: 16, ruleId: "framework.wordpress_version", title: same("WordPress 6.2"), message: L("WordPress 6.2, PHP 8.4 ile çalışmaz. PHP 8.4 için en az WordPress 6.7 gerekiyor; önce WordPress'i güncelleyin.", "WordPress 6.2 doesn't run on PHP 8.4. PHP 8.4 needs at least WordPress 6.7; update WordPress first."), since: "8.3", severity: "breaking", fix: "manual", replacement: same("WordPress 6.7+"), snippet: "$wp_version = '6.2.2';" },
  { file: "includes/class-db.php", line: 42, ruleId: "removed.mysql", title: same("mysql_connect()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "$this->link = mysql_connect($host, $user, $pass);" },
  { file: "includes/class-db.php", line: 58, ruleId: "removed.mysql", title: same("mysql_query()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "$res = mysql_query($sql, $this->link);" },
  { file: "includes/class-db.php", line: 61, ruleId: "removed.mysql", title: same("mysql_fetch_assoc()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "while ($row = mysql_fetch_assoc($res)) {" },
  { file: "includes/class-cart.php", line: 118, ruleId: "removed.each", title: same("each()"), message: L("each() kaldırıldı. while (list() = each()) döngüleri foreach'e çevrilir.", "each() was removed. while (list() = each()) loops become foreach."), since: "8.0", severity: "breaking", fix: "likely", replacement: L("foreach ya da key()/current()/next()", "foreach or key()/current()/next()"), snippet: "while (list($key, $item) = each($this->items)) {" },
  { file: "includes/class-cart.php", line: 204, ruleId: "removed.create_function", title: same("create_function()"), message: L("create_function() kaldırıldı. Kod dizesi bir closure'a çevrilmeli; dize çalışma anında üretiliyorsa elle inceleyin.", "create_function() was removed. The code string has to become a closure; review it by hand if the string is built at runtime."), since: "8.0", severity: "breaking", fix: "likely", replacement: L("anonim fonksiyon (closure)", "an anonymous function (closure)"), snippet: "usort($items, create_function('$a,$b', 'return $a[\"price\"] - $b[\"price\"];'));" },
  { file: "includes/helpers.php", line: 15, ruleId: "removed.curly_offset", title: L("Süslü parantezle dizi erişimi", "Curly brace offset access"), message: L("$x{0} yazımı PHP 8.0'da kaldırıldı ve ayrıştırma hatası verir.", "The $x{0} syntax was removed in PHP 8.0 and is now a parse error."), since: "8.0", severity: "breaking", fix: "auto", replacement: same("$x[0]"), snippet: "$first = $sku{0};" },
  { file: "includes/class-crypto.php", line: 33, ruleId: "removed.mcrypt", title: same("mcrypt_encrypt()"), message: L("ext/mcrypt PHP 7.2'de çekirdekten çıkarıldı. Şifreleme openssl ya da sodium ile yeniden yazılmalı; eski şifreli verinin hâlâ çözülebildiğini doğrulayın.", "ext/mcrypt was dropped from core in PHP 7.2. Rewrite the encryption with openssl or sodium and make sure existing encrypted data can still be decrypted."), since: "7.2", severity: "breaking", fix: "manual", replacement: L("openssl_* ya da sodium_*", "openssl_* or sodium_*"), snippet: "return mcrypt_encrypt(MCRYPT_RIJNDAEL_128, $this->key, $plain, MCRYPT_MODE_CBC, $iv);" },
  { file: "includes/helpers.php", line: 71, ruleId: "removed.ereg", title: same("ereg()"), message: L("POSIX regex fonksiyonları kaldırıldı. Desen, sınırlayıcılarla PCRE sözdizimine çevrilmelidir.", "POSIX regex functions were removed. The pattern has to be converted to PCRE syntax with delimiters."), since: "7.0", severity: "breaking", fix: "likely", replacement: same("preg_match"), snippet: "if (ereg('^[0-9]{4}$', $code)) {" },
  { file: "admin/settings-page.php", line: 27, ruleId: "deprecated.filter_sanitize_string", title: same("FILTER_SANITIZE_STRING"), message: L("FILTER_SANITIZE_STRING kullanımdan kalktı. İhtiyacınıza göre htmlspecialchars() ya da strip_tags() kullanın.", "FILTER_SANITIZE_STRING is deprecated. Use htmlspecialchars() or strip_tags(), depending on what you need."), since: "8.1", severity: "deprecated", fix: "manual", replacement: same("htmlspecialchars()"), snippet: "$title = filter_input(INPUT_POST, 'title', FILTER_SANITIZE_STRING);" },
  { file: "admin/settings-page.php", line: 88, ruleId: "deprecated.strftime", title: same("strftime()"), message: L("strftime() kullanımdan kalktı ve PHP 9'da kaldırılacak. Biçim karakterleri date() ile birebir eşleşmez.", "strftime() is deprecated and will be removed in PHP 9. Its format characters don't map one-to-one to date()."), since: "8.1", severity: "deprecated", fix: "manual", replacement: L("IntlDateFormatter ya da date()", "IntlDateFormatter or date()"), snippet: "echo strftime('%d %B %Y', $order->created);" },
  { file: "includes/class-mailer.php", line: 54, ruleId: "deprecated.utf8_encode", title: same("utf8_encode()"), message: L("utf8_encode() kullanımdan kalktı. Kaynak metnin gerçekten ISO-8859-1 olduğundan emin olun.", "utf8_encode() is deprecated. Make sure the source text really is ISO-8859-1."), since: "8.2", severity: "deprecated", fix: "likely", replacement: same("mb_convert_encoding($s, 'UTF-8', 'ISO-8859-1')"), snippet: "$subject = utf8_encode($subject);" },
  { file: "includes/class-mailer.php", line: 77, ruleId: "deprecated.dollar_brace_interpolation", title: DOLLAR_BRACE_TITLE, message: DOLLAR_BRACE, since: "8.2", severity: "deprecated", fix: "auto", replacement: same("\"{$var}\""), snippet: "$body = \"Merhaba ${name}, siparişiniz hazır.\";" },
  { file: "includes/class-mailer.php", line: 79, ruleId: "deprecated.dollar_brace_interpolation", title: DOLLAR_BRACE_TITLE, message: DOLLAR_BRACE, since: "8.2", severity: "deprecated", fix: "auto", replacement: same("\"{$var}\""), snippet: "$body .= \"Takip no: ${tracking}\";" },
  { file: "includes/class-order.php", line: 12, ruleId: "deprecated.implicit_nullable", title: NULLABLE_TITLE, message: NULLABLE, since: "8.4", severity: "deprecated", fix: "auto", replacement: NULLABLE_REPL, snippet: "public function __construct(Customer $customer = null, array $items = null)" },
  { file: "includes/class-order.php", line: 46, ruleId: "deprecated.implicit_nullable", title: NULLABLE_TITLE, message: NULLABLE, since: "8.4", severity: "deprecated", fix: "auto", replacement: NULLABLE_REPL, snippet: "public function applyCoupon(Coupon $coupon = null): float" },
  { file: "includes/class-order.php", line: 90, ruleId: "deprecated.get_class_no_args", title: L("Parametresiz get_class()", "get_class() without arguments"), message: L("Parametresiz get_class() ve get_parent_class() çağrıları 8.3'te kullanımdan kalktı.", "Calling get_class() or get_parent_class() without arguments is deprecated in 8.3."), since: "8.3", severity: "deprecated", fix: "auto", replacement: same("self::class"), snippet: "$type = get_class();" },
  {
    file: "includes/class-files.php",
    line: 40,
    ruleId: "deprecated.splfileinfo_bad_state_ex",
    title: same("->_bad_state_ex()"),
    message: L("SplFileInfo::_bad_state_ex() 8.2'de kullanımdan kalktı.", "SplFileInfo::_bad_state_ex() is deprecated in 8.2."),
    since: "8.2",
    severity: "deprecated",
    fix: "manual",
    replacement: L("çağrıyı silin", "remove the call"),
    snippet: "$file->_bad_state_ex();",
    options: [
      {
        id: "remove",
        title: L("Çağrıyı sil", "Delete the call"),
        effect: L("Büyük olasılıkla kastedilen budur; çağrıdan sonraki kod artık çalışır.", "Likely what was meant; the code after the call now runs."),
        code: "",
        fix: "likely",
        recommended: false,
      },
      {
        id: "throw",
        title: L("Hatayı koru", "Keep the error"),
        effect: L("Çağrının PHP 8'de her zaman fırlattığı Error açıkça fırlatılır.", "Throws the Error the call always threw on PHP 8."),
        code: "throw new \\Error('The parent constructor was not called: the object is in an invalid state');",
        fix: "auto",
        recommended: false,
      },
    ],
  },
  {
    file: "includes/class-payment.php",
    line: 140,
    ruleId: "deprecated.date_rfc7231",
    title: same("DATE_RFC7231"),
    message: L("DATE_RFC7231 saat dilimini yok sayıyordu ve 8.5'te kullanımdan kalktı.", "DATE_RFC7231 ignored the time zone and is deprecated in 8.5."),
    since: "8.5",
    severity: "deprecated",
    fix: "auto",
    replacement: L("biçimi açıkça yazın", "write the format out"),
    snippet: "header('Expires: ' . date(DATE_RFC7231, $expires));",
    options: [
      {
        id: "literal",
        title: L("Aynı çıktı", "Same output"),
        effect: L("Sabitin tuttuğu biçim yazılır; yerel saat yine GMT etiketiyle çıkar.", "Writes the format the constant held; local time is still labelled GMT."),
        code: "header('Expires: ' . date('D, d M Y H:i:s \\G\\M\\T', $expires));",
        fix: "auto",
        recommended: true,
      },
      {
        id: "utc",
        title: L("Gerçek GMT saati", "Real GMT time"),
        effect: L(
          "Saat önce UTC'ye çevrilir. Varsayılan saat dilimi ya da nesnenin saat dilimi UTC değilse çıktı değişir.",
          "The time is converted to UTC first. The output changes when the default or the object's time zone isn't UTC.",
        ),
        code: "header('Expires: ' . gmdate('D, d M Y H:i:s \\G\\M\\T', $expires));",
        fix: "likely",
        recommended: false,
      },
    ],
  },
  { file: "includes/class-payment.php", line: 152, ruleId: "deprecated.backtick", title: L("Ters tırnakla komut çalıştırma", "Backtick shell execution"), message: L("Ters tırnak operatörü 8.5'te kullanımdan kalktı.", "The backtick operator is deprecated in 8.5."), since: "8.5", severity: "deprecated", fix: "auto", replacement: same("shell_exec()"), snippet: "$rev = `git rev-parse HEAD`;" },
  { file: "includes/template.php", line: 22, ruleId: "review.eval", title: L("eval() kullanımı", "eval() usage"), message: L("eval() içindeki kod statik olarak incelenemez; çalıştırdığı kodun yeni sürümde de çalıştığını elle doğrulayın.", "Code inside eval() can't be analyzed statically; check by hand that what it runs still works on the new version."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "eval('?>' . $compiled);" },
  { file: "includes/template.php", line: 48, ruleId: "review.extract", title: L("extract() kullanımı", "extract() usage"), message: L("extract() hangi değişkenleri oluşturduğunu gizler; tip ve null davranışını elle kontrol edin.", "extract() hides which variables it creates; check types and nulls by hand."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "extract($vars, EXTR_SKIP);" },
  { file: "includes/legacy/compat.php", line: 9, ruleId: "review.variable_variable", title: L("Değişken değişkende dizi erişimi ($$a[...])", "Array access on a variable variable ($$a[...])"), message: L("$$ad[...] PHP 7'den beri farklı okunuyor: PHP 5 bunu ${$ad[...]}, PHP 7 ve sonrası ${$ad}[...] olarak okur. Kastedilen anlamı süslü parantezle açıkça yazın.", "$$name[...] has been read differently since PHP 7: PHP 5 read it as ${$name[...]}, PHP 7 and later as ${$name}[...]. Write the meaning you want with braces."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "$value = $$config['key'];" },
];

function summarize(findings: Finding[]): Summary {
  const count = (s: Severity | FixKind) => findings.filter((f) => f.severity === s || f.fix === s).length;
  const weight = (f: Finding) =>
    ({ breaking: 4, deprecated: 1, behavior: 1.5 })[f.severity] * ({ auto: 0.25, likely: 0.6, manual: 1 })[f.fix];
  const penalty = findings.reduce((s, f) => s + weight(f), 0);
  return {
    total: findings.length,
    breaking: count("breaking"),
    deprecated: count("deprecated"),
    behavior: count("behavior"),
    auto: count("auto"),
    likely: count("likely"),
    manual: count("manual"),
    filesAffected: new Set(findings.map((f) => f.file)).size,
    score: Math.round(100 * Math.exp(-penalty / 40)),
  };
}

function lines(spec: [DiffLine["kind"], string][], start: number): DiffLine[] {
  let o = start;
  let n = start;
  return spec.map(([kind, text]) => ({
    kind,
    text,
    oldNo: kind === "add" ? null : o++,
    newNo: kind === "del" ? null : n++,
  }));
}

type RuleSpec = [name: string, title: Localized, since: string, fix: FixKind];

function change(file: string, fix: FixKind, rules: RuleSpec[], hunks: { old: number; spec: [DiffLine["kind"], string][] }[]): FileChange {
  const built = hunks.map((h) => ({ oldStart: h.old, newStart: h.old, lines: lines(h.spec, h.old) }));
  const all = built.flatMap((h) => h.lines);
  return {
    file,
    fix,
    rules: rules.map(([name, title, since, f]) => ({ class: `Rector\\Php${since.replace(".", "")}\\Rector\\${name}`, name, title, since, fix: f })),
    diff: { hunks: built, additions: all.filter((l) => l.kind === "add").length, deletions: all.filter((l) => l.kind === "del").length },
  };
}

const CHANGES: FileChange[] = [
  change("includes/class-cart.php", "likely", [["WhileEachToForeachRector", same("while (each()) → foreach"), "7.2", "likely"], ["CreateFunctionToAnonymousFunctionRector", same("create_function() → closure"), "7.2", "likely"]], [
    { old: 115, spec: [["context", "    public function total(): float"], ["context", "    {"], ["context", "        $sum = 0;"], ["del", "        while (list($key, $item) = each($this->items)) {"], ["add", "        foreach ($this->items as $key => $item) {"], ["context", "            $sum += $item['price'] * $item['qty'];"], ["context", "        }"], ["context", "        return $sum;"]] },
    { old: 202, spec: [["context", "    public function sorted(): array"], ["context", "    {"], ["del", "        usort($items, create_function('$a,$b', 'return $a[\"price\"] - $b[\"price\"];'));"], ["add", "        usort($items, function ($a, $b) {"], ["add", "            return $a[\"price\"] - $b[\"price\"];"], ["add", "        });"], ["context", "        return $items;"], ["context", "    }"]] },
  ]),
  change("includes/class-mailer.php", "likely", [["Utf8DecodeEncodeToMbConvertEncodingRector", same("utf8_encode/decode → mb_convert_encoding"), "8.2", "likely"], ["VariableInStringInterpolationFixerRector", same("\"${var}\" → \"{$var}\""), "8.2", "auto"]], [
    { old: 52, spec: [["context", "    public function send(string $to, string $subject): bool"], ["context", "    {"], ["del", "        $subject = utf8_encode($subject);"], ["add", "        $subject = mb_convert_encoding($subject, 'UTF-8', 'ISO-8859-1');"], ["context", "        $headers = $this->headers();"]] },
    { old: 76, spec: [["context", "        $name = $this->customer->name;"], ["del", "        $body = \"Merhaba ${name}, siparişiniz hazır.\";"], ["add", "        $body = \"Merhaba {$name}, siparişiniz hazır.\";"], ["context", "        $tracking = $this->order->tracking;"], ["del", "        $body .= \"Takip no: ${tracking}\";"], ["add", "        $body .= \"Takip no: {$tracking}\";"], ["context", "        return wp_mail($to, $subject, $body, $headers);"]] },
  ]),
  change("includes/class-order.php", "auto", [["ExplicitNullableParamTypeRector", L("Örtük nullable parametre açıkça yazılır", "Make implicit nullable params explicit"), "8.4", "auto"], ["RemoveGetClassGetParentClassNoArgsRector", same("get_class() → self::class"), "8.3", "auto"]], [
    { old: 10, spec: [["context", "class Order"], ["context", "{"], ["del", "    public function __construct(Customer $customer = null, array $items = null)"], ["add", "    public function __construct(?Customer $customer = null, ?array $items = null)"], ["context", "    {"]] },
    { old: 45, spec: [["context", ""], ["del", "    public function applyCoupon(Coupon $coupon = null): float"], ["add", "    public function applyCoupon(?Coupon $coupon = null): float"], ["context", "    {"]] },
    { old: 89, spec: [["context", "    {"], ["del", "        $type = get_class();"], ["add", "        $type = self::class;"], ["context", "        return $type . '#' . $this->id;"]] },
  ]),
  change("includes/helpers.php", "auto", [["CurlyToSquareBracketArrayStringRector", same("$x{0} → $x[0]"), "7.4", "auto"], ["RenameFunctionRector", L("Kaldırılan fonksiyon yeni adıyla değiştirilir", "Rename removed functions"), "8.0", "auto"]], [
    { old: 14, spec: [["context", "function kuzey_sku_prefix(string $sku): string {"], ["del", "    $first = $sku{0};"], ["add", "    $first = $sku[0];"], ["context", "    return strtoupper($first);"]] },
  ]),
  change("includes/class-payment.php", "auto", [["ShellExecFunctionCallOverBackticksRector", L("Ters tırnak → shell_exec()", "Backticks → shell_exec()"), "8.5", "auto"]], [
    { old: 151, spec: [["context", "    private function revision(): string {"], ["del", "        $rev = `git rev-parse HEAD`;"], ["add", "        $rev = shell_exec('git rev-parse HEAD');"], ["context", "        return trim((string) $rev);"]] },
  ]),
];

const PIN_FILE = "Dockerfile";

const pinChange = (target: string) =>
  change(PIN_FILE, "likely", [["server.php_version_pin", L(`PHP sürümü ${target} olur`, `PHP version becomes ${target}`), target, "likely"]], [
    { old: 1, spec: [["del", "FROM php:7.4-apache"], ["add", `FROM php:${target}-apache`], ["context", "RUN docker-php-ext-install pdo_mysql"]] },
  ]);

const DEPS: Omit<DependencyReport, "target" | "rootStatus"> = {
  rootConstraint: ">=5.6",
  hasLockFile: true,
  packages: [
    { name: "phpmailer/phpmailer", version: "v5.2.28", phpConstraint: ">=5.0.0 <8.0", status: "incompatible", dev: false },
    { name: "illuminate/database", version: "v8.83.27", phpConstraint: "^7.3|^8.0", status: "incompatible", dev: false, note: L("Bu sürüm resmi olarak en fazla PHP 8.1'i destekliyor.", "This version officially supports PHP up to 8.1.") },
    { name: "guzzlehttp/guzzle", version: "6.5.8", phpConstraint: "^7.2.5 || ^8.0", status: "compatible", dev: false },
    { name: "monolog/monolog", version: "1.27.1", phpConstraint: ">=5.3.0", status: "compatible", dev: false },
    { name: "tecnickcom/tcpdf", version: "6.2.26", phpConstraint: null, status: "unknown", dev: false },
    { name: "phpunit/phpunit", version: "7.5.20", phpConstraint: "^7.1", status: "incompatible", dev: true },
    { name: "symfony/polyfill-mbstring", version: "v1.31.0", phpConstraint: ">=7.2", status: "compatible", dev: false },
  ],
  extensions: [
    { name: "mysqli", uses: 14, file: "includes/class-db.php", line: 31, symbol: "mysqli_connect()", declared: false, removed: false, available: true },
    { name: "gd", uses: 6, file: "includes/media.php", line: 12, symbol: "imagecreatetruecolor()", declared: false, removed: false, available: true },
    { name: "imap", uses: 2, file: "includes/mailbox.php", line: 8, symbol: "imap_open()", declared: false, removed: true, available: false },
    { name: "curl", uses: 3, file: "includes/http.php", line: 20, symbol: "curl_init()", declared: true, removed: false, available: true },
  ],
  wordpress: [
    { kind: "plugin", slug: "contact-form-7", name: "Contact Form 7", version: "5.1.6", requiresPhp: "5.6", dir: "wp-content/plugins/contact-form-7" },
    { kind: "plugin", slug: "old-gallery", name: "Old Gallery", version: "1.0.2", requiresPhp: null, dir: "wp-content/plugins/old-gallery" },
    { kind: "theme", slug: "twentytwenty", name: "Twenty Twenty", version: "1.2", requiresPhp: "5.2.4", dir: "wp-content/themes/twentytwenty" },
  ],
};

export function mockApi(
  opts: {
    delay?: number;
    scale?: number;
    staleIgnores?: IgnoreEntry[];
    config?: ProjectConfig;
    deps?: (report: DependencyReport) => DependencyReport;
    gaps?: DockerGap[];
    conflicts?: SnapshotConflict[];
  } = {},
): Api {
  const delay = opts.delay ?? 450;
  const listeners = new Set<(p: ScanProgress) => void>();
  let settings: Settings = { phpPath: null, snapshotLimit: 30, editor: null, editorCommand: null };
  const ignores = new Map<string, string>();
  let stale = opts.staleIgnores ?? [];
  let lastScan: ScanResult | null = null;
  let runtimeLog: string | null = null;
  let runtimeCheck: CheckRun | null = null;
  let runtimeRoots: string[] = [];
  let dockerImages = 0;
  let dockerRuns: DockerSummary[] = [];
  let dockerListener: ((step: DockerStep) => void) | null = null;
  const checkRuns: CheckSummary[] = [];
  let config: ProjectConfig = opts.config ?? { target: null, exclude: [], baseline: null, docker: { source: null, extensions: null, pages: null, dump: null, prepare: null, test: null }, git: { requireClean: false, branchPerBatch: false } };
  const info = async (path: string): Promise<ProjectInfo> => {
    await sleep(delay / 3);
    return {
      root: path,
      name: path.split(/[\\/]/).filter(Boolean).pop() ?? path,
      framework: "wordpress",
      phpFiles: 148,
      phpConstraint: ">=5.6",
      currentVersion: "7.4",
      hasComposerLock: true,
      hasVendor: true,
      isGitRepo: true,
      excludedDirs: ["wp-admin", "wp-includes", ...config.exclude],
      config,
      configError: null,
    };
  };
  let dockerTrial: DockerComparison | null = null;
  let dockerStopped = false;
  const withRuntime = (findings: Finding[]): { findings: Finding[]; runtime: ScanResult["runtime"] } => {
    if (!runtimeLog) return { findings: findings.map((f) => ({ ...f, runtimeHits: 0 })), runtime: null };
    const hit = new Set(findings.filter((f) => f.ruleId.startsWith("removed.")).slice(0, 3).map((f) => f.id));
    const marked = findings.filter((f) => !f.ruleId.startsWith("runtime.")).map((f) => ({ ...f, runtimeHits: hit.has(f.id) ? 12 : 0 }));
    const extra: Finding = {
      id: "includes/cart.php:44:1:runtime.warning",
      file: "includes/cart.php",
      line: 44,
      column: 1,
      ruleId: "runtime.warning",
      title: L('Undefined array key "qty"', 'Undefined array key "qty"'),
      message: L("Bu mesaj çalışma anı log'unda 7 kez görüldü; statik tarama burada bir şey bulmadı.", "This message appeared 7 times in the runtime log; the static scan found nothing here."),
      since: "8.4",
      severity: "behavior",
      fix: "manual",
      replacement: null,
      snippet: "$qty = $item['qty'];",
      runtimeHits: 7,
    };
    return { findings: [...marked, extra], runtime: { file: runtimeLog, events: 31, confirmed: hit.size, added: 1, outside: runtimeRoots.length ? 9 : 4, outsideFatals: 0, check: runtimeCheck, roots: runtimeRoots } };
  };
  let rectorCache = 18_400_000;
  const history = new Map<string, { at: number; score: number; total: number; counts: ScanCounts; ids: string[] }[]>();
  const pastScan = (hoursAgo: number, score: number, breaking: number, deprecated: number, behavior: number, files: number): HistoryEntry => ({
    at: Math.floor(Date.now() / 1000) - hoursAgo * 3600,
    target: "8.4",
    score,
    total: breaking + deprecated + behavior,
    counts: { breaking, deprecated, behavior, files },
  });
  const pastScans = [pastScan(74, 6, 16, 14, 6, 14), pastScan(69, 9, 15, 13, 6, 14), pastScan(50, 13, 14, 12, 5, 13), pastScan(45, 16, 13, 11, 5, 12), pastScan(26, 19, 12, 10, 4, 12), pastScan(21, 22, 11, 9, 4, 11)];
  const track = (scan: ScanResult): ScanResult => {
    const list = history.get(scan.target) ?? [];
    const prev = list[list.length - 1];
    const ids = scan.findings.map((f) => f.id);
    const added = prev ? ids.filter((id) => !prev.ids.includes(id)).length : null;
    const fixed = prev ? prev.ids.filter((id) => !ids.includes(id)).length : null;
    const { breaking, deprecated, behavior, score, total } = scan.summary;
    list.push({ at: Math.floor(Date.now() / 1000) + list.length, score, total, counts: { breaking, deprecated, behavior, files: new Set(scan.findings.map((f) => f.file)).size }, ids });
    history.set(scan.target, list);
    const entries = list.slice(-30).map(({ at, score, total, counts }) => ({ at, target: scan.target, score, total, counts }));
    return { ...scan, progress: { history: entries, newSinceLast: added, fixedSinceLast: fixed } };
  };
  const withIgnores = (scan: ScanResult): ScanResult => {
    lastScan = scan;
    const all = [...scan.findings, ...scan.ignored.map((i) => i.finding)];
    const merged = withRuntime(all);
    const findings = merged.findings.filter((f) => !ignores.has(f.id));
    const ignored = merged.findings.filter((f) => ignores.has(f.id)).map((finding) => ({ finding, reason: ignores.get(finding.id)! }));
    return { ...scan, findings, ignored, staleIgnores: stale, runtime: merged.runtime, summary: summarize(findings) };
  };
  let pending = [...CHANGES];
  let cancelled = false;
  let previewId = 0;
  const fixed = new Set<string>();
  let snapshots: Snapshot[] = [
    { id: "1761400000000", createdAt: Date.now() - 1000 * 60 * 60 * 26, projectRoot: MOCK_ROOT, label: "Uyumluluk düzeltmeleri · PHP 8.1", files: [{ path: "includes/helpers.php", existed: true }] },
  ];

  const toolchain = (): ToolchainStatus => ({
    php: { path: "/opt/homebrew/bin/php", version: "8.4.27" },
    composer: { path: "/opt/homebrew/bin/composer", version: "2.8.12" },
    rector: { path: "~/Library/Application Support/io.github.efekoca.ferry/tools/vendor/rector/rector/bin/rector", version: "2.8.3" },
    docker: { path: "/usr/local/bin/docker", version: "Docker version 28.4.0" },
    toolsDir: "~/Library/Application Support/io.github.efekoca.ferry/tools",
    platform: "macos",
  });

  return {
    pickProjectFolder: async () => MOCK_ROOT,
    pickSaveFile: async (name) => `/Users/demo/Desktop/${name}`,
    pickPhpBinary: async () => "/opt/homebrew/opt/php@8.4/bin/php",
    pickProbeFile: async (name) => `/Users/demo/Desktop/${name}`,
    writeProbe: async () => "3f9c2a7be41d08c6a5e2f1b07d9c3e88",
    pickLogFile: async () => "/var/log/php/php_errors.log",
    importRuntimeLog: async (_path, log) => {
      if (!lastScan) throw new Error("Scan the project first.");
      runtimeLog = log.split("/").pop() ?? log;
      runtimeCheck = null;
      return withIgnores(lastScan);
    },
    setRuntimeRoots: async (_path, roots) => {
      if (!lastScan) throw new Error("Scan the project first.");
      runtimeRoots = roots.split(",").map((r) => r.trim()).filter(Boolean);
      return withIgnores(lastScan);
    },
    dockerHistory: async () => dockerRuns,
    dockerImageUsage: async () => ({ images: dockerImages, bytes: dockerImages * 650_000_000 }),
    removeDockerImages: async () => {
      const removed = dockerImages;
      dockerImages = 0;
      return removed;
    },
    sshHosts: async () => ["prod", "staging"],
    databasePlan: async () => ({ engine: "mysql" as const, host: "127.0.0.1", name: "magaza", user: "app", live: true, seed: null, dumps: ["database/dump.sql"] }),
    dockerCompare: async (_path, source, target, extensions, cleanImages, pages, database, _prepare, _test, fix) => {
      if (!lastScan) throw new Error("Scan the project first.");
      const run = (version: string, lintErrors: number, passed: boolean, log: string) => ({
        version,
        image: `php:${version}-cli`,
        lintErrors,
        lintFiles: 148,
        lintStopped: false,
        pagesStopped: false,
        pagesLimited: false,
        tests: { exitCode: passed ? 0 : 2, timedOut: false, passed, byFile: false, ran: true, total: 12 },
        failedTests: passed ? [] : ["CartTest::testTotal"],
        missingExtensions: extensions ? [] : ["mysqli"],
        installedExtensions: extensions ? ["mysqli"] : [],
        imageFailed: false,
        events: [],
        log,
      });
      const event = { path: "/app/includes/class-cart.php", line: 118, level: "fatal" as const, message: "Error: Call to undefined function each()", count: 1 };
      dockerStopped = false;
      for (const [step, version] of [["lint", source], ["pages", source], ["tests", source], ["lint", target], ["pages", target], ["tests", target]] as const) {
        dockerListener?.({ step, version });
        await new Promise((r) => setTimeout(r, delay));
        if (dockerStopped) throw new Error("The comparison was stopped.");
      }
      if (fix) {
        const comparison = { verdict: "clean" as const, gaps: [], stamp: "mock", source: run(source, 0, true, "OK (12 tests)"), target: run(target, 0, true, "OK (12 tests)"), dependenciesInstalled: true, installLog: "", newFailures: [], dataChanges: [], databaseUsed: !!database, pages: [], newOnTarget: [], goneOnTarget: [event], origins: [] };
        const none = { fixed: [], remaining: [], introduced: [] };
        dockerTrial = comparison;
        return { comparison, scan: null, outcome: { messages: { ...none, fixed: [event] }, tests: { ...none, fixed: ["CartTest::testTotal"] }, pages: { ...none, fixed: pages.length ? ["/cart.php"] : [] }, tables: none, lintBefore: 1, lintAfter: 0, regressedBefore: true, regressedAfter: false, origins: [] } };
      }
      dockerImages = extensions && !cleanImages ? 2 : dockerImages;
      dockerRuns = [{ at: Date.now() / 1000, source, target, regressed: true, incomplete: false, stamp: "mock", testsCompared: true, pages: pages.length ? 2 : 0, brokenPages: pages.length ? 1 : 0, differingPages: pages.length ? 1 : 0, unverifiedPages: 0, newFailures: 1, newMessages: 1, dataChanges: database ? 1 : 0 }, ...dockerRuns];
      runtimeLog = `Docker · PHP ${source} → ${target}`;
      runtimeCheck = null;
      return {
        comparison: { verdict: opts.gaps?.length ? ("incomplete" as const) : ("regressed" as const), gaps: opts.gaps ?? [], stamp: "mock", source: run(source, 0, true, "OK (12 tests)"), target: { ...run(target, 1, false, "ERRORS!"), events: [event] }, dependenciesInstalled: true, installLog: "", newFailures: ["CartTest::testTotal"], dataChanges: database ? [{ table: "orders", sourceRows: 3, targetRows: 4 }] : [], databaseUsed: !!database, pages: pages.length ? [{ path: "/cart.php", change: "errors" as const, sourceStatus: 200, targetStatus: 200, newErrors: ["Uncaught Error: Call to undefined function each()"], sourceText: "Sepet", targetText: "Fatal error" }, { path: "/", change: "same" as const, sourceStatus: 200, targetStatus: 200, newErrors: [], sourceText: "Mağaza", targetText: "Mağaza" }] : [], newOnTarget: [event], goneOnTarget: [], origins: [{ kind: "project" as const, name: "", version: null, messages: 1, fatal: 1, advice: null }, { kind: "package" as const, name: "acme/mailer", version: "1.4.0", messages: 2, fatal: 0, advice: null }, { kind: "core" as const, name: "WordPress", version: null, messages: 1, fatal: 0, advice: { tr: "Önce WordPress'i güncelleyin.", en: "Update WordPress first." } }] },
        scan: withIgnores(lastScan),
        outcome: null,
      };
    },
    keepDockerTrial: async () => {
      if (!dockerTrial) throw new Error("Try the fixes first.");
      return { comparison: dockerTrial, scan: null, outcome: null };
    },
    cancelDockerCompare: async () => {
      dockerStopped = true;
    },
    onDockerStep: async (listener) => {
      dockerListener = listener;
      return () => {
        dockerListener = null;
      };
    },
    localPhpVersions: async () => [
      { line: "8.4", version: "8.4.27", path: "/opt/homebrew/opt/php@8.4/bin/php" },
      { line: "8.5", version: "8.5.12", path: "/opt/homebrew/bin/php" },
    ],
    runLocalCheck: async (_path, target, kind) => {
      if (!lastScan) throw new Error("Scan the project first.");
      await sleep(delay);
      runtimeLog = kind === "lint" ? `php -l · PHP ${target}` : `PHPUnit · PHP ${target}`;
      runtimeCheck = {
        kind,
        command: kind === "lint" ? "php -l" : "php vendor/bin/phpunit --display-deprecations --display-warnings --no-progress",
        exitCode: kind === "lint" ? null : 0,
        timedOut: false,
        passed: true,
        durationMs: 1840,
        files: kind === "lint" ? 128 : 0,
        failed: 0,
        notRun: 0,
        log: kind === "lint" ? "" : "OK (42 tests, 97 assertions)\n",
      };
      checkRuns.unshift({ at: Math.floor(Date.now() / 1000) + checkRuns.length, target, label: runtimeLog, kind, passed: true, timedOut: false, exitCode: 0, failed: 0, durationMs: 1840 });
      return withIgnores(lastScan);
    },
    closeProject: async () => {
      runtimeLog = null;
      runtimeCheck = null;
    },
    clearRuntimeLog: async () => {
      if (!lastScan) throw new Error("Scan the project first.");
      runtimeLog = null;
      runtimeCheck = null;
      return withIgnores(lastScan);
    },
    inspectProject: info,
    saveProjectConfig: async (path, next) => {
      config = next;
      return info(path);
    },
    scanProject: async (_path, target, run): Promise<ScanResult> => {
      const total = 148;
      const steps = 12;
      for (let i = 0; i <= steps; i++) {
        const done = Math.round((total * i) / steps);
        listeners.forEach((l) => l({ run, done, total, file: i < steps ? `includes/file-${done}.php` : "" }));
        await sleep(delay / steps);
      }
      const seeds = SEEDS.filter((s) => versionLte(s.since, target) && !(fixed.has(s.file) && s.fix !== "manual"));
      const findings: Finding[] = Array.from({ length: opts.scale ?? 1 }, (_, copy) =>
        seeds.map((s) => {
          const file = copy ? s.file.replace(/\.php$/, `-${copy}.php`) : s.file;
          return { ...s, file, column: 9, id: `${file}:${s.line}:9:${s.ruleId}`, runtimeHits: 0 };
        }),
      ).flat();
      return track(withIgnores({ target, filesFound: total, filesScanned: total, linesScanned: 18432, stamp: "mock", findings, errors: [], durationMs: 184, summary: summarize(findings), ignored: [], staleIgnores: [] }));
    },
    onScanProgress: async (handler) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
    wordpressLatest: async (items) => {
      await sleep(delay / 3);
      const known: Record<string, WpLatest> = {
        "contact-form-7": { kind: "plugin", slug: "contact-form-7", version: "6.2.4", requiresPhp: "7.4", lastUpdated: "2026-10-29", closed: false },
        "old-gallery": { kind: "plugin", slug: "old-gallery", version: null, requiresPhp: null, lastUpdated: null, closed: true },
        twentytwenty: { kind: "theme", slug: "twentytwenty", version: "3.3", requiresPhp: "5.2.4", lastUpdated: "2026-10-28", closed: false },
      };
      return items.map((i) => known[i.slug]).filter(Boolean);
    },
    findCompatibleVersions: async (_target, packages) => {
      await sleep(delay / 3);
      const known: Record<string, [string, boolean]> = { "phpmailer/phpmailer": ["v6.9.1", true], "illuminate/database": ["v10.0.0", true], "phpunit/phpunit": ["9.6.0", true], "acme/mailer": ["v2.1.0", true] };
      return packages.filter((p) => known[p.name]).map((p) => ({ name: p.name, version: known[p.name][0], majorChange: known[p.name][1] }));
    },
    gitStatus: async (_path, files) => ({ repo: true, dirty: files.filter((f) => f === "includes/class-cart.php") }),
    compareTargets: async () => {
      await sleep(delay);
      return (["8.2", "8.3", "8.4", "8.5"] as const).map((target) => {
        const findings = SEEDS.filter((s) => versionLte(s.since, target)).map((s) => ({ ...s, id: s.file + s.line, column: 1, runtimeHits: 0 }));
        return { target, summary: summarize(findings), blockingPackages: target === "8.2" ? 2 : 3, rootBlocks: false };
      });
    },
    checkDependencies: async (_path, target) => {
      await sleep(delay / 3);
      const report: DependencyReport = { ...DEPS, target, rootStatus: "compatible" };
      return opts.deps ? opts.deps(report) : report;
    },
    toolchainStatus: async () => {
      await sleep(delay / 4);
      return toolchain();
    },
    installRector: async () => {
      await sleep(delay * 3);
      return toolchain();
    },
    rectorPreview: async (_path, target, mode, withPins, _choices) => {
      cancelled = false;
      const id = ++previewId;
      await sleep(delay * 2);
      if (cancelled) throw new Error("Rector was stopped.");
      if (mode === "pins") return { id, target, mode, changes: fixed.has(PIN_FILE) ? [] : [pinChange(target)], errors: [], durationMs: 12 };
      const changes = pending.filter((c) => c.rules.some((r) => !r.since || versionLte(r.since, target)));
      const pins = withPins && !fixed.has(PIN_FILE) ? [pinChange(target)] : [];
      return { id, target, mode, changes: [...changes, ...pins], errors: [], durationMs: 2140 };
    },
    cancelScan: async () => {},
    cancelLocalCheck: async () => {},
    cancelPreview: async () => {
      cancelled = true;
    },
    applyChanges: async (path, id, files, label, _skipped, commitMessage) => {
      await sleep(delay);
      if (id !== previewId) throw new Error("The preview is out of date. Calculate the changes again.");
      const snapshot: Snapshot = { id: String(Date.now()), createdAt: Date.now(), projectRoot: path, label, files: files.map((p) => ({ path: p, existed: true })) };
      snapshots = [snapshot, ...snapshots];
      pending = pending.filter((c) => !files.includes(c.file));
      files.forEach((f) => fixed.add(f));
      return { snapshot, written: files, lintErrors: [], lintChecked: true, branch: commitMessage ? `ferry/php-${files.length}` : null, commit: commitMessage ? "abc1234" : null };
    },
    listSnapshots: async () => snapshots,
    restoreSnapshot: async (_path, id) => {
      await sleep(delay);
      const snap = snapshots.find((s) => s.id === id);
      if (!snap) throw new Error("snapshot not found");
      pending = [...CHANGES];
      snap.files.forEach((f) => fixed.delete(f.path));
      return snap.files.map((f) => f.path);
    },
    snapshotChanges: async (_path, id) => {
      await sleep(delay / 2);
      const snap = snapshots.find((s) => s.id === id);
      if (!snap) throw new Error("snapshot not found");
      return CHANGES.filter((c) => snap.files.some((f) => f.path === c.file));
    },
    snapshotConflicts: async () => opts.conflicts ?? [],
    scanHistory: async () => [...pastScans, ...[...history].flatMap(([target, list]) => list.map(({ at, score, total, counts }) => ({ at, target, score, total, counts })))],
    rectorCacheSize: async () => rectorCache,
    clearRectorCache: async () => {
      rectorCache = 0;
    },
    backupUsage: async () => ({
      snapshots: snapshots.length,
      projects: snapshots.length ? 1 : 0,
      bytes: snapshots.reduce((sum, s) => sum + s.files.length * 6144, 0),
    }),
    deleteAllBackups: async () => {
      snapshots = [];
    },
    deleteSnapshot: async (_path, id) => {
      snapshots = snapshots.filter((s) => s.id !== id);
    },
    exportReport: async () => {
      await sleep(delay / 2);
    },
    revealPath: async () => {},
    ignoreFinding: async (_path, id, reason) => {
      if (!lastScan) throw new Error("Scan the project first.");
      ignores.set(id, reason.trim());
      return withIgnores(lastScan);
    },
    unignoreFinding: async (_path, id) => {
      if (!lastScan) throw new Error("Scan the project first.");
      ignores.delete(id);
      return withIgnores(lastScan);
    },
    openInEditor: async () => {},
    sourceLines: async (_path, _file, line) => {
      const code = ["<?php", "$items = load_items();", "$total = mysql_query($sql);", "$rows = array();", "return $rows;"];
      return code.map((text, i) => ({ number: Math.max(1, line - 2) + i, text }));
    },
    pruneIgnores: async () => {
      if (!lastScan) throw new Error("Scan the project first.");
      stale = [];
      return withIgnores(lastScan);
    },
    interruptedWrites: async () => [],
    dismissInterrupted: async () => {},
    checkHistory: async () => checkRuns,
    getSettings: async () => settings,
    saveSettings: async (s) => {
      settings = s;
    },
    setLanguage: async () => {},
    setWindowTheme: async () => {},
  };
}
