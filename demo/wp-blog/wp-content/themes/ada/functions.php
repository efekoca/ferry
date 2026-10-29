<?php
function ada_setup()
{
    add_theme_support('title-tag');
    add_theme_support('post-thumbnails');
    register_nav_menus(array('primary' => __('Ana menü', 'ada')));
}
add_action('after_setup_theme', 'ada_setup');

function ada_excerpt($text)
{
    return utf8_encode(strip_tags($text));
}

function ada_date($post)
{
    return strftime('%e %B %Y', strtotime($post->post_date));
}

function ada_first_letter($title)
{
    return $title{0};
}
