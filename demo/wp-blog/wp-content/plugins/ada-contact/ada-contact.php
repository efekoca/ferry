<?php
/*
Plugin Name: Ada İletişim
Version: 1.4.0
*/
class Ada_Contact
{
    public function __construct()
    {
        add_shortcode('ada_contact', array($this, 'render'));
        add_action('init', array($this, 'handle'));
    }

    public function handle()
    {
        if (empty($_POST['ada_contact'])) {
            return;
        }
        $name = filter_var($_POST['name'], FILTER_SANITIZE_STRING);
        $message = filter_var($_POST['message'], FILTER_SANITIZE_STRING);
        wp_mail(get_option('admin_email'), "İletişim: ${name}", $message);
    }

    public function render(array $atts = null)
    {
        ob_start();
        include __DIR__ . '/form.php';
        return ob_get_clean();
    }
}
new Ada_Contact();
