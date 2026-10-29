<?php
namespace Kuzey\Controllers;

use Kuzey\Support\Text;

class CartController
{
    public function add()
    {
        $sku = Text::clean($_POST['sku']);
        if (!Text::isSku($sku)) {
            header('HTTP/1.1 422 Unprocessable Entity');
            return;
        }
        $_SESSION['cart'][$sku] = isset($_SESSION['cart'][$sku]) ? $_SESSION['cart'][$sku] + 1 : 1;
        echo json_encode(array('ok' => true, 'count' => array_sum($_SESSION['cart'])));
    }
}
