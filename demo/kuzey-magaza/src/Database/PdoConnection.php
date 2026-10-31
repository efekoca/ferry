<?php
namespace Kuzey\Database;

use PDO;

class PdoConnection extends PDO
{
    public function query($sql, array $params = array())
    {
        $statement = $this->prepare($sql);
        $statement->execute($params);
        return $statement;
    }
}
