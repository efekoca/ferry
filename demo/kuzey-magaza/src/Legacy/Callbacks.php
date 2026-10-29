<?php
namespace Kuzey\Legacy;

class Callbacks
{
    public function run($object, $method, array $args)
    {
        if (count($args) === 1) {
            return call_user_method($method, $object, $args[0]);
        }
        return call_user_method_array($method, $object, $args);
    }

    public function sorter($field)
    {
        return create_function('$a, $b', 'return strcmp($a["' . $field . '"], $b["' . $field . '"]);');
    }

    public function pairs(array $items)
    {
        $out = array();
        reset($items);
        while (list($key, $value) = each($items)) {
            $out[] = $key . '=' . $value;
        }
        return $out;
    }

    public function template($code, array $vars)
    {
        extract($vars);
        return eval('?>' . $code);
    }

    public function plugins()
    {
        return `ls plugins`;
    }
}
