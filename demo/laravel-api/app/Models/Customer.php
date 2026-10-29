<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;
use Illuminate\Database\Eloquent\Relations\HasMany;

class Customer extends Model
{
    protected $fillable = ['name', 'email', 'tax_number'];

    public function invoices(): HasMany
    {
        return $this->hasMany(Invoice::class);
    }
}
