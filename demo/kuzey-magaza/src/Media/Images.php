<?php
namespace Kuzey\Media;

class Images
{
    public function thumbnail($path, $width)
    {
        $exif = read_exif_data($path);
        $image = imagecreatefromjpeg($path);
        $thumb = imagescale($image, $width);
        imagejpeg($thumb, $path . '.thumb.jpg');
        imagedestroy($image);
        imagedestroy($thumb);
        return $exif;
    }

    public function toWbmp($image, $png, $jpeg)
    {
        image2wbmp($image, 'out.wbmp');
        png2wbmp($png, 'png.wbmp', 32, 32, 7);
        jpeg2wbmp($jpeg, 'jpeg.wbmp', 32, 32, 7);
    }

    public function mime($path)
    {
        $info = finfo_open(FILEINFO_MIME_TYPE);
        $mime = finfo_file($info, $path);
        finfo_close($info);
        return $mime;
    }
}
