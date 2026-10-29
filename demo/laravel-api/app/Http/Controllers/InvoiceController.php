<?php

namespace App\Http\Controllers;

use App\Models\Invoice;
use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;

class InvoiceController extends Controller
{
    public function index(Request $request): JsonResponse
    {
        $invoices = Invoice::query()
            ->with('customer')
            ->when($request->boolean('overdue'), fn ($q) => $q->overdue())
            ->latest('issued_at')
            ->paginate(25);

        return response()->json($invoices);
    }

    public function show(Invoice $invoice): JsonResponse
    {
        return response()->json($invoice->load('customer'));
    }
}
