<?php
use Kuzey\Models\Order;

class OrderTest extends PHPUnit_Framework_TestCase
{
    public function testTotal()
    {
        $order = new Order(null, array(array('price' => 10, 'qty' => 2)));
        $this->assertEquals(20, $order->total());
    }
}
