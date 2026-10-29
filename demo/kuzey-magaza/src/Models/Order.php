<?php
namespace Kuzey\Models;

class Order extends Model
{
    public function __construct(Customer $customer = null, array $items = null, $note = null)
    {
        $this->customer = $customer;
        $this->items = $items ?: array();
        $this->note = $note;
    }

    public function total()
    {
        $sum = (real) 0;
        foreach ($this->items as $item) {
            $sum += (double) $item['price'] * (integer) $item['qty'];
        }
        return $sum;
    }

    public function isPaid()
    {
        return (boolean) $this->paid_at;
    }

    public function checksum()
    {
        return md5((binary) serialize($this->items));
    }

    public function forget()
    {
        $this->note = (unset) $this->note;
    }

    public function type()
    {
        return get_class() . ' < ' . get_parent_class();
    }

    public function isFloat($value)
    {
        return is_real($value);
    }
}
