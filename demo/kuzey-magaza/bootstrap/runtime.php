<?php
error_reporting(E_ALL | E_STRICT);
set_magic_quotes_runtime(0);
magic_quotes_runtime(0);

if (get_magic_quotes_gpc()) {
    $_POST = array_map('stripslashes', $_POST);
}
if (get_magic_quotes_runtime()) {
    set_include_path(get_include_path() . PATH_SEPARATOR . __DIR__ . '/../lib');
}
restore_include_path();

function __autoload($class)
{
    $file = __DIR__ . '/../src/' . str_replace(array('Kuzey\\', '\\'), array('', '/'), $class) . '.php';
    if (is_file($file)) {
        require $file;
    }
}

function fail_hard($message)
{
    trigger_error($message, E_USER_ERROR);
}

$settings = parse_ini_file(__DIR__ . '/../config/app.ini', true);
foreach ($settings as $section => $values) {
    $$section = $values;
}
