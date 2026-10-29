<!doctype html>
<html lang="tr">
<head><meta charset="utf-8"><title>Kuzey Mağaza</title></head>
<body>
<h1>Öne çıkanlar</h1>
<ul>
<?php foreach ($featured as $product): ?>
    <li><?= htmlspecialchars($product['name']) ?> · <?= Text::price($product['price']) ?></li>
<?php endforeach; ?>
</ul>
</body>
</html>
