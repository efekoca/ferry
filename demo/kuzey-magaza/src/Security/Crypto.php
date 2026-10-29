<?php
namespace Kuzey\Security;

class Crypto
{
    private $key;

    public function __construct($key)
    {
        $this->key = $key;
    }

    public function encrypt($plain)
    {
        $iv = mcrypt_create_iv(mcrypt_get_iv_size(MCRYPT_RIJNDAEL_128, MCRYPT_MODE_CBC), MCRYPT_DEV_URANDOM);
        return base64_encode($iv . mcrypt_encrypt(MCRYPT_RIJNDAEL_128, $this->key, $plain, MCRYPT_MODE_CBC, $iv));
    }

    public function decrypt($payload)
    {
        $raw = base64_decode($payload);
        $iv = substr($raw, 0, 16);
        return rtrim(mcrypt_decrypt(MCRYPT_RIJNDAEL_128, $this->key, substr($raw, 16), MCRYPT_MODE_CBC, $iv), "\0");
    }

    public function legacyHash($value)
    {
        return bin2hex(mhash(MHASH_SHA256, $value));
    }

    public function token()
    {
        return md5(lcg_value() . gmp_strval(gmp_random(4)));
    }
}
