<?php
namespace Kuzey\Support;

class Text
{
    public static function isSku($value)
    {
        return ereg('^[A-Z]{3}-[0-9]{4}$', $value);
    }

    public static function isEmail($value)
    {
        return eregi('^[a-z0-9._%-]+@[a-z0-9.-]+\.[a-z]{2,}$', $value);
    }

    public static function slug($title)
    {
        $slug = ereg_replace('[^a-z0-9]+', '-', strtolower($title));
        return eregi_replace('(^-|-$)', '', $slug);
    }

    public static function words($text)
    {
        return split(' ', $text);
    }

    public static function tags($text)
    {
        return spliti(',', $text);
    }

    public static function caseless($pattern)
    {
        return sql_regcase($pattern);
    }

    public static function toUtf8($legacy)
    {
        return utf8_encode($legacy);
    }

    public static function toLatin1($text)
    {
        return utf8_decode($text);
    }

    public static function cyrillic($text)
    {
        return convert_cyr_string($text, 'w', 'k');
    }

    public static function hebrew($text)
    {
        return hebrevc($text);
    }

    public static function multibyteMatch($text)
    {
        return mbereg('^[ğüşıöçĞÜŞİÖÇ]+$', $text) && count(mbsplit('\s', $text)) > 0;
    }

    public static function price($amount)
    {
        setlocale(LC_MONETARY, 'tr_TR');
        return money_format('%i', $amount);
    }

    public static function clean($input)
    {
        $a = filter_var($input, FILTER_SANITIZE_STRING);
        return filter_var($a, FILTER_SANITIZE_STRIPPED);
    }

    public static function initial($name)
    {
        return $name{0};
    }

    public static function greeting($name)
    {
        return "Merhaba ${name}, siparişiniz hazırlanıyor.";
    }
}
