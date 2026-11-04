<?php

namespace Kuzey \ Legacy;

class ReportSort
{
    public $columns = array();
    private $store;

    public function __construct($store)
    {
        $this->store = $store;
    }

    public function byTotal(array $rows)
    {
        usort($rows, function ($a, $b) {
            return $a['total'] > $b['total'];
        });
        return $rows;
    }

    public function newestFirst(array $rows)
    {
        uasort($rows, function ($a, $b) {
            return $a['created'] < $b['created'];
        });
        return $rows;
    }

    public function column(array $map)
    {
        return $this->store->$map['column'];
    }

    public function nextSeries()
    {
        $series = 'AZ';
        $series++;
        return $series;
    }

    public function extension($path)
    {
        if (substr($path, strrpos($path, '.') + 1) === false) {
            return '';
        }
        return pathinfo($path, PATHINFO_EXTENSION);
    }

    public function firstRow(array $rows)
    {
        foreach ($rows as $row) {
            if (current($rows) === $row) {
                return $row;
            }
        }
        return null;
    }

    public function audit($first, $second)
    {
        $first = trim($first);
        return func_get_args();
    }

    public function tags($raw)
    {
        $tags = false;
        $tags[] = $raw;
        return $tags;
    }

    public function connect($host)
    {
        $db = mysqli_connect($host, 'kuzey', 'kuzey');
        if (mysqli_query($db, 'SET NAMES utf8') === false) {
            return null;
        }
        return $db;
    }
}
