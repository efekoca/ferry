<?php

class Maintenance implements Serializable
{
    public $mode;

    public function __construct($mode)
    {
        $this->mode = $mode;
        $this->startedAt = time();
    }

    public function serialize()
    {
        return serialize(array($this->mode));
    }

    public function unserialize($data)
    {
        list($this->mode) = unserialize($data);
    }

    public function summary(array $tags, $done, $total)
    {
        ini_set('session.sid_length', 48);
        session_set_save_handler('m_open', 'm_close', 'm_read', 'm_write', 'm_destroy', 'm_gc');
        $line = 'Done: ' . $done + 1 . ' of ' . $total;
        switch ($this->mode) {
            case 'full';
                return implode($tags, ', ') . $line;
            default:
                return $line;
        }
    }

    public function database($conn, $result, $stmt)
    {
        $rows = pg_numrows($result);
        $lookup = array();
        $lookup[null] = $rows;
        mysqli_execute($stmt);
        $version = PGSQL_LIBPQ_VERSION_STR;
        return array($lookup, $version, ibase_connect('db'));
    }

    public function files($path, $socket)
    {
        $zip = zip_open($path);
        $handle = gzopen($path, 'r');
        $text = gzgetss($handle, 1024);
        $flags = FILE_BINARY | FILE_TEXT;
        $dir = opendir($path);
        while (($entry = readdir()) !== false) {
            $text .= $entry;
        }
        set_socket_blocking($socket, true);
        socket_set_timeout($socket, 5);
        return array($zip, $text, $flags, $dir, $php_errormsg, $HTTP_RAW_POST_DATA);
    }

    public function legacyExtensions($ldap, $result, $image)
    {
        ldap_control_paged_result_response($ldap, $result);
        $request = xmlrpc_encode_request('ping', array());
        $packet = wddx_serialize_value($request);
        $latin = recode_string('utf8..latin1', $packet);
        $font = imagepsloadfont('font.pfb');
        $formatter = datefmt_create('tr_TR', 1, 1);
        datefmt_set_timezone_id($formatter, 'Europe/Istanbul');
        mbregex_encoding('UTF-8');
        assert_options(ASSERT_ACTIVE, 1);
        return array($latin, $font, ASSERT_QUIET_EVAL, MB_OVERLOAD_STRING);
    }

    public function constants()
    {
        return array(
            filter_input(INPUT_REQUEST, 'id', FILTER_SANITIZE_MAGIC_QUOTES),
            filter_input(INPUT_SESSION, 'user'),
            idn_to_ascii('ferry.dev', 0, INTL_IDNA_VARIANT_2003),
            mt_srand(42, MT_RAND_PHP),
            date_sunrise(time(), SUNFUNCS_RET_TIMESTAMP),
            MYSQLI_REFRESH_GRANT,
            SOAP_FUNCTIONS_ALL,
            date(DATE_RFC7231),
            MHASH_MD5,
            mb_check_encoding(),
        );
    }
}
