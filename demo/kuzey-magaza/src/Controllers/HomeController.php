<?php
namespace Kuzey\Controllers;

use Kuzey\Support\Text;

class HomeController
{
    public function index()
    {
        $featured = array(
            array('name' => 'Bakır cezve', 'price' => 349.9),
            array('name' => 'El dokuması kilim', 'price' => 2450),
        );
        include __DIR__ . '/../../templates/home.php';
    }
}
