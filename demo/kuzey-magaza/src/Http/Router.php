<?php
namespace Kuzey\Http;

class Router
{
    private $routes = array();

    public function get($path, $action)
    {
        $this->routes['GET'][$path] = $action;
    }

    public function post($path, $action)
    {
        $this->routes['POST'][$path] = $action;
    }

    public function dispatch($method, $uri)
    {
        foreach ($this->routes[$method] as $path => $action) {
            $pattern = '#^' . preg_replace('#\{[a-z]+\}#', '([^/]+)', $path) . '$#';
            if (preg_match($pattern, $uri, $m)) {
                list($class, $fn) = explode('@', $action);
                array_shift($m);
                return call_user_func_array(array(new $class(), $fn), $m);
            }
        }
        header('HTTP/1.1 404 Not Found');
    }
}
