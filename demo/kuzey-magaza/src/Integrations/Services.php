<?php
namespace Kuzey\Integrations;

class Services implements \Countable, \ArrayAccess
{
    private $items = array();

    public function connect()
    {
        $db = new \mysqli('localhost', 'kuzey', 'secret', 'shop');
        if (!$db->ping()) {
            return null;
        }
        $pdo = new \PDO('mysql:host=localhost;dbname=shop', 'kuzey', 'secret');
        $pdo->setAttribute(\PDO::MYSQL_ATTR_INIT_COMMAND, 'SET NAMES utf8mb4');
        $cached = $pdo->query('SELECT payload FROM cache')->fetchAll(\PDO::FETCH_SERIALIZE);
        return array($db, $pdo, $cached);
    }

    public function inspect($object)
    {
        $property = new \ReflectionProperty($object, 'items');
        $property->setAccessible(true);
        \ReflectionClass::export(get_class($object));
        return $property->getValue($object);
    }

    public function track($order)
    {
        $seen = new \SplObjectStorage();
        $seen->attach($order);
        return $seen;
    }

    public function headers($url)
    {
        file_get_contents($url);
        return $http_response_header;
    }

    public function count()
    {
        return count($this->items);
    }

    public function offsetExists($key)
    {
        return isset($this->items[$key]);
    }

    public function offsetGet($key)
    {
        return $this->items[$key];
    }

    public function offsetSet($key, $value)
    {
        $this->items[$key] = $value;
    }

    public function offsetUnset($key)
    {
        unset($this->items[$key]);
    }
}
