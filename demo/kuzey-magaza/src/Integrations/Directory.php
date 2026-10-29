<?php
namespace Kuzey\Integrations;

class Directory
{
    public function staff($ldap, $base)
    {
        ldap_control_paged_result($ldap, 100);
        $result = ldap_search($ldap, $base, '(objectClass=person)');
        ldap_sort($ldap, $result, 'sn');
        return ldap_get_entries($ldap, $result);
    }

    public function mailingListHash($address)
    {
        return ezmlm_hash($address);
    }

    public function report($odbc)
    {
        $result = odbc_exec($odbc, 'SELECT * FROM sales');
        return odbc_result_all($result);
    }

    public function readFeed($path)
    {
        $handle = fopen($path, 'r');
        $lines = array();
        while (!feof($handle)) {
            $lines[] = fgetss($handle);
        }
        return $lines;
    }

    public function parseXml($xml, $handler)
    {
        $parser = xml_parser_create();
        xml_set_object($parser, $handler);
        xml_set_element_handler($parser, 'start', 'end');
        xml_parse($parser, $xml);
        xml_parser_free($parser);
    }
}
