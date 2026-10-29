<?php

namespace App\Services;

use App\Models\Invoice;

final class PdfExporter
{
    public function filename(Invoice $invoice): string
    {
        return sprintf('fatura-%s-%s.pdf', $invoice->number, $invoice->issued_at->format('Y-m-d'));
    }

    public function fetchLogo(string $url): string
    {
        $ch = curl_init($url);
        curl_setopt($ch, CURLOPT_RETURNTRANSFER, true);
        $body = (string) curl_exec($ch);
        curl_close($ch);

        return $body;
    }
}
