<?php
// Prints this PHP's overridable built-in methods as JSON; scripts/builtin-methods.mjs merges the versions.

$type = function (?ReflectionType $t): string {
    if ($t === null) {
        return '';
    }
    $names = $t instanceof ReflectionNamedType ? [$t->getName()] : array_map(fn ($n) => (string) $n, $t->getTypes());
    if ($t->allowsNull() && !in_array('null', $names, true) && !in_array('mixed', $names, true)) {
        $names[] = 'null';
    }
    return implode('|', $names);
};

$rows = [];
foreach (array_merge(get_declared_classes(), get_declared_interfaces()) as $name) {
    $class = new ReflectionClass($name);
    if (!$class->isInternal()) {
        continue;
    }
    $methods = [];
    foreach ($class->getMethods() as $method) {
        if ($method->getDeclaringClass()->getName() !== $class->getName() || $method->isPrivate() || $method->isFinal() || $method->isConstructor()) {
            continue;
        }
        $params = [];
        $variadic = false;
        foreach ($method->getParameters() as $p) {
            if ($p->isVariadic()) {
                $variadic = true;
                continue;
            }
            $params[] = [$p->isPassedByReference(), $type($p->getType())];
        }
        $methods[strtolower($method->getName())] = [
            $method->isStatic(),
            $method->getNumberOfRequiredParameters(),
            $params,
            $variadic,
            $type($method->hasTentativeReturnType() ? $method->getTentativeReturnType() : $method->getReturnType()),
            $method->hasTentativeReturnType(),
        ];
    }
    $parents = array_map('strtolower', $class->getInterfaceNames());
    if ($class->getParentClass()) {
        $parents[] = strtolower($class->getParentClass()->getName());
    }
    if ($methods || $parents) {
        $rows[strtolower($name)] = ['parents' => array_values(array_unique($parents)), 'methods' => (object) $methods];
    }
}

echo json_encode(['version' => PHP_MAJOR_VERSION . '.' . PHP_MINOR_VERSION, 'classes' => $rows]);
