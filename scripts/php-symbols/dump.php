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
    'extensions' => array_values(array_unique(array_map('strtolower', array_merge(get_loaded_extensions(), get_loaded_extensions(true))))),
    'functions' => array(),
    'classes' => array(),
    'constants' => array(),
    'ini' => array(),
);

foreach (array_merge(get_loaded_extensions(), get_loaded_extensions(true)) as $ext) {
    $settings = @ini_get_all($ext, false);
    foreach (is_array($settings) ? array_keys($settings) : array() as $name) {
        $out['ini'][$name] = strtolower($ext);
    }
}
// ini_get_all() can't look up Zend extensions such as OPcache by name.
foreach (array_keys(ini_get_all(null, false)) as $name) {
    if (!isset($out['ini'][$name])) {
        $out['ini'][$name] = strpos($name, 'opcache.') === 0 ? 'zend opcache' : 'core';
    }
}

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
    $typed = array();
    if (method_exists($c, 'getReflectionConstants')) {
        foreach ($c->getReflectionConstants() as $k) {
            $constants[$k->getName()] = method_exists($k, 'isDeprecated') ? $k->isDeprecated() : false;
            if (method_exists($k, 'hasType') && $k->hasType() && !$k->isFinal() && !$k->isPrivate()) {
                $typed[$k->getName()] = (string) $k->getType();
            }
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
        'final' => $c->isFinal(),
        'methods' => $methods,
        'constants' => $constants,
        'typed' => $typed,
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
