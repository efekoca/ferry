<?php

class Extensions
{
    public function serverInfo(mysqli $db)
    {
        $flags = MYSQLI_SERVER_QUERY_WAS_SLOW | MYSQLI_SERVER_PS_OUT_PARAMS;
        $missing = array(MYSQLI_NO_DATA, MYSQLI_DATA_TRUNCATED);
        $result = $db->store_result(MYSQLI_STORE_RESULT_COPY_DATA);
        return array('mariadb' => MYSQLI_IS_MARIADB, 'flags' => $flags, 'missing' => $missing, 'result' => $result);
    }

    public function price($amount)
    {
        $formatter = new NumberFormatter('tr_TR', NumberFormatter::CURRENCY);
        $parsed = $formatter->parse($amount, NumberFormatter::TYPE_CURRENCY);
        return $parsed === false ? U_MULTIPLE_DECIMAL_SEPERATORS : $parsed;
    }

    public function archive($path)
    {
        $zip = new ZipArchive();
        $zip->open($path);
        $zip->setCompressionName('report.csv', ZipArchive::CM_DEFLATE, ZipArchive::FL_RECOMPRESS);
        return $zip;
    }

    public function header(DateTime $date)
    {
        return $date->format(DateTimeInterface::RFC7231);
    }

    public function restore(DateTime $date, SplFileInfo $file)
    {
        $date->__wakeup();
        $file->_bad_state_ex();
    }

    public function status($pdo)
    {
        return $pdo->getAttribute(PDO::ATTR_SERVER_INFO) === Pdo\Pgsql::TRANSACTION_IDLE ? DOM_PHP_ERR : 0;
    }

    public function arguments($callback)
    {
        $types = array();
        foreach ((new ReflectionFunction($callback))->getParameters() as $param) {
            $types[] = $param->isArray() ? 'array' : ($param->isCallable() ? 'callable' : $param->getClass());
        }
        return (new ReflectionFunction($callback))->isDisabled() ? array() : $types;
    }
}
