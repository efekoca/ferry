<?php
require __DIR__ . '/../bootstrap/runtime.php';

$router = new Kuzey\Http\Router();
$router->get('/', 'Kuzey\Controllers\HomeController@index');
$router->get('/urun/{slug}', 'Kuzey\Controllers\ProductController@show');
$router->post('/sepet', 'Kuzey\Controllers\CartController@add');
$router->dispatch($_SERVER['REQUEST_METHOD'], parse_url($_SERVER['REQUEST_URI'], PHP_URL_PATH));
