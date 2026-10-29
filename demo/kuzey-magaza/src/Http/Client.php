<?php
namespace Kuzey\Http;

class Client
{
    public function get($url)
    {
        $share = curl_share_init();
        $ch = curl_init($url);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        curl_setopt($ch, CURLOPT_SHARE, $share);
        $body = curl_exec($ch);
        curl_close($ch);
        curl_share_close($share);
        return $body;
    }

    public function ping($host)
    {
        return `ping -c 1 {$host}`;
    }
}
