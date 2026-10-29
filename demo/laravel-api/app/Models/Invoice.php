<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Invoice extends Model
{
    protected $fillable = ['number', 'customer_id', 'total', 'issued_at'];

    protected $casts = ['issued_at' => 'immutable_datetime', 'total' => 'decimal:2'];

    public function customer()
    {
        return $this->belongsTo(Customer::class);
    }

    public function scopeOverdue($query, \DateTimeInterface $now = null)
    {
        return $query->where('due_at', '<', $now ?? now())->whereNull('paid_at');
    }
}
