<?php

define('REPORT_VERSION', '1.4', true);

class OldReport
{
    var $rows;

    function OldReport($rows)
    {
        $this->rows = $rows;
    }

    function label($row)
    {
        return $row[status] == 'paid' ? 'Ödendi' : $row[status] == 'open' ? 'Açık' : 'Diğer';
    }

    function export($path)
    {
        $file = fopen($path, 'w');
        foreach ($this->rows as $row) {
            fputcsv($file, $row);
        }
        fclose($file);
    }

    function fromQuery($query)
    {
        parse_str($query);
        assert('is_array($this->rows)');
        return isset($page) ? (int) $page : 1;
    }

    function entities($html)
    {
        return mb_convert_encoding($html, 'HTML-ENTITIES', 'UTF-8');
    }

    function legacyServices()
    {
        libxml_disable_entity_loader(true);
        $ok = filter_var('http://example.com', FILTER_VALIDATE_URL, FILTER_FLAG_SCHEME_REQUIRED | FILTER_FLAG_HOST_REQUIRED);
        $curl = curl_init();
        curl_setopt($curl, CURLOPT_BINARYTRANSFER, true);
        $mail = imap_open('{localhost:993/imap/ssl}INBOX', 'user', 'secret');
        $spell = pspell_new('tr');
        $oracle = oci_connect('user', 'secret', 'db');
        $legacy = mssql_connect('db', 'user', 'secret');
        $sybase = sybase_connect('db', 'user', 'secret');
        return array($ok, $mail, $spell, $oracle, $legacy, $sybase);
    }
}

class Object
{
}
