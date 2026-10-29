<?php
namespace Kuzey\Models;

abstract class Model
{
    protected $attributes = array();

    public function __get($name)
    {
        return isset($this->attributes[$name]) ? $this->attributes[$name] : null;
    }

    public function __set($name, $value)
    {
        $this->attributes[$name] = $value;
    }

    public function fill(array $data)
    {
        foreach ($data as $key => $value) {
            $this->$key = $value;
        }
        return $this;
    }
}
