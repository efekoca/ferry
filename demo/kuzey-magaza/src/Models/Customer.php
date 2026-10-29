<?php
namespace Kuzey\Models;

final class Customer extends Model
{
    public function __construct(string $name, string $email)
    {
        $this->name = $name;
        $this->email = $email;
    }

    public function displayName(): string
    {
        return mb_convert_case($this->name, MB_CASE_TITLE, 'UTF-8');
    }

    public function initials(): string
    {
        $parts = preg_split('/\s+/u', trim($this->name)) ?: [];
        return implode('', array_map(fn ($p) => mb_substr($p, 0, 1), $parts));
    }
}
