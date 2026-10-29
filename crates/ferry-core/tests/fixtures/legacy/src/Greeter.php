<?php

class Greeter
{
    private $items = array();

    public function greet(Foo $foo = null, $name = 'dünya')
    {
        while (list($k, $v) = each($this->items)) {
            echo $k;
        }
        $s = "Merhaba ${name}";
        if (strpos($s, 'x') !== false) {
            return utf8_encode($s);
        }
        return get_class();
    }
}
