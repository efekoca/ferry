<?php
// Reads crates/ferry-core/src/symbols.rs and checks on this PHP that every removed symbol is gone
// and every deprecated one that still exists raises a deprecation (some were removed later).
// Prints one line per mismatch.
$src = file_get_contents($argv[1]);
preg_match_all('/pub const (\w+): [^=]+= &\[(.*?)\n\];/s', $src, $tables, PREG_SET_ORDER);
$rows = array();
foreach ($tables as $t) {
    preg_match_all('/\("([^"]+)", \((\d+), (\d+)\)(?:, "([^"]*)")?(?:, "([^"]*)")?\)/', $t[2], $m, PREG_SET_ORDER);
    $rows[$t[1]] = $m;
}
$bad = 0;
$deprecations = 0;
set_error_handler(function ($no) use (&$deprecations) {
    if ($no === E_DEPRECATED) {
        $deprecations++;
    }
    return true;
});
foreach ($rows['REMOVED_FUNCTIONS'] as $r) {
    if (function_exists($r[1])) { echo "still exists: function {$r[1]}\n"; $bad++; }
}
foreach ($rows['REMOVED_CONSTANTS'] as $r) {
    if (defined($r[1])) { echo "still exists: constant {$r[1]}\n"; $bad++; }
}
foreach ($rows['REMOVED_CLASSES'] as $r) {
    if (class_exists($r[1], false) || interface_exists($r[1], false)) { echo "still exists: class {$r[1]}\n"; $bad++; }
}
foreach ($rows['REMOVED_METHODS'] as $r) {
    list($class, $method) = explode('::', $r[1]);
    if (method_exists($class, $method)) { echo "still exists: method {$r[1]}\n"; $bad++; }
}
foreach ($rows['REMOVED_INI'] as $r) {
    if (ini_get($r[1]) !== false) { echo "still exists: ini {$r[1]}\n"; $bad++; }
}
foreach ($rows['REMOVED_CLASS_CONSTANTS'] as $r) {
    if (defined(stripslashes($r[1]))) { echo "still exists: class constant {$r[1]}\n"; $bad++; }
}
foreach ($rows['DEPRECATED_CLASS_CONSTANTS'] as $r) {
    $before = $deprecations;
    constant(stripslashes($r[1]));
    if ($deprecations === $before) { echo "no deprecation: class constant {$r[1]}\n"; $bad++; }
}
foreach ($rows['TYPED_CONSTANTS'] as $r) {
    list($class, $name) = explode('::', stripslashes($r[1]));
    $type = defined("$class::$name") ? (new ReflectionClassConstant($class, $name))->getType() : null;
    if ((string) $type !== $r[4]) { echo "type differs: constant {$r[1]}\n"; $bad++; }
}
foreach ($rows['FINAL_CLASSES'] as $r) {
    if (!class_exists($r[4], false) || !(new ReflectionClass($r[4]))->isFinal()) { echo "not final: class {$r[1]}\n"; $bad++; }
}
foreach ($rows['DEPRECATED_FUNCTIONS'] as $r) {
    if (function_exists($r[1]) && !(new ReflectionFunction($r[1]))->isDeprecated()) { echo "not deprecated: function {$r[1]}\n"; $bad++; }
}
foreach ($rows['DEPRECATED_METHODS'] as $r) {
    list($class, $method) = explode('::', $r[1]);
    if (method_exists($class, $method) && !(new ReflectionMethod($class, $method))->isDeprecated()) { echo "not deprecated: method {$r[1]}\n"; $bad++; }
}
foreach ($rows['DEPRECATED_CONSTANTS'] as $r) {
    $before = $deprecations;
    constant($r[1]);
    if ($deprecations === $before) { echo "no deprecation: constant {$r[1]}\n"; $bad++; }
}
$total = array_sum(array_map('count', $rows));
echo "checked $total symbols on PHP " . PHP_VERSION . ", $bad mismatches\n";
exit($bad ? 1 : 0);
