<?php
namespace Kuzey\Database;

class Connection
{
    private $mysqli;

    public function __construct(\mysqli $mysqli)
    {
        $this->mysqli = $mysqli;
    }

    public function keepAlive()
    {
        if (!mysqli_ping($this->mysqli)) {
            $this->mysqli->real_connect();
        }
    }

    public function killQuery($threadId)
    {
        return mysqli_kill($this->mysqli, $threadId);
    }

    public function flushTables()
    {
        return mysqli_refresh($this->mysqli, MYSQLI_REFRESH_TABLES);
    }

    public function find($table, $id, array $columns = null)
    {
        $cols = $columns ? implode(', ', $columns) : '*';
        $stmt = $this->mysqli->prepare("SELECT {$cols} FROM `{$table}` WHERE id = ?");
        $stmt->bind_param('i', $id);
        $stmt->execute();
        return $stmt->get_result()->fetch_assoc();
    }
}
