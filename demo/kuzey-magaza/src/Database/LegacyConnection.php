<?php
namespace Kuzey\Database;

class LegacyConnection
{
    private $link;

    public function __construct($host, $user, $pass, $name)
    {
        $this->link = mysql_connect($host, $user, $pass);
        mysql_select_db($name, $this->link);
        mysql_query("SET NAMES 'utf8'", $this->link);
    }

    public function all($sql)
    {
        $result = mysql_query($sql, $this->link);
        $rows = array();
        while ($row = mysql_fetch_assoc($result)) {
            $rows[] = $row;
        }
        return $rows;
    }

    public function escape($value)
    {
        return mysql_real_escape_string($value, $this->link);
    }

    public function close()
    {
        mysql_close($this->link);
    }
}
