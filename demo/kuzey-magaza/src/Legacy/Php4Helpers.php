<?php

class Php4Helpers
{
    public function connect($dsn)
    {
        $db = &new LegacyDatabase($dsn);
        if (!$db->ok) {
            log_error('connection failed');
            break;
        }
        return $db;
    }

    public function close()
    {
        unset($this);
    }

    public function label($status)
    {
        switch ($status) {
            case 'paid':
                return 'Paid';
            default;
                return 'Open';
        }
    }
}
