<?php
// Prints this PHP's built-in functions, classes, methods, constants and ini settings as JSON.
// Must run on PHP 7.0, so it avoids newer syntax.

function signature($f)
{
    $params = array();
    foreach ($f->getParameters() as $p) {
        $params[] = array($p->getName(), $p->isOptional(), $p->isVariadic(), $p->hasType() ? (string) $p->getType() : null);
    }
    return array(
        'required' => $f->getNumberOfRequiredParameters(),
        'total' => $f->getNumberOfParameters(),
        'variadic' => $f->isVariadic(),
        'deprecated' => $f->isDeprecated(),
        'params' => $params,
    );
}

$out = array(
    'version' => PHP_VERSION,
    'extensions' => array_map('strtolower', get_loaded_extensions()),
    'functions' => array(),
    'classes' => array(),
    'constants' => array(),
    'ini' => array_keys(ini_get_all(null, false)),
);

foreach (get_defined_functions()['internal'] as $name) {
    $f = new ReflectionFunction($name);
    $out['functions'][strtolower($name)] = array('ext' => strtolower((string) $f->getExtensionName())) + signature($f);
}

$types = array_merge(get_declared_classes(), get_declared_interfaces(), function_exists('get_declared_traits') ? get_declared_traits() : array());
foreach ($types as $name) {
    $c = new ReflectionClass($name);
    if (!$c->isInternal()) {
        continue;
    }
    $methods = array();
    foreach ($c->getMethods() as $m) {
        if ($m->isPrivate()) {
            continue;
        }
        $methods[strtolower($m->getName())] = array('static' => $m->isStatic()) + signature($m);
    }
    $constants = array();
    if (method_exists($c, 'getReflectionConstants')) {
        foreach ($c->getReflectionConstants() as $k) {
            $constants[$k->getName()] = method_exists($k, 'isDeprecated') ? $k->isDeprecated() : false;
        }
    } else {
        foreach (array_keys($c->getConstants()) as $k) {
            $constants[$k] = false;
        }
    }
    $kind = $c->isInterface() ? 'interface' : ($c->isTrait() ? 'trait' : (method_exists($c, 'isEnum') && $c->isEnum() ? 'enum' : 'class'));
    $out['classes'][strtolower($c->getName())] = array(
        'name' => $c->getName(),
        'ext' => strtolower((string) $c->getExtensionName()),
        'kind' => $kind,
        'methods' => $methods,
        'constants' => $constants,
    );
}

foreach (get_defined_constants(true) as $ext => $list) {
    if ($ext === 'user') {
        continue;
    }
    foreach (array_keys($list) as $name) {
        $deprecated = class_exists('ReflectionConstant') ? (new ReflectionConstant($name))->isDeprecated() : false;
        $out['constants'][$name] = array('ext' => strtolower($ext), 'deprecated' => $deprecated);
    }
}

echo json_encode($out, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), "\n";
