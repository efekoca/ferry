<?php
namespace Kuzey\Dates;

class Calendar
{
    public function human($timestamp)
    {
        setlocale(LC_TIME, 'tr_TR.UTF-8');
        return strftime('%e %B %Y, %A', $timestamp);
    }

    public function utc($timestamp)
    {
        return gmstrftime('%Y-%m-%dT%H:%M:%SZ', $timestamp);
    }

    public function parse($value)
    {
        return strptime($value, '%d.%m.%Y');
    }

    public function storeHours($lat, $lng)
    {
        return array(
            'open' => date_sunrise(time(), SUNFUNCS_RET_STRING, $lat, $lng),
            'close' => date_sunset(time(), SUNFUNCS_RET_STRING, $lat, $lng),
        );
    }
}
