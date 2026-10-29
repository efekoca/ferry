<?php
/*
Plugin Name: Ada Galeri
Version: 2.0.1
*/
function ada_gallery_thumb($file)
{
    $image = imagecreatefromjpeg($file);
    $thumb = imagescale($image, 320);
    imagejpeg($thumb, $file . '-320.jpg');
    imagedestroy($image);
    imagedestroy($thumb);
}

function ada_gallery_meta($file)
{
    $info = finfo_open(FILEINFO_MIME_TYPE);
    $mime = finfo_file($info, $file);
    finfo_close($info);
    return $mime;
}
