import type { Api } from "./api";
import type { Localized } from "../i18n";
import type {
  DependencyReport,
  DiffLine,
  FileChange,
  Finding,
  FixKind,
  ProjectInfo,
  ScanProgress,
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

type Seed = Omit<Finding, "id" | "column">;

const SEEDS: Seed[] = [
  { file: "wp-includes/version.php", line: 16, ruleId: "framework.wordpress_version", title: same("WordPress 6.2"), message: L("WordPress 6.2, PHP 8.4 ile çalışmaz. PHP 8.4 için en az WordPress 6.7 gerekiyor; önce WordPress'i güncelleyin.", "WordPress 6.2 doesn't run on PHP 8.4. PHP 8.4 needs at least WordPress 6.7; update WordPress first."), since: "8.3", severity: "breaking", fix: "manual", replacement: same("WordPress 6.7+"), snippet: "$wp_version = '6.2.2';" },
  { file: "includes/class-db.php", line: 42, ruleId: "removed.mysql", title: same("mysql_connect()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "$this->link = mysql_connect($host, $user, $pass);" },
  { file: "includes/class-db.php", line: 58, ruleId: "removed.mysql", title: same("mysql_query()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "$res = mysql_query($sql, $this->link);" },
  { file: "includes/class-db.php", line: 61, ruleId: "removed.mysql", title: same("mysql_fetch_assoc()"), message: MYSQL, since: "7.0", severity: "breaking", fix: "manual", replacement: MYSQL_REPL, snippet: "while ($row = mysql_fetch_assoc($res)) {" },
  { file: "includes/class-cart.php", line: 118, ruleId: "removed.each", title: same("each()"), message: L("each() kaldırıldı. while (list() = each()) döngüleri foreach'e çevrilir.", "each() was removed. while (list() = each()) loops become foreach."), since: "8.0", severity: "breaking", fix: "likely", replacement: L("foreach ya da key()/current()/next()", "foreach or key()/current()/next()"), snippet: "while (list($key, $item) = each($this->items)) {" },
  { file: "includes/class-cart.php", line: 204, ruleId: "removed.create_function", title: same("create_function()"), message: L("create_function() kaldırıldı. Kod dizesi bir closure'a çevrilmeli; dize çalışma anında üretiliyorsa elle inceleyin.", "create_function() was removed. The code string has to become a closure; review it by hand if the string is built at runtime."), since: "8.0", severity: "breaking", fix: "likely", replacement: L("anonim fonksiyon (closure)", "an anonymous function (closure)"), snippet: "usort($items, create_function('$a,$b', 'return $a[\"price\"] - $b[\"price\"];'));" },
  { file: "includes/helpers.php", line: 15, ruleId: "removed.curly_offset", title: L("Süslü parantezle dizi erişimi", "Curly brace offset access"), message: L("$x{0} yazımı PHP 8.0'da kaldırıldı ve ayrıştırma hatası verir.", "The $x{0} syntax was removed in PHP 8.0 and is now a parse error."), since: "8.0", severity: "breaking", fix: "auto", replacement: same("$x[0]"), snippet: "$first = $sku{0};" },
  { file: "includes/helpers.php", line: 33, ruleId: "removed.get_magic_quotes_gpc", title: same("get_magic_quotes_gpc()"), message: L("Magic quotes PHP 5.4'ten beri yok; çağrının yerine false yazılabilir.", "Magic quotes have been gone since PHP 5.4; replace the call with false."), since: "8.0", severity: "breaking", fix: "manual", replacement: same("false"), snippet: "if (get_magic_quotes_gpc()) { $v = stripslashes($v); }" },
  { file: "includes/helpers.php", line: 71, ruleId: "removed.ereg", title: same("ereg()"), message: L("POSIX regex fonksiyonları kaldırıldı. Desen, sınırlayıcılarla PCRE sözdizimine çevrilmelidir.", "POSIX regex functions were removed. The pattern has to be converted to PCRE syntax with delimiters."), since: "7.0", severity: "breaking", fix: "likely", replacement: same("preg_match"), snippet: "if (ereg('^[0-9]{4}$', $code)) {" },
  { file: "admin/settings-page.php", line: 27, ruleId: "deprecated.filter_sanitize_string", title: same("FILTER_SANITIZE_STRING"), message: L("FILTER_SANITIZE_STRING kullanımdan kalktı. İhtiyacınıza göre htmlspecialchars() ya da strip_tags() kullanın.", "FILTER_SANITIZE_STRING is deprecated. Use htmlspecialchars() or strip_tags(), depending on what you need."), since: "8.1", severity: "deprecated", fix: "manual", replacement: same("htmlspecialchars()"), snippet: "$title = filter_input(INPUT_POST, 'title', FILTER_SANITIZE_STRING);" },
  { file: "admin/settings-page.php", line: 88, ruleId: "deprecated.strftime", title: same("strftime()"), message: L("strftime() kullanımdan kalktı ve PHP 9'da kaldırılacak. Biçim karakterleri date() ile birebir eşleşmez.", "strftime() is deprecated and will be removed in PHP 9. Its format characters don't map one-to-one to date()."), since: "8.1", severity: "deprecated", fix: "manual", replacement: L("IntlDateFormatter ya da date()", "IntlDateFormatter or date()"), snippet: "echo strftime('%d %B %Y', $order->created);" },
  { file: "includes/class-mailer.php", line: 54, ruleId: "deprecated.utf8_encode", title: same("utf8_encode()"), message: L("utf8_encode() kullanımdan kalktı. Kaynak metnin gerçekten ISO-8859-1 olduğundan emin olun.", "utf8_encode() is deprecated. Make sure the source text really is ISO-8859-1."), since: "8.2", severity: "deprecated", fix: "likely", replacement: same("mb_convert_encoding($s, 'UTF-8', 'ISO-8859-1')"), snippet: "$subject = utf8_encode($subject);" },
  { file: "includes/class-mailer.php", line: 77, ruleId: "deprecated.dollar_brace_interpolation", title: DOLLAR_BRACE_TITLE, message: DOLLAR_BRACE, since: "8.2", severity: "deprecated", fix: "auto", replacement: same("\"{$var}\""), snippet: "$body = \"Merhaba ${name}, siparişiniz hazır.\";" },
  { file: "includes/class-mailer.php", line: 79, ruleId: "deprecated.dollar_brace_interpolation", title: DOLLAR_BRACE_TITLE, message: DOLLAR_BRACE, since: "8.2", severity: "deprecated", fix: "auto", replacement: same("\"{$var}\""), snippet: "$body .= \"Takip no: ${tracking}\";" },
  { file: "includes/class-order.php", line: 12, ruleId: "deprecated.implicit_nullable", title: NULLABLE_TITLE, message: NULLABLE, since: "8.4", severity: "deprecated", fix: "auto", replacement: NULLABLE_REPL, snippet: "public function __construct(Customer $customer = null, array $items = null)" },
  { file: "includes/class-order.php", line: 46, ruleId: "deprecated.implicit_nullable", title: NULLABLE_TITLE, message: NULLABLE, since: "8.4", severity: "deprecated", fix: "auto", replacement: NULLABLE_REPL, snippet: "public function applyCoupon(Coupon $coupon = null): float" },
  { file: "includes/class-order.php", line: 90, ruleId: "deprecated.get_class_no_args", title: L("Parametresiz get_class()", "get_class() without arguments"), message: L("Parametresiz get_class() ve get_parent_class() çağrıları 8.3'te kullanımdan kalktı.", "Calling get_class() or get_parent_class() without arguments is deprecated in 8.3."), since: "8.3", severity: "deprecated", fix: "auto", replacement: same("self::class"), snippet: "$type = get_class();" },
  { file: "includes/class-payment.php", line: 31, ruleId: "deprecated.e_strict", title: same("E_STRICT"), message: L("E_STRICT kullanımdan kalktı; bu hata seviyesi artık hiç tetiklenmiyor.", "E_STRICT is deprecated; this error level is never raised anymore."), since: "8.4", severity: "deprecated", fix: "manual", replacement: L("E_STRICT'i kaldırın", "remove E_STRICT"), snippet: "error_reporting(E_ALL & ~E_STRICT);" },
  { file: "includes/class-payment.php", line: 140, ruleId: "deprecated.curl_close", title: same("curl_close()"), message: L("PHP 8.0'dan beri hiçbir şey yapmıyor, 8.5'te kullanımdan kalktı.", "Has done nothing since PHP 8.0 and is deprecated in 8.5."), since: "8.5", severity: "deprecated", fix: "auto", replacement: L("çağrıyı silin", "remove the call"), snippet: "curl_close($ch);" },
  { file: "includes/class-payment.php", line: 152, ruleId: "deprecated.backtick", title: L("Ters tırnakla komut çalıştırma", "Backtick shell execution"), message: L("Ters tırnak operatörü 8.5'te kullanımdan kalktı.", "The backtick operator is deprecated in 8.5."), since: "8.5", severity: "deprecated", fix: "auto", replacement: same("shell_exec()"), snippet: "$rev = `git rev-parse HEAD`;" },
  { file: "includes/template.php", line: 22, ruleId: "review.eval", title: L("eval() kullanımı", "eval() usage"), message: L("eval() içindeki kod statik olarak incelenemez; çalıştırdığı kodun yeni sürümde de çalıştığını elle doğrulayın.", "Code inside eval() can't be analyzed statically; check by hand that what it runs still works on the new version."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "eval('?>' . $compiled);" },
  { file: "includes/template.php", line: 48, ruleId: "review.extract", title: L("extract() kullanımı", "extract() usage"), message: L("extract() hangi değişkenleri oluşturduğunu gizler; tip ve null davranışını elle kontrol edin.", "extract() hides which variables it creates; check types and nulls by hand."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "extract($vars, EXTR_SKIP);" },
  { file: "includes/legacy/compat.php", line: 9, ruleId: "review.variable_variable", title: L("Değişken değişken ($$)", "Variable variable ($$)"), message: L("$$ kullanımı statik analizden kaçar ve PHP 7'den beri farklı bir öncelikle değerlendirilir.", "Variable variables escape static analysis and have been evaluated with different precedence since PHP 7."), since: "7.0", severity: "behavior", fix: "manual", replacement: null, snippet: "$$key = $value;" },
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
};

export function mockApi(opts: { delay?: number; scale?: number } = {}): Api {
  const delay = opts.delay ?? 450;
  const listeners = new Set<(p: ScanProgress) => void>();
  let settings: Settings = { phpPath: null, snapshotLimit: null };
  let pending = [...CHANGES];
  let cancelled = false;
  let previewId = 0;
  const fixed = new Set<string>();
  let snapshots: Snapshot[] = [
    { id: "1761400000000", createdAt: Date.now() - 1000 * 60 * 60 * 26, projectRoot: MOCK_ROOT, label: "Uyumluluk düzeltmeleri · PHP 8.1", files: [{ path: "includes/helpers.php", existed: true }] },
  ];

  const toolchain = (): ToolchainStatus => ({
    php: { path: "/opt/homebrew/bin/php", version: "8.4.14" },
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
    inspectProject: async (path): Promise<ProjectInfo> => {
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
        excludedDirs: ["wp-admin", "wp-includes"],
      };
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
          return { ...s, file, column: 9, id: `${file}:${s.line}:9:${s.ruleId}` };
        }),
      ).flat();
      return { target, filesScanned: total, linesScanned: 18432, findings, errors: [], durationMs: 184, summary: summarize(findings) };
    },
    onScanProgress: async (handler) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
    checkDependencies: async (_path, target) => {
      await sleep(delay / 3);
      return { ...DEPS, target, rootStatus: "compatible" };
    },
    toolchainStatus: async () => {
      await sleep(delay / 4);
      return toolchain();
    },
    installRector: async () => {
      await sleep(delay * 3);
      return toolchain();
    },
    rectorPreview: async (_path, target, mode) => {
      cancelled = false;
      const id = ++previewId;
      await sleep(delay * 2);
      if (cancelled) throw new Error("Rector was stopped.");
      const changes = pending.filter((c) => c.rules.some((r) => !r.since || versionLte(r.since, target)));
      return { id, target, mode, changes, errors: [], durationMs: 2140 };
    },
    cancelPreview: async () => {
      cancelled = true;
    },
    applyChanges: async (path, id, files, label) => {
      await sleep(delay);
      if (id !== previewId) throw new Error("The preview is out of date. Calculate the changes again.");
      const snapshot: Snapshot = { id: String(Date.now()), createdAt: Date.now(), projectRoot: path, label, files: files.map((p) => ({ path: p, existed: true })) };
      snapshots = [snapshot, ...snapshots];
      pending = pending.filter((c) => !files.includes(c.file));
      files.forEach((f) => fixed.add(f));
      return { snapshot, written: files, lintErrors: [] };
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
    snapshotConflicts: async () => [],
    deleteSnapshot: async (_path, id) => {
      snapshots = snapshots.filter((s) => s.id !== id);
    },
    exportReport: async () => {
      await sleep(delay / 2);
    },
    revealPath: async () => {},
    getSettings: async () => settings,
    saveSettings: async (s) => {
      settings = s;
    },
    setLanguage: async () => {},
    setWindowTheme: async () => {},
  };
}
