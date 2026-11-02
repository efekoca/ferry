<?php

class Queue
{
    private $items;

    public function __construct($size)
    {
        $this->items = new SplFixedArray($size);
    }

    public function restore($snapshot)
    {
        $items = unserialize($snapshot);
        if ($items instanceof SplFixedArray) {
            $items->__wakeup();
            $this->items = $items;
        }
    }
}
