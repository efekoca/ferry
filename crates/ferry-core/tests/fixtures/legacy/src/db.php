<?php
// Eski veritabanı katmanı
$link = mysql_connect('localhost', 'root', '');
$result = mysql_query('SELECT 1', $link);
$code = 'echo 1;';
eval($code);
