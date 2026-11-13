import type { Lang } from "./index";

/** Step-by-step help for a finding Ferry can't fix on its own, with a short example. Code in the
 * text sits between backticks. */
export interface Guide {
  steps: Record<Lang, string[]>;
  before?: string;
  after?: string;
}

/** A package that keeps the old functions working without changing the code. */
export interface CompatPackage {
  name: string;
  command: string;
  license: string;
  php: string;
  limits: Record<Lang, string[]>;
  alternative: Record<Lang, string>;
}

export const PACKAGES: Record<string, CompatPackage> = {
  "removed.mcrypt": {
    name: "phpseclib/mcrypt_compat",
    command: "composer require phpseclib/mcrypt_compat",
    license: "MIT",
    php: ">=5.6.1",
    limits: {
      tr: [
        "Rijndael (AES), DES, Triple DES, Blowfish, RC2 ve arcfour'u destekler; CAST, GOST, LOKI97, SAFER+, Serpent, XTEA gibi algoritmaları desteklemez.",
        "Varsayılan olarak PHP 7.1'deki mcrypt'i taklit eder. PHP 5.6'dan eski sürümlerin yazdığı veri için autoloader'dan önce `PHPSECLIB_MCRYPT_TARGET_VERSION` tanımlayın.",
        "Saf PHP olduğu için eklentiden yavaştır.",
      ],
      en: [
        "Supports Rijndael (AES), DES, Triple DES, Blowfish, RC2 and arcfour; not CAST, GOST, LOKI97, SAFER+, Serpent, XTEA and the like.",
        "Emulates PHP 7.1's mcrypt by default. For data written by PHP before 5.6, define `PHPSECLIB_MCRYPT_TARGET_VERSION` before the autoloader.",
        "Plain PHP, so slower than the extension.",
      ],
    },
    alternative: {
      tr: "Uzun vadede veriyi openssl ile yeniden şifreleyin. 16 baytlık anahtarla Rijndael-128 CBC verisi `openssl_decrypt($data, 'aes-128-cbc', $key, OPENSSL_RAW_DATA | OPENSSL_ZERO_PADDING, $iv)` ile okunur; sondaki sıfır baytları `rtrim()` ile atın. 24 ve 32 baytlık anahtarlar için `aes-192-cbc` ve `aes-256-cbc` kullanın.",
      en: "In the long run, re-encrypt the data with openssl. Rijndael-128 CBC data with a 16-byte key reads back with `openssl_decrypt($data, 'aes-128-cbc', $key, OPENSSL_RAW_DATA | OPENSSL_ZERO_PADDING, $iv)`; strip the trailing zero bytes with `rtrim()`. Use `aes-192-cbc` and `aes-256-cbc` for 24- and 32-byte keys.",
    },
  },
  "removed.imap": {
    name: "fain182/ext-imap-polyfill",
    command: "composer require fain182/ext-imap-polyfill",
    license: "MIT",
    php: "^8.1",
    limits: {
      tr: [
        "Genç bir proje: ilk sürümü 2026'da çıktı ve henüz 1.0 değil.",
        "Yalnızca LOGIN ile giriş yapar; `/secure`, `OP_SECURE`, `/authuser=` ve NNTP desteklenmez.",
        "imap eklentisi yüklüyse hiçbir şey yapmaz, yani eklentiyle birlikte durabilir.",
      ],
      en: [
        "A young project: its first release came out in 2026 and it isn't 1.0 yet.",
        "Only LOGIN authentication; `/secure`, `OP_SECURE`, `/authuser=` and NNTP aren't supported.",
        "Does nothing when the imap extension is loaded, so it can stay next to it.",
      ],
    },
    alternative: {
      tr: "Eklenti kurabildiğiniz sunucularda PECL imap eski davranışı birebir korur (PHP 8.3 ve sonrası).",
      en: "Where you can install extensions, PECL imap keeps the old behavior exactly (PHP 8.3 and later).",
    },
  },
  "removed.xmlrpc": {
    name: "phpxmlrpc/polyfill-xmlrpc",
    command: "composer require phpxmlrpc/polyfill-xmlrpc",
    license: "BSD-3-Clause",
    php: "^5.4 || ^7.0 || ^8.0",
    limits: {
      tr: [
        "Ürettiği XML aynı yapıdadır ama boşlukları farklıdır; karşı sunucunun kabul ettiğini kontrol edin.",
        "Yalnızca UTF-8 ve ISO-8859-1 kodlamalarını destekler.",
        "`xmlrpc_encode_request()`'in seçenekleri kısmen desteklenir; `xmlrpc_parse_method_descriptions()` ve `xmlrpc_server_register_introspection_callback()` bir şey yapmaz.",
      ],
      en: [
        "The XML it writes has the same structure but different whitespace; check that the other server accepts it.",
        "Only the UTF-8 and ISO-8859-1 encodings.",
        "`xmlrpc_encode_request()`'s options are partly supported; `xmlrpc_parse_method_descriptions()` and `xmlrpc_server_register_introspection_callback()` do nothing.",
      ],
    },
    alternative: {
      tr: "PECL'deki xmlrpc 2021'den kalma bir beta ve bakımı yapılmıyor.",
      en: "PECL's xmlrpc is a 2021 beta and isn't maintained.",
    },
  },
  "removed.oci8": {
    name: "oci8/oci8",
    command: "pie install oci8/oci8",
    license: "PHP-3.01",
    php: ">=8.2",
    limits: {
      tr: [
        "Composer paketi değil, PHP eklentisi: her sunucuya ve Docker kalıbına ayrıca kurulmalı.",
        "Sunucuda Oracle Instant Client kütüphaneleri gerekir.",
        "Kod olduğu gibi kalır; oci_ öneki olmayan eski takma adları Ferry ayrıca düzeltir.",
      ],
      en: [
        "A PHP extension, not a Composer package: it has to be installed on every server and in the Docker image.",
        "Needs the Oracle Instant Client libraries on the server.",
        "The code stays as it is; Ferry fixes the old aliases without the oci_ prefix separately.",
      ],
    },
    alternative: {
      tr: "PIE yoksa `pecl install oci8` aynı eklentiyi kurar. `PDO_OCI`'ye geçmek çağrıları yeniden yazmayı gerektirir.",
      en: "Without PIE, `pecl install oci8` installs the same extension. Moving to `PDO_OCI` means rewriting the calls.",
    },
  },
};

const sun = (event: "sunrise" | "sunset", fn: string): Guide => {
  const twilight = event === "sunrise" ? "begin" : "end";
  return {
    steps: {
      tr: [
        `\`${fn}($ts, SUNFUNCS_RET_TIMESTAMP, $lat, $lng)\` yerine \`date_sun_info($ts, $lat, $lng)['${event}']\` yazın. Varsayılan zenit (90°50') \`${event}\` anahtarına denk gelir; 96°, 102° ve 108° için \`civil_twilight_${twilight}\`, \`nautical_twilight_${twilight}\` ve \`astronomical_twilight_${twilight}\` kullanın.`,
        "`SUNFUNCS_RET_STRING` yerine `date('H:i', …)` yazın. Eski metin saat diliminin 1970'teki farkını kullanıyordu; yaz saatinde ya da farkı sonradan değişen bölgelerde bir saat kayıktı. `date()` doğru yerel saati verir.",
        "Enlem ve boylamı vermiyorsanız eskiden `date.default_latitude` ve `date.default_longitude` ayarlarından geliyordu; `(float) ini_get(...)` ile açıkça verin.",
        "Güneşin hiç doğmadığı ya da batmadığı günlerde eski fonksiyon false döndürüyordu, `date_sun_info()` ise true ya da false döndürür. Sonucu `is_int()` ile denetleyin.",
        "PHP 8'de iki fonksiyonun sonucu bir iki dakika farklı olabilir.",
      ],
      en: [
        `Replace \`${fn}($ts, SUNFUNCS_RET_TIMESTAMP, $lat, $lng)\` with \`date_sun_info($ts, $lat, $lng)['${event}']\`. The default zenith (90°50') matches the \`${event}\` key; for 96°, 102° and 108° use \`civil_twilight_${twilight}\`, \`nautical_twilight_${twilight}\` and \`astronomical_twilight_${twilight}\`.`,
        "Replace `SUNFUNCS_RET_STRING` with `date('H:i', …)`. The old string used the time zone's offset from 1970, so it was an hour off in summer time or where the offset changed later. `date()` gives the correct local time.",
        "If you don't pass latitude and longitude, they used to come from `date.default_latitude` and `date.default_longitude`; pass `(float) ini_get(...)` explicitly.",
        "On days when the sun never rises or sets, the old function returned false, while `date_sun_info()` returns true or false. Check the result with `is_int()`.",
        "On PHP 8 the two functions can differ by a minute or two.",
      ],
    },
    before: `$time = ${fn}(time(), SUNFUNCS_RET_STRING, $lat, $lng);`,
    after: `$time = date('H:i', date_sun_info(time(), $lat, $lng)['${event}']);`,
  };
};

const PORT_TO_PDO: Record<Lang, string> = {
  tr: "PDO PHP 8.0'dan beri hatada varsayılan olarak PDOException fırlatır; false denetimlerini buna göre try/catch'e çevirin.",
  en: "PDO throws PDOException on errors by default since PHP 8.0; turn the old false checks into try/catch.",
};

const LAST_LINK: Record<Lang, string> = {
  tr: "Bağlantı verilmeyen çağrılar son açılan bağlantıyı kullanıyordu; bağlantı nesnesini her çağrıya açıkça verin.",
  en: "Calls without a link used the last connection opened; pass the connection object to every call.",
};

export const GUIDES: Record<string, Guide> = {
  "removed.mysql": {
    steps: {
      tr: [
        "Bir uyumluluk katmanına güvenmeyin: bilinen `mysql_*` paketleri terk edilmiş ya da yıllardır güncellenmiyor. Kodu dosya dosya mysqli'ye taşıyın.",
        "Bağlantıyı sona alan fonksiyonlarda argüman sırası değişir: `mysql_query($sql, $link)` → `mysqli_query($link, $sql)`. `mysql_select_db`, `mysql_real_escape_string`, `mysql_error`, `mysql_errno`, `mysql_insert_id` ve `mysql_affected_rows` da bağlantıyı ilk argüman olarak alır.",
        "Sonuç fonksiyonları aynı adla `mysqli_` önekine geçer (`mysql_fetch_assoc` → `mysqli_fetch_assoc`); `MYSQL_ASSOC` gibi sabitler `MYSQLI_ASSOC` olur.",
        LAST_LINK.tr,
        "`mysql_connect('host:3306')` biçimindeki port ya da soket `mysqli_connect()`'in ayrı argümanlarına bölünmeli. `mysql_result()`'un karşılığı yok: `mysqli_data_seek()` ve `mysqli_fetch_row()` kullanın.",
        "mysqli PHP 8.1'den beri hatada istisna fırlatır; eski false denetimlerinin çalışması için bağlanmadan önce `mysqli_report(MYSQLI_REPORT_OFF)` yazın.",
      ],
      en: [
        "Don't rely on a compatibility layer: the known `mysql_*` packages are abandoned or haven't been updated in years. Move the code to mysqli file by file.",
        "Functions that took the link last change their argument order: `mysql_query($sql, $link)` → `mysqli_query($link, $sql)`. `mysql_select_db`, `mysql_real_escape_string`, `mysql_error`, `mysql_errno`, `mysql_insert_id` and `mysql_affected_rows` also take the link first.",
        "The result functions keep their names with the `mysqli_` prefix (`mysql_fetch_assoc` → `mysqli_fetch_assoc`), and constants such as `MYSQL_ASSOC` become `MYSQLI_ASSOC`.",
        LAST_LINK.en,
        "A port or socket in `mysql_connect('host:3306')` has to move into `mysqli_connect()`'s own arguments. `mysql_result()` has no equivalent: use `mysqli_data_seek()` and `mysqli_fetch_row()`.",
        "mysqli throws on errors since PHP 8.1; to keep the old false checks working, call `mysqli_report(MYSQLI_REPORT_OFF)` before connecting.",
      ],
    },
    before: "$link = mysql_connect($host, $user, $pass);\nmysql_select_db($name, $link);\n$result = mysql_query($sql, $link);\n$row = mysql_fetch_assoc($result);",
    after: "$link = mysqli_connect($host, $user, $pass);\nmysqli_select_db($link, $name);\n$result = mysqli_query($link, $sql);\n$row = mysqli_fetch_assoc($result);",
  },
  "removed.mssql": {
    steps: {
      tr: [
        "Bakımı süren bir `mssql_*` uyumluluk paketi yok; çağrıları PDO'ya taşıyın.",
        "Sunucuda hangi sürücü varsa onu seçin: Microsoft'un `pdo_sqlsrv`'si (`'sqlsrv:Server=host;Database=db'`) ya da FreeTDS ile `pdo_dblib` (`'dblib:host=host;dbname=db'`).",
        "`mssql_connect` → `new PDO(DSN, $user, $pass)`; `mssql_select_db` → DSN'deki veritabanı adı; `mssql_query` → `$pdo->query()`; `mssql_fetch_assoc/array/row` → `$stmt->fetch(PDO::FETCH_ASSOC/BOTH/NUM)`; `mssql_close` → `$pdo = null`.",
        "`mssql_num_rows` için satırları `fetchAll()` ile alıp sayın; `rowCount()` SELECT'te güvenilir değil.",
        LAST_LINK.tr,
        PORT_TO_PDO.tr,
      ],
      en: [
        "There is no maintained `mssql_*` compatibility package; move the calls to PDO.",
        "Pick the driver the server has: Microsoft's `pdo_sqlsrv` (`'sqlsrv:Server=host;Database=db'`) or `pdo_dblib` with FreeTDS (`'dblib:host=host;dbname=db'`).",
        "`mssql_connect` → `new PDO(DSN, $user, $pass)`; `mssql_select_db` → the database name in the DSN; `mssql_query` → `$pdo->query()`; `mssql_fetch_assoc/array/row` → `$stmt->fetch(PDO::FETCH_ASSOC/BOTH/NUM)`; `mssql_close` → `$pdo = null`.",
        "For `mssql_num_rows`, fetch the rows with `fetchAll()` and count them; `rowCount()` isn't reliable for SELECT.",
        LAST_LINK.en,
        PORT_TO_PDO.en,
      ],
    },
    before: "$db = mssql_connect('db', 'user', 'secret');",
    after: "$db = new PDO('sqlsrv:Server=db', 'user', 'secret');",
  },
  "removed.sybase": {
    steps: {
      tr: [
        "`sybase_*` için uyumluluk paketi yok; FreeTDS ile `pdo_dblib`'e taşıyın: `new PDO('dblib:host=HOST;dbname=DB', $user, $pass)`.",
        "`sybase_query` → `$pdo->query()`; `sybase_fetch_assoc/array/row` → `fetch(PDO::FETCH_ASSOC/BOTH/NUM)`; `sybase_select_db` → DSN'deki veritabanı adı; `sybase_close` → `$pdo = null`.",
        LAST_LINK.tr,
        PORT_TO_PDO.tr,
      ],
      en: [
        "There is no compatibility package for `sybase_*`; move to `pdo_dblib` with FreeTDS: `new PDO('dblib:host=HOST;dbname=DB', $user, $pass)`.",
        "`sybase_query` → `$pdo->query()`; `sybase_fetch_assoc/array/row` → `fetch(PDO::FETCH_ASSOC/BOTH/NUM)`; `sybase_select_db` → the database name in the DSN; `sybase_close` → `$pdo = null`.",
        LAST_LINK.en,
        PORT_TO_PDO.en,
      ],
    },
    before: "$db = sybase_connect('db', 'user', 'secret');",
    after: "$db = new PDO('dblib:host=db', 'user', 'secret');",
  },
  "removed.interbase": {
    steps: {
      tr: [
        "Önce karar verin: `ibase_*` çağrılarını korumak mı, PDO'ya taşımak mı? PECL'de ya da Packagist'te interbase paketi yok.",
        "Kodu korumak için Firebird projesinin sürdürdüğü FirebirdSQL/php-firebird eklentisini hedef PHP için derleyin ve kullandığınız `ibase_*` fonksiyonlarının o derlemede olduğunu doğrulayın.",
        "Taşımak için: `ibase_connect($db, $user, $pass)` → `new PDO('firebird:dbname=' . $db, $user, $pass)`; `ibase_query` → `query()` ya da `prepare()` ile `execute()`; `ibase_fetch_assoc/row` → `fetch(PDO::FETCH_ASSOC/NUM)`; `ibase_trans/commit/rollback` → `beginTransaction()`/`commit()`/`rollBack()`.",
        PORT_TO_PDO.tr,
      ],
      en: [
        "Decide first: keep the `ibase_*` calls or move to PDO? There is no interbase package on PECL or Packagist.",
        "To keep the code, build the FirebirdSQL/php-firebird extension that the Firebird project maintains for the target PHP, and check that the `ibase_*` functions you use are in that build.",
        "To move: `ibase_connect($db, $user, $pass)` → `new PDO('firebird:dbname=' . $db, $user, $pass)`; `ibase_query` → `query()`, or `prepare()` and `execute()`; `ibase_fetch_assoc/row` → `fetch(PDO::FETCH_ASSOC/NUM)`; `ibase_trans/commit/rollback` → `beginTransaction()`/`commit()`/`rollBack()`.",
        PORT_TO_PDO.en,
      ],
    },
    before: "$db = ibase_connect('db');",
    after: "$db = new PDO('firebird:dbname=db');",
  },
  "removed.pspell": {
    steps: {
      tr: [
        "Kodu değiştirmeden çalıştırmak için PECL pspell'i kurun (1.0.1, PHP 8.1 ve sonrası); aynı fonksiyonları ve aspell sözlüklerini kullanır. Composer ya da PIE paketi yok.",
        "Ya da PHP ile gelen enchant eklentisine geçin: `pspell_new($dil)` → `enchant_broker_request_dict(enchant_broker_init(), $dil)`; `pspell_check` → `enchant_dict_check`; `pspell_suggest` → `enchant_dict_suggest`; `pspell_add_to_session` → `enchant_dict_add_to_session`; `pspell_add_to_personal` → `enchant_dict_add`.",
        "`pspell_config_*` ve `pspell_save_wordlist`'in karşılığı yok.",
        "enchant'ın önerileri seçtiği arka uca (çoğunlukla hunspell) göre aspell'inkinden farklı olabilir; dilin sözlüğünün sunucuda olduğunu kontrol edin.",
      ],
      en: [
        "To run the code unchanged, install PECL pspell (1.0.1, PHP 8.1 and later); it keeps the same functions and aspell dictionaries. There is no Composer or PIE package.",
        "Or move to the bundled enchant extension: `pspell_new($lang)` → `enchant_broker_request_dict(enchant_broker_init(), $lang)`; `pspell_check` → `enchant_dict_check`; `pspell_suggest` → `enchant_dict_suggest`; `pspell_add_to_session` → `enchant_dict_add_to_session`; `pspell_add_to_personal` → `enchant_dict_add`.",
        "`pspell_config_*` and `pspell_save_wordlist` have no equivalent.",
        "enchant's suggestions depend on the backend it picks (often hunspell), so they can differ from aspell's; check that the language's dictionary is on the server.",
      ],
    },
    before: "$spell = pspell_new('tr');\n$ok = pspell_check($spell, $word);",
    after: "$spell = enchant_broker_request_dict(enchant_broker_init(), 'tr');\n$ok = enchant_dict_check($spell, $word);",
  },
  "removed.wddx": {
    steps: {
      tr: [
        "Her paketin iki ucunu bulun: `wddx_serialize_value/vars` ya da `wddx_packet_*` ile nerede üretildiğini ve kimin okuduğunu (bu kod mu, başka bir sistem mi).",
        "Veriyi yalnızca bu proje okuyorsa iki ucu birlikte değiştirin: `wddx_serialize_value($v)` → `json_encode($v)`, `wddx_deserialize($s)` → `json_decode($s, true)`.",
        "Veritabanında, önbellekte ya da dosyalarda saklanan WDDX paketlerini, eski biçim hâlâ okunabilirken bir kez dönüştürün.",
        "Başka bir sistem WDDX XML'i bekliyorsa biçimi XMLWriter ile kendiniz yazın: `<wddxPacket version='1.0'><header/><data>…</data></wddxPacket>`.",
      ],
      en: [
        "Find both ends of each packet: where `wddx_serialize_value/vars` or `wddx_packet_*` produce it and who reads it (this code, or another system).",
        "If only this project reads the data, change both ends together: `wddx_serialize_value($v)` → `json_encode($v)`, `wddx_deserialize($s)` → `json_decode($s, true)`.",
        "Convert WDDX packets stored in databases, caches or files once, while the old format can still be read.",
        "If another system expects WDDX XML, write the format yourself with XMLWriter: `<wddxPacket version='1.0'><header/><data>…</data></wddxPacket>`.",
      ],
    },
    before: "$packet = wddx_serialize_value($order);\n$order = wddx_deserialize($packet);",
    after: "$packet = json_encode($order);\n$order = json_decode($packet, true);",
  },
  "removed.imageps": {
    steps: {
      tr: [
        ".pfb/.pfa yazı tiplerini ve tüm `imageps*` çağrılarını bulun.",
        "GD yazıları FreeType ile çizer ve FreeType Type 1 yazı tiplerini de okur; yazı tipi dosyası çoğunlukla doğrudan `imagefttext()`'e verilebilir. Yüklenmezse FontForge gibi bir araçla OpenType'a çevirin.",
        "`imagepsloadfont($dosya)` yerine dosya yolunu tutun; `imagepstext($im, $metin, $font, $boyut, $renk, $arka, $x, $y)` → `imagefttext($im, $boyut, 0, $x, $y, $renk, $yol, $metin)`; `imagepsbbox` → `imageftbbox`; `imagepsfreefont`'u silin.",
        "`imagepsencodefont`, `imagepsextendfont` ve `imagepsslantfont`'un karşılığı yok; kodlamayı ya da dönüşümü yazı tipinin kendisine işleyin.",
        "Çıkan görüntüleri karşılaştırın: `imagefttext` boyutu punto olarak alır ve kenar yumuşatması farklıdır.",
      ],
      en: [
        "Find the .pfb/.pfa fonts and every `imageps*` call.",
        "GD draws text through FreeType, which also reads Type 1 fonts, so the font file can usually go straight to `imagefttext()`. If it doesn't load, convert it to OpenType with a tool such as FontForge.",
        "Keep the font path instead of `imagepsloadfont($file)`; `imagepstext($im, $text, $font, $size, $color, $bg, $x, $y)` → `imagefttext($im, $size, 0, $x, $y, $color, $path, $text)`; `imagepsbbox` → `imageftbbox`; delete `imagepsfreefont`.",
        "`imagepsencodefont`, `imagepsextendfont` and `imagepsslantfont` have no equivalent; build the encoding or transformation into the font itself.",
        "Compare the rendered images: `imagefttext` takes the size in points and antialiases differently.",
      ],
    },
    before: "$font = imagepsloadfont('font.pfb');\nimagepstext($image, 'Kuzey', $font, 24, $black, $white, 10, 40);",
    after: "$font = __DIR__ . '/font.pfb';\nimagefttext($image, 24, 0, 10, 40, $black, $font, 'Kuzey');",
  },
  "removed.ldap_sort": {
    steps: {
      tr: [
        "`ldap_sort()` çağrısını silin ve kayıtları `ldap_get_entries()`'ten sonra sıralayın.",
        "`ldap_get_entries()` öznitelik adlarını küçük harfe çevirir; sıralarken adı küçük harfle yazın. Eski sıralama değerleri `strcmp()` ile, büyük-küçük harfe duyarlı karşılaştırıyordu.",
        "Dizideki `'count'` anahtarını sıralamadan önce çıkarın, sonra geri koyun.",
        "Sunucunun sıralaması yeterliyse `ldap_search()`'e `LDAP_CONTROL_SORTREQUEST` denetimini verin; sunucunun desteklemesi gerekir ve sıra farklı olabilir.",
      ],
      en: [
        "Delete the `ldap_sort()` call and sort the entries after `ldap_get_entries()` instead.",
        "`ldap_get_entries()` lower-cases attribute names, so use the lower-case name when sorting. The old sort compared values with `strcmp()`, case-sensitively.",
        "Take the `'count'` key out of the array before sorting and put it back afterwards.",
        "If the server's order is enough, pass the `LDAP_CONTROL_SORTREQUEST` control to `ldap_search()`; the server has to support it and the order can differ.",
      ],
    },
    before: "ldap_sort($ldap, $result, 'sn');\n$entries = ldap_get_entries($ldap, $result);",
    after: "$entries = ldap_get_entries($ldap, $result);\n$count = $entries['count'];\nunset($entries['count']);\nusort($entries, fn ($a, $b) => strcmp($a['sn'][0] ?? '', $b['sn'][0] ?? ''));\n$entries['count'] = $count;",
  },
  "deprecated.odbc_result_all": {
    steps: {
      tr: [
        "`odbc_result_all()` kalan satırları bir HTML tablosu olarak basıyor ve satır sayısını döndürüyordu.",
        "Satırları `odbc_fetch_array()` ile alıp tabloyu kendiniz yazın; sütun adları için `odbc_field_name()` kullanabilirsiniz.",
        "Eski fonksiyon değerleri kaçırmıyordu. Ekrana basarken `htmlspecialchars()` kullanın; HTML içeren değerlerde çıktı değişir.",
        "Satır sayısını kullanan kod varsa döngüde sayıp aynı değeri döndürün.",
      ],
      en: [
        "`odbc_result_all()` printed the remaining rows as an HTML table and returned the row count.",
        "Fetch the rows with `odbc_fetch_array()` and write the table yourself; `odbc_field_name()` gives the column names.",
        "The old function didn't escape values. Use `htmlspecialchars()` when printing; the output changes for values that contain HTML.",
        "If code uses the row count, count the rows in the loop and return that.",
      ],
    },
    before: "return odbc_result_all($result);",
    after: "$count = 0;\necho '<table>';\nwhile ($row = odbc_fetch_array($result)) {\n    $count++;\n    echo '<tr><td>', implode('</td><td>', array_map('htmlspecialchars', $row)), '</td></tr>';\n}\necho '</table>';\nreturn $count;",
  },
  "deprecated.zip_procedural": {
    steps: {
      tr: [
        "`zip_*` fonksiyonlarını ZipArchive ile yazın: `zip_open($yol)` → `$zip = new \\ZipArchive(); $zip->open($yol)`. İkisi de hatada aynı tam sayı kodunu döndürür; `open()` başarıda true döndürür.",
        "`while ($e = zip_read($z))` → `for ($i = 0; $i < $zip->numFiles; $i++)`. `zip_entry_name($e)` → `$zip->getNameIndex($i)`.",
        "`zip_entry_filesize` ve `zip_entry_compressedsize` → `$zip->statIndex($i)['size']` ve `['comp_size']`. `zip_entry_compressionmethod` `'deflated'` gibi bir metin döndürüyordu; `statIndex($i)['comp_method']` tam sayıdır (8).",
        "`zip_entry_open`, `zip_entry_read` ve `zip_entry_close` yerine `$zip->getFromIndex($i)` ya da `$zip->getStream($ad)`; `zip_close($z)` → `$zip->close()`.",
      ],
      en: [
        "Write the `zip_*` calls with ZipArchive: `zip_open($path)` → `$zip = new \\ZipArchive(); $zip->open($path)`. Both return the same integer code on errors; `open()` returns true on success.",
        "`while ($e = zip_read($z))` → `for ($i = 0; $i < $zip->numFiles; $i++)`. `zip_entry_name($e)` → `$zip->getNameIndex($i)`.",
        "`zip_entry_filesize` and `zip_entry_compressedsize` → `$zip->statIndex($i)['size']` and `['comp_size']`. `zip_entry_compressionmethod` returned a string such as `'deflated'`; `statIndex($i)['comp_method']` is an integer (8).",
        "Replace `zip_entry_open`, `zip_entry_read` and `zip_entry_close` with `$zip->getFromIndex($i)` or `$zip->getStream($name)`; `zip_close($z)` → `$zip->close()`.",
      ],
    },
    before: "$z = zip_open($path);\nwhile ($entry = zip_read($z)) {\n    $names[] = zip_entry_name($entry);\n}\nzip_close($z);",
    after: "$zip = new \\ZipArchive();\n$zip->open($path);\nfor ($i = 0; $i < $zip->numFiles; $i++) {\n    $names[] = $zip->getNameIndex($i);\n}\n$zip->close();",
  },
  "deprecated.strftime": {
    steps: {
      tr: [
        "Biçim yalnızca yerel ayardan bağımsız belirteçler kullanıyorsa `date()`'e çevirin: `%Y→Y, %m→m, %d→d, %H→H, %M→i, %S→s, %F→Y-m-d, %T→H:i:s`.",
        "Ay ve gün adları (`%B`, `%A`, `%b`, `%a`) `setlocale()`'e bağlıysa IntlDateFormatter kullanın; desen: `%d→dd, %e→d, %m→MM, %b→MMM, %B→MMMM, %Y→y, %a→EEE, %A→EEEE, %H→HH, %M→mm`. Düz metni tek tırnağa alın.",
        "`%e` tek basamaklı günün önüne boşluk koyuyordu; ICU'nun `d`'si koymaz.",
        "IntlDateFormatter yerel ayarı `setlocale()`'den değil argümandan alır; `setlocale()`'e verdiğiniz yerel ayarı açıkça yazın.",
      ],
      en: [
        "If the format only uses locale-independent specifiers, switch to `date()`: `%Y→Y, %m→m, %d→d, %H→H, %M→i, %S→s, %F→Y-m-d, %T→H:i:s`.",
        "If month and day names (`%B`, `%A`, `%b`, `%a`) depend on `setlocale()`, use IntlDateFormatter with the pattern `%d→dd, %e→d, %m→MM, %b→MMM, %B→MMMM, %Y→y, %a→EEE, %A→EEEE, %H→HH, %M→mm`. Put literal text in single quotes.",
        "`%e` padded single-digit days with a space; ICU's `d` doesn't.",
        "IntlDateFormatter takes the locale as an argument, not from `setlocale()`; write the locale you passed to `setlocale()` explicitly.",
      ],
    },
    before: "setlocale(LC_TIME, 'tr_TR.UTF-8');\necho strftime('%e %B %Y, %A', $ts);",
    after: "$format = new \\IntlDateFormatter('tr_TR', \\IntlDateFormatter::NONE, \\IntlDateFormatter::NONE, null, null, 'd MMMM y, EEEE');\necho $format->format($ts);",
  },
  "deprecated.strptime": {
    steps: {
      tr: [
        "Birebir karşılığı yok: `strptime()` `tm_*` anahtarlı bir dizi döndürüyordu (`tm_year` 1900'den beri geçen yıl, `tm_mon` 0'dan başlar).",
        "Biçimi çevirin: `%d→d, %e→j, %m→m, %Y→Y, %y→y, %H→H, %M→i, %S→s, %b→M, %B→F`.",
        "`date_parse_from_format($bicim, $deger)` ya da `DateTime::createFromFormat('!' . $bicim, $deger)` ile ayrıştırın; yerel ay adları için `IntlDateFormatter::parse()` kullanın.",
        "Sonucu kullanan kodu yeni anahtarlara çevirin: `tm_mday` → `day`, `tm_mon + 1` → `month`, `tm_year + 1900` → `year`.",
      ],
      en: [
        "There is no exact equivalent: `strptime()` returned an array with `tm_*` keys (`tm_year` counts from 1900, `tm_mon` starts at 0).",
        "Translate the format: `%d→d, %e→j, %m→m, %Y→Y, %y→y, %H→H, %M→i, %S→s, %b→M, %B→F`.",
        "Parse with `date_parse_from_format($format, $value)` or `DateTime::createFromFormat('!' . $format, $value)`; for localized month names use `IntlDateFormatter::parse()`.",
        "Update the code that reads the result to the new keys: `tm_mday` → `day`, `tm_mon + 1` → `month`, `tm_year + 1900` → `year`.",
      ],
    },
    before: "$tm = strptime($value, '%d.%m.%Y');\n$year = $tm['tm_year'] + 1900;",
    after: "$date = date_parse_from_format('d.m.Y', $value);\n$year = $date['year'];",
  },
  "deprecated.date_sunrise": sun("sunrise", "date_sunrise"),
  "deprecated.date_sunset": sun("sunset", "date_sunset"),
  "deprecated.sunfuncs_ret": {
    steps: {
      tr: [
        "`SUNFUNCS_RET_*` sabitleri yalnızca `date_sunrise()` ve `date_sunset()`'in ne döndüreceğini seçer; o çağrıları `date_sun_info()`'ya çevirdiğinizde sabitler de gider.",
        "`SUNFUNCS_RET_TIMESTAMP` → `date_sun_info(...)['sunrise']` ya da `['sunset']`; `SUNFUNCS_RET_STRING` → `date('H:i', …)`; `SUNFUNCS_RET_DOUBLE` → saat cinsinden değer, örneğin `date('G', $t) + date('i', $t) / 60`.",
      ],
      en: [
        "The `SUNFUNCS_RET_*` constants only select what `date_sunrise()` and `date_sunset()` return; they go away once those calls become `date_sun_info()`.",
        "`SUNFUNCS_RET_TIMESTAMP` → `date_sun_info(...)['sunrise']` or `['sunset']`; `SUNFUNCS_RET_STRING` → `date('H:i', …)`; `SUNFUNCS_RET_DOUBLE` → the time in hours, for example `date('G', $t) + date('i', $t) / 60`.",
      ],
    },
    before: "$t = date_sunrise(time(), SUNFUNCS_RET_TIMESTAMP, $lat, $lng);",
    after: "$t = date_sun_info(time(), $lat, $lng)['sunrise'];",
  },
  "deprecated.mb_encoding": {
    steps: {
      tr: [
        "HTML-ENTITIES'e çevirmek için `mb_encode_numericentity($s, [0x80, 0x10FFFF, 0, 0x1FFFFF], 'UTF-8')` kullanın. mbstring adı olan yerlerde `&eacute;` yazıyordu, bu `&#233;` yazar: tarayıcıda aynı, baytları farklı.",
        "HTML-ENTITIES'ten çevirmek için `html_entity_decode($s, ENT_QUOTES | ENT_HTML401, 'UTF-8')` kullanın.",
        "BASE64'e çevirmek için `rtrim(chunk_split(base64_encode($s), 76, \"\\r\\n\"), \"\\r\\n\")` mbstring'in 76 sütunluk satırlarını birebir verir; yalnız `base64_encode()` 57 bayta kadar aynıdır. BASE64'ten çevirmek için `base64_decode()`.",
        "Quoted-Printable ve UUENCODE için `quoted_printable_encode()`/`decode()` ve `convert_uuencode()`/`uudecode()` var; satır kırma ve çerçeveleri farklı, çıktıyı karşılaştırın.",
        "`mb_strlen($s, 'BASE64')` gibi başka `mb_*` çağrılarında önce veriyi çözün, sonra gerçek kodlamayı verin.",
      ],
      en: [
        "To convert to HTML-ENTITIES, use `mb_encode_numericentity($s, [0x80, 0x10FFFF, 0, 0x1FFFFF], 'UTF-8')`. mbstring wrote `&eacute;` where a name exists; this writes `&#233;`: the same in a browser, different bytes.",
        "To convert from HTML-ENTITIES, use `html_entity_decode($s, ENT_QUOTES | ENT_HTML401, 'UTF-8')`.",
        "To convert to BASE64, `rtrim(chunk_split(base64_encode($s), 76, \"\\r\\n\"), \"\\r\\n\")` gives mbstring's 76-column lines exactly; plain `base64_encode()` only matches up to 57 bytes. To convert from BASE64, use `base64_decode()`.",
        "Quoted-Printable and UUENCODE have `quoted_printable_encode()`/`decode()` and `convert_uuencode()`/`uudecode()`; their line breaks and framing differ, so compare the output.",
        "In other `mb_*` calls such as `mb_strlen($s, 'BASE64')`, decode the data first and pass the real encoding.",
      ],
    },
    before: "$html = mb_convert_encoding($text, 'HTML-ENTITIES', 'UTF-8');",
    after: "$html = mb_encode_numericentity($text, [0x80, 0x10FFFF, 0, 0x1FFFFF], 'UTF-8');",
  },
  "deprecated.mt_rand_php": {
    steps: {
      tr: [
        "`MT_RAND_PHP`'nin ürettiği diziyi kullanımdan kalkmamış bir yolla üretmek mümkün değil; `Random\\Engine\\Mt19937`'deki aynı kip de kullanımdan kalktı.",
        "Tohumun ne için olduğuna bakın. Yalnızca üreteci başlatıyorsa ya da üretilen değerler hiçbir yerde karşılaştırılmıyorsa kipi silin: `mt_srand($tohum, MT_RAND_PHP)` → `mt_srand($tohum)`.",
        "Saklanan değerler, örnek veriler ya da üretilmiş kodlar bu diziye bağlıysa onları değişiklikten sonra yeniden üretin ya da değerleri veri olarak saklayın. Örneğin 42 tohumuyla `mt_rand(1, 100)` 64 yerine 43 verir.",
      ],
      en: [
        "There is no non-deprecated way to produce `MT_RAND_PHP`'s sequence; the same mode in `Random\\Engine\\Mt19937` is deprecated too.",
        "Look at what the seed is for. If it only starts the generator, or nothing compares the generated values, drop the mode: `mt_srand($seed, MT_RAND_PHP)` → `mt_srand($seed)`.",
        "If stored values, fixtures or generated codes depend on the sequence, regenerate them after the change, or keep the values as data. With seed 42, for example, `mt_rand(1, 100)` gives 43 instead of 64.",
      ],
    },
    before: "mt_srand($seed, MT_RAND_PHP);",
    after: "mt_srand($seed);",
  },
  "deprecated.ini_setting": {
    steps: {
      tr: [
        "Ferry yalnızca varsayılandan farklı değerleri bildirir; satırı silmek davranışı değiştirir, ne değiştirdiğine bakın.",
        "`session.sid_length` ve `session.sid_bits_per_character` (8.4): karşılığı yok; yeni oturum kimlikleri 32 karakter olur. Eski uzunluğu bekleyen depolama varsa kontrol edin.",
        "`session.use_trans_sid`, `session.trans_sid_tags`, `session.trans_sid_hosts` ve `session.use_only_cookies` (8.4): oturum kimliğini adreste taşıma kalkıyor; çerez kullanın. `session.referer_check`: denetimi koda taşıyın, `HTTP_REFERER`'in alan adını `parse_url()` ile karşılaştırın.",
        "`assert.*` (8.3): `zend.assertions` ve `assert.exception`'ı `php.ini`'de ayarlayın.",
        "`intl.error_level` (8.5): `intl.use_exceptions=1` yazıp `\\IntlException` yakalayın. `allow_url_include`, `mbstring.func_overload`, `track_errors`, `filter.default`, `auto_detect_line_endings`, `oci8.old_oci_close_semantics` ve `report_memleaks` için satırı silin.",
      ],
      en: [
        "Ferry only reports values that differ from the default, so removing the line changes behavior; check what it changes.",
        "`session.sid_length` and `session.sid_bits_per_character` (8.4): no replacement; new session IDs become 32 characters. Check any storage that expects the old length.",
        "`session.use_trans_sid`, `session.trans_sid_tags`, `session.trans_sid_hosts` and `session.use_only_cookies` (8.4): session IDs in URLs go away; use cookies. `session.referer_check`: move the check into code and compare the host of `HTTP_REFERER` with `parse_url()`.",
        "`assert.*` (8.3): set `zend.assertions` and `assert.exception` in `php.ini`.",
        "`intl.error_level` (8.5): set `intl.use_exceptions=1` and catch `\\IntlException`. For `allow_url_include`, `mbstring.func_overload`, `track_errors`, `filter.default`, `auto_detect_line_endings`, `oci8.old_oci_close_semantics` and `report_memleaks`, remove the line.",
      ],
    },
    before: "ini_set('session.sid_length', 48);",
    after: "// Removed: session IDs are 32 characters from now on.",
  },
  "removed.reserved_name": {
    steps: {
      tr: [
        "Hedef sürümlerde ayrılmış olmayan ve ad alanında boş bir ad seçin: `object` (7.2), `mixed` (8.0), `never` (8.1) ve skaler tip adları ayrılmıştır. Örneğin `Object` → `BaseObject`.",
        "Bildirimi ve tüm başvuruları yeniden adlandırın: new, extends, implements, instanceof, catch, tip bildirimleri, ::class, statik çağrılar ve use satırları.",
        "Adı metin olarak da arayın: `class_exists('Object')`, `['Object', 'm']` gibi çağrılabilirler, yapılandırma dosyaları ve veritabanında ya da önbellekte saklanan `O:6:\"Object\"` biçimli serileştirilmiş veri.",
        "`class_alias()` işe yaramaz: eski ad ayrılmış olduğu için tanımlanamaz.",
      ],
      en: [
        "Pick a name that isn't reserved in the target versions and is free in the namespace: `object` (7.2), `mixed` (8.0), `never` (8.1) and the scalar type names are reserved. For example `Object` → `BaseObject`.",
        "Rename the declaration and every reference: new, extends, implements, instanceof, catch, type declarations, ::class, static calls and use statements.",
        "Search for the name in strings too: `class_exists('Object')`, callables like `['Object', 'm']`, configuration files, and serialized data such as `O:6:\"Object\"` stored in databases or caches.",
        "`class_alias()` can't help: the old name is reserved, so it can't be declared.",
      ],
    },
    before: "class Object\n{\n}",
    after: "class BaseObject\n{\n}",
  },
  "behavior.foreach_array_pointer": {
    steps: {
      tr: [
        "PHP 5, değerle dönen foreach'te dizinin işaretçisini döngüden önce bir kez ilerletiyordu: `current()` her turda ikinci elemanı, `key()` 1'i veriyordu. Referansla dönen döngüde işaretçi her turda bir adım öndeydi. PHP 7'den beri foreach işaretçiye dokunmuyor.",
        "Döngüdeki elemanı kastettiyseniz foreach'e `$key =>` ekleyin ve `current()`/`key()` yerine `$value` ve `$key` kullanın.",
        "İlk elemanı kastettiyseniz döngüden önce bir kez okuyun: `$first = reset($items);`",
        "Fonksiyona parametre olarak gelen dizide PHP 5 de aynı davranıyordu; orada bir şey değişmez.",
      ],
      en: [
        "PHP 5 moved the array's pointer once before a by-value foreach: `current()` returned the second element on every pass and `key()` returned 1. In a by-reference loop the pointer stayed one step ahead. Since PHP 7 foreach doesn't touch the pointer.",
        "If you meant the element being iterated, add `$key =>` to the foreach and use `$value` and `$key` instead of `current()`/`key()`.",
        "If you meant the first element, read it once before the loop: `$first = reset($items);`",
        "For an array that came in as a function parameter, PHP 5 already behaved the same way, so nothing changes there.",
      ],
    },
    before: "foreach ($items as $item) {\n    echo key($items), '=', $item, \"\\n\";\n}",
    after: "foreach ($items as $key => $item) {\n    echo $key, '=', $item, \"\\n\";\n}",
  },
  "behavior.seeded_random_sequence": {
    steps: {
      tr: [
        "Sabit bir tohumla (`mt_srand(42)`, `srand(42)`) üretilip saklanan ya da karşılaştırılan değerleri bulun: test beklentileri, örnek veriler, üretilmiş kodlar.",
        "Bu değerler PHP 7.1'de değişti: aralıkla çağrılan `mt_rand()` başka sayı verir, `rand()` artık `mt_rand()` ile aynıdır, `shuffle()`, `str_shuffle()` ve `array_rand()` de farklı sıralar. Eski diziyi genel olarak geri getirmenin yolu yok.",
        "Saklanan değerleri hedef sürümde bir kez yeniden üretin.",
        "Diziyi bundan sonra sabit tutmak için tohumlanmış bir `Random\\Randomizer` kullanın (PHP 8.2+); `Mt19937` ile `mt_rand()`'ın yeni sonuçlarını verir.",
        "Tohumsuz ya da zamanla tohumlanan çağrılara dokunmayın; onların zaten sabit bir dizisi yoktu.",
      ],
      en: [
        "Find values made with a fixed seed (`mt_srand(42)`, `srand(42)`) that are stored or compared: test expectations, fixtures, generated codes.",
        "These values changed in PHP 7.1: `mt_rand()` with a range gives other numbers, `rand()` is now the same as `mt_rand()`, and `shuffle()`, `str_shuffle()` and `array_rand()` order differently. There is no general way to bring the old sequence back.",
        "Regenerate the stored values once on the target version.",
        "To keep the sequence stable from now on, use a seeded `Random\\Randomizer` (PHP 8.2+); with `Mt19937` it gives `mt_rand()`'s current results.",
        "Leave unseeded or time-seeded calls alone; they never had a fixed sequence.",
      ],
    },
    before: "mt_srand(42);\n$code = mt_rand(1000, 9999);",
    after: "$random = new \\Random\\Randomizer(new \\Random\\Engine\\Mt19937(42));\n$code = $random->getInt(1000, 9999);",
  },
  "review.eval": {
    steps: {
      tr: [
        "Tırnak içinde verilen kodu Ferry de tarar ve bulgularını bu satırda gösterir. Kod bir değişkenden geliyorsa kaynağını bulun. Şablon ya da önbellek dosyalarından geliyorsa o dosyaları da gözden geçirin; Ferry `.phtml` ve `.inc` gibi PHP dosyalarını tarar, Twig gibi başka biçimleri okumaz.",
        "PHP 7.0'dan beri eval içindeki sözdizimi hatası false döndürmüyor, ParseError fırlatıyor: `if (eval(…) === false)` denetimlerini `try { … } catch (\\ParseError $e) { … }` ile değiştirin.",
        "PHP 8.0'dan beri değerlendirilen koddaki bazı uyarılar Error'a dönüştü (tanımsız sabitler, yanlış argüman sayısı); şablonların geçtiği yolları hedef sürümün testlerinde çalıştırın.",
      ],
      en: [
        "Ferry scans code passed as quoted text too and shows its findings on this line. If the code comes from a variable, find its source. If it comes from template or cache files, review those too; Ferry scans PHP files such as `.phtml` and `.inc`, but not other formats such as Twig.",
        "Since PHP 7.0 a syntax error inside eval throws ParseError instead of returning false: replace `if (eval(…) === false)` checks with `try { … } catch (\\ParseError $e) { … }`.",
        "Since PHP 8.0 some warnings in the evaluated code became Error exceptions (undefined constants, wrong argument counts); run the template paths in the target version's tests.",
      ],
    },
    before: "$result = eval($code);\nif ($result === false) {\n    log_error('bad template');\n}",
    after: "try {\n    $result = eval($code);\n} catch (\\ParseError $e) {\n    log_error('bad template');\n}",
  },
  "review.extract": {
    steps: {
      tr: [
        "`extract()`'a ne geldiğine bakın: PHP 8.0'dan beri dizi olmayan değer TypeError fırlatıyor (PHP 7 uyarı verip null döndürüyordu). null gelebiliyorsa `is_array()` ile koruyun.",
        "Dizide `'this'` anahtarı varsa PHP 7.1'den beri Error fırlatır.",
        "Sonraki kodun (ya da eklenen şablonun) okuduğu değişkenleri listeleyin; null olabilen ve metin fonksiyonlarına giden değerler PHP 8.1'de kullanımdan kalkma uyarısı verir.",
        "Değişken adları belliyse `extract()`'ı açık atamalarla değiştirin; değilse yerel değişkenlerin üzerine yazmaması için `EXTR_SKIP` verin.",
      ],
      en: [
        "Check what reaches `extract()`: since PHP 8.0 a non-array throws TypeError (PHP 7 warned and returned null). Guard with `is_array()` where null can arrive.",
        "A `'this'` key in the array throws Error since PHP 7.1.",
        "List the variables the code after it (or the included template) reads; values that can be null and reach string functions raise deprecation warnings on PHP 8.1.",
        "Where the variable names are known, replace `extract()` with explicit assignments; otherwise pass `EXTR_SKIP` so it doesn't overwrite local variables.",
      ],
    },
    before: "extract($vars);\ninclude $template;",
    after: "extract(is_array($vars) ? $vars : [], EXTR_SKIP);\ninclude $template;",
  },
  "removed.assign_this": {
    steps: {
      tr: [
        "Değişkene başka bir ad verin (örneğin `$item`) ve fonksiyondaki bütün kullanımlarını değiştirin: parametre, `foreach`, `catch` ya da `list()` hedefi ve sonraki okumalar.",
        "Sınıf dışındaki bir fonksiyonda `$this` eskiden sıradan bir değişkendi; yeniden adlandırmak davranışı değiştirmez.",
        "Closure'ın `use` listesindeki `$this`'i silin: sınıf içinde oluşturulan closure nesneyi zaten `$this` olarak görür.",
        "`extract()`, `parse_str()` ya da `$$ad` ile `this` adlı değişken yazan kod da PHP 7.1'den beri Error fırlatır ve Ferry bunları işaretlemez. Diziden `'this'` anahtarını çıkarın.",
      ],
      en: [
        "Give the variable another name (for example `$item`) and change every use in the function: the parameter, the `foreach`, `catch` or `list()` target, and the reads after it.",
        "In a function outside a class, `$this` used to be an ordinary variable, so renaming it doesn't change behavior.",
        "Remove `$this` from a closure's `use` list: a closure created inside a class already sees the object as `$this`.",
        "Code that writes a variable named `this` through `extract()`, `parse_str()` or `$$name` throws Error since PHP 7.1 too, and Ferry doesn't mark it. Drop the `'this'` key from the array.",
      ],
    },
    before: "function total($this) {\n    return $this['price'] * $this['qty'];\n}",
    after: "function total($item) {\n    return $item['price'] * $item['qty'];\n}",
  },
  "removed.globals_write": {
    steps: {
      tr: [
        "Tek bir global değişkeni `$GLOBALS['ad'] = $deger;` ile değiştirin, `unset($GLOBALS['ad']);` ile silin; bunlar PHP 8.1'de de çalışır.",
        "Dizinin tamamına atama (`$GLOBALS = $dizi;`) PHP 8.0 ve öncesinde de global değişkenleri değiştirmiyordu, yalnızca bu kapsamdaki diziyi değiştiriyordu. Diziye başka bir ad verin; değişkenleri değiştirmek istediyseniz her birini `$GLOBALS['ad']` ile atayın.",
        "`array_pop()`, `sort()` gibi diziyi yerinde değiştiren fonksiyonlara ve referans parametrelerine `$GLOBALS` verilemez. Bir kopyayla çalışın (`$vars = $GLOBALS;`); değişkeni de silmek istediyseniz `unset($GLOBALS[$ad])` ekleyin.",
        "`$vars = &$GLOBALS;` gibi referanslar da derleme hatası verir; o değişken yerine doğrudan `$GLOBALS['ad']` yazın.",
        "Kopyalara da bakın: PHP 8.0 ve öncesinde `$GLOBALS`'ın kopyasına yazmak (`$vars = $GLOBALS; $vars['a'] = 2;`) global `$a`'yı da değiştiriyordu, 8.1'den beri kopya ayrı. Ferry bu kopyaları işaretlemez.",
      ],
      en: [
        "Change a single global with `$GLOBALS['name'] = $value;` and remove it with `unset($GLOBALS['name']);`; both still work on PHP 8.1.",
        "Assigning the whole array (`$GLOBALS = $array;`) didn't change the global variables on PHP 8.0 and before either, only the array in this scope. Give the array another name; if you meant to change the variables, assign each one with `$GLOBALS['name']`.",
        "`$GLOBALS` can't be passed to functions that change an array in place, such as `array_pop()` and `sort()`, or to reference parameters. Work on a copy (`$vars = $GLOBALS;`), and add `unset($GLOBALS[$name])` if you meant to remove the variable too.",
        "References such as `$vars = &$GLOBALS;` are compile errors too; write `$GLOBALS['name']` directly instead of that variable.",
        "Check copies as well: on PHP 8.0 and before, writing to a copy of `$GLOBALS` (`$vars = $GLOBALS; $vars['a'] = 2;`) also changed the global `$a`; since 8.1 the copy is separate. Ferry doesn't mark these copies.",
      ],
    },
    before: "$vars = &$GLOBALS;\n$vars['count']++;",
    after: "$GLOBALS['count']++;",
  },
  "removed.incompatible_user_override": {
    steps: {
      tr: [
        "Önerilen satır üst metodun imzasını gösterir. Alt metot üstteki her parametreyi almalı: eksikleri sona ekleyin, alt metoda eklediğiniz fazla parametrelere varsayılan değer verin (`$recalculate = null`).",
        "Parametre tipi üsttekinden dar olamaz: aynı tipi ya da daha genişini (üst sınıf, arayüz, birleşim tipi) yazın veya tipi kaldırın. Referansla geçiş (`&`) ve `static` ikisinde de aynı olmalı.",
        "Üst metot bir dönüş tipi bildiriyorsa alt metot da aynısını ya da daha darını bildirmeli: `?int` yerine `int` yazılabilir, tersi olmaz.",
        "Alt metot gerçekten başka argümanlar istiyorsa ona yeni bir ad verin ve çağıranları güncelleyin; aynı adı korumak üst sınıf üzerinden yapılan çağrıları bozar.",
      ],
      en: [
        "The suggested line shows the parent method's signature. The child has to take every parameter the parent takes: add the missing ones at the end, and give the extra parameters you added to the child a default (`$recalculate = null`).",
        "A parameter type can't be narrower than the parent's: use the same type or a wider one (a parent class, an interface, a union type), or drop the type. Passing by reference (`&`) and `static` have to match too.",
        "If the parent declares a return type, the child has to declare the same one or a narrower one: `int` can replace `?int`, not the other way round.",
        "If the child really needs other arguments, give it a new name and update its callers; keeping the name breaks calls made through the parent class.",
      ],
    },
    before: "public function fill(array $data, $recalculate)",
    after: "public function fill(array $data, $recalculate = null)",
  },
  "removed.parent_without_parent": {
    steps: {
      tr: [
        "Bu çağrı hiçbir sürümde çalışmadı: PHP 7.4 ve öncesinde satıra gelindiğinde Error (PHP 5'te ölümcül hata) veriyordu. PHP 8 ise dosyayı hiç yüklemiyor.",
        "Sınıfın bir üst sınıftan türemesi gerekiyorsa ve `extends` sonradan silindiyse onu geri ekleyin.",
        "Değilse `parent::` çağrısını silin. Çalıştığı her yol zaten hata verdiği için silmek çalışan kodu değiştirmez. Çağrı bir ifadenin içindeyse o satırın yerine ne döneceğine karar verin.",
      ],
      en: [
        "This call never worked: on PHP 7.4 and before it threw Error when the line ran (a fatal error on PHP 5), and PHP 8 doesn't load the file at all.",
        "If the class is meant to extend another class and the `extends` was removed later, add it back.",
        "Otherwise delete the `parent::` call. Every path that reached it already failed, so deleting it doesn't change working code. If the call sits inside an expression, decide what that line should produce instead.",
      ],
    },
    before: "public function __construct()\n{\n    parent::__construct();\n    $this->items = [];\n}",
    after: "public function __construct()\n{\n    $this->items = [];\n}",
  },
  "removed.preg_eval_modifier": {
    steps: {
      tr: [
        "Desendeki `e` belirtecini silin ve `preg_replace()` yerine `preg_replace_callback()` yazın.",
        "Değiştirme metni PHP koduydu: onu bir fonksiyonun gövdesine taşıyın ve sonucu `return` ile döndürün. `$1`, `\\1` ya da `${1}` yerine `$m[1]`, `$0` yerine `$m[0]` yazın.",
        "Eski kod eşleşmedeki tırnak ve ters bölü işaretlerinin önüne `\\` ekliyordu; çift tırnak içinde `it's` metni `it\\'s` olarak geliyordu. Callback eşleşmeyi olduğu gibi alır. Eski kod bu işaretleri `stripslashes()` ile siliyorduysa o çağrıyı da kaldırın.",
        "Desen dizisinde tek bir `e`'li desen bile bütün çağrının null döndürmesine yol açar. Yalnızca bazı desenlerde `e` varsa onları ayrı bir `preg_replace_callback()` çağrısına alın ve sırayı koruyun.",
      ],
      en: [
        "Remove the `e` modifier from the pattern and use `preg_replace_callback()` instead of `preg_replace()`.",
        "The replacement was PHP code: move it into a function body and `return` the result. Write `$m[1]` instead of `$1`, `\\1` or `${1}`, and `$m[0]` instead of `$0`.",
        "The old code put a `\\` before quotes and backslashes in the match, so inside double quotes `it's` arrived as `it\\'s`. The callback gets the match as it is. If the old code removed those backslashes with `stripslashes()`, drop that call too.",
        "A single `e` pattern in a pattern array makes the whole call return null. If only some patterns have `e`, move them into a separate `preg_replace_callback()` call and keep the order.",
      ],
    },
    before: "$title = preg_replace('/(\\w+)/e', 'ucfirst(\"$1\")', $title);",
    after: "$title = preg_replace_callback('/(\\w+)/', function ($m) {\n    return ucfirst($m[1]);\n}, $title);",
  },
  "removed.reserved_alias": {
    steps: {
      tr: [
        "Takma adı ayrılmış olmayan bir adla değiştirin; önerilen satır `Base` önekini kullanır (`use App\\Item as BaseObject;`).",
        "Bu dosyada eski kısa adın geçtiği her yeri yeni adla değiştirin: `new`, `extends`, `implements`, `instanceof`, `catch`, tip bildirimleri, `::class` ve statik çağrılar. Takma ad yalnızca bu dosyada geçerli, başka dosyaya dokunmanız gerekmez.",
        "`as` olmadan içe aktarılan sınıfın kendi adı ayrılmışsa (`use Lib\\String;`) sınıf da hedef sürümde tanımlanamaz. Projenizin sınıfıysa onu da yeniden adlandırın; bir paketten geliyorsa paketin adı değiştiren sürümüne geçin.",
      ],
      en: [
        "Replace the alias with a name that isn't reserved; the suggested line uses a `Base` prefix (`use App\\Item as BaseObject;`).",
        "Change every place in this file that uses the old short name: `new`, `extends`, `implements`, `instanceof`, `catch`, type declarations, `::class` and static calls. The alias only applies in this file, so other files stay as they are.",
        "If the class imported without `as` has a reserved name itself (`use Lib\\String;`), the class can't be declared on the target version either. Rename it too if it's your project's class; if it comes from a package, move to a release of the package that renamed it.",
      ],
    },
    before: "use App\\Item as Object;\n\nfunction save(Object $item) {}",
    after: "use App\\Item as BaseObject;\n\nfunction save(BaseObject $item) {}",
  },
  "removed.resource_check": {
    steps: {
      tr: [
        "Değeri oluşturduğunuz yerde başarıyı `false` ile denetleyin: bu fonksiyonlar hata olunca hâlâ false döndürür. `if (!is_resource($ch))` → `if ($ch === false)`.",
        "Değer kapatılmış ya da silinmiş olabiliyorsa (örneğin yıkıcıda) iki sürümde de çalışan biçimi kullanın: `is_resource($ch) || $ch instanceof \\CurlHandle`. Sınıf adı fonksiyona göre değişir (`GdImage`, `PgSql\\Connection`, `Socket` gibi); hedef sürümde `get_debug_type($ch)` adı verir.",
        "PHP 8'de `curl_close()` gibi bazı kapatma fonksiyonları bir şey yapmaz ve nesne kapatıldıktan sonra da geçerli görünür. Kapattıktan sonra değişkeni null yapın (`$ch = null;`) ki denetim false dönsün.",
        "`get_resource_type($h)` bu değerler için TypeError fırlatır; türü `instanceof` ya da `get_debug_type()` ile öğrenin.",
      ],
      en: [
        "Where the value is created, check for failure with `false`: these functions still return false on errors. `if (!is_resource($ch))` → `if ($ch === false)`.",
        "Where the value may have been closed or removed (for example in a destructor), use a check that works on both versions: `is_resource($ch) || $ch instanceof \\CurlHandle`. The class depends on the function (`GdImage`, `PgSql\\Connection`, `Socket` and so on); `get_debug_type($ch)` names it on the target version.",
        "On PHP 8 some close functions, such as `curl_close()`, do nothing, and the object still looks valid after closing. Set the variable to null after closing (`$ch = null;`) so the check returns false.",
        "`get_resource_type($h)` throws TypeError for these values; find the type with `instanceof` or `get_debug_type()`.",
      ],
    },
    before: "$ch = curl_init($url);\nif (!is_resource($ch)) {\n    throw new RuntimeException('curl');\n}",
    after: "$ch = curl_init($url);\nif ($ch === false) {\n    throw new RuntimeException('curl');\n}",
  },
  "removed.static_call_to_instance_method": {
    steps: {
      tr: [
        "Metot `$this` kullanmıyorsa bildirimine `static` ekleyin; çağrılar olduğu gibi kalır. Üst ya da alt sınıflarda aynı adlı statik olmayan bir metot varsa PHP sınıfı yüklerken hata verir, o zaman bir sonraki adıma geçin.",
        "Metot `$this` kullanıyorsa bu satır çalıştığında zaten her sürümde hata veriyordu. Bir nesne üzerinden çağırın: elinizdeki nesneyle `$cart->total()` ya da `(new Cart())->total()`.",
        "`['Cart', 'total']` ve `'Cart::total'` gibi çağrılabilirler PHP 8'de geçersizdir ve `is_callable()` bunlar için false döndürür. Metodu static yapın ya da nesneyle verin: `[new Cart(), 'total']`.",
      ],
      en: [
        "If the method doesn't use `$this`, add `static` to its declaration and the calls stay as they are. If a parent or child class has a non-static method with the same name, PHP fails when it loads the class; go to the next step then.",
        "If the method uses `$this`, this line already failed on every version when it ran. Call it on an object: the one you already have (`$cart->total()`) or `(new Cart())->total()`.",
        "Callables such as `['Cart', 'total']` and `'Cart::total'` are invalid on PHP 8, and `is_callable()` returns false for them. Make the method static or pass an object: `[new Cart(), 'total']`.",
      ],
    },
    before: "class Price\n{\n    public function format($amount)\n    {\n        return number_format($amount, 2);\n    }\n}\n\necho Price::format(10);",
    after: "class Price\n{\n    public static function format($amount)\n    {\n        return number_format($amount, 2);\n    }\n}\n\necho Price::format(10);",
  },
  "removed.too_few_arguments": {
    steps: {
      tr: [
        "Bildirime bakıp çağrının vermediği parametreleri bulun: varsayılan değeri olmayan her parametre verilmeli.",
        "Eski davranışı korumak için eksik argümanları `null` olarak verin: `pay(10)` → `pay(10, null)`. PHP 7.0 ve öncesinde bu parametreler de null oluyordu.",
        "Parametre her çağrıda isteğe bağlıysa bildirime varsayılan değer ekleyin (`$currency = null`). Varsayılan yalnızca sondaki parametrelere işe yarar: zorunlu bir parametreden önce gelen varsayılan yok sayılır ve PHP 8 kullanımdan kalkma uyarısı verir.",
      ],
      en: [
        "Look at the declaration and find the parameters the call leaves out: every parameter without a default has to be passed.",
        "To keep the old behavior, pass the missing arguments as `null`: `pay(10)` → `pay(10, null)`. On PHP 7.0 and before those parameters were null too.",
        "If the parameter is optional for every caller, give it a default in the declaration (`$currency = null`). Defaults only work on trailing parameters: a default before a required parameter is ignored, and PHP 8 raises a deprecation warning for it.",
      ],
    },
    before: "function pay($amount, $currency, $note = '') {}\n\npay(10);",
    after: "function pay($amount, $currency, $note = '') {}\n\npay(10, null);",
  },
  "syntax.unbalanced": {
    steps: {
      tr: [
        "Dosyayı projenin bugünkü PHP sürümüyle `php -l` ile denetleyin; PHP hatanın satırını söyler. Ferry eşleşmeyen ilk parantezi gösterir, asıl hata daha yukarıda olabilir.",
        "Dosya hiçbir sürümde derlenmediği için proje onu büyük olasılıkla hiç yüklemiyor: yarım kalmış bir kopya, eski bir yedek ya da bir birleştirmede bozulmuş bir dosya olabilir. Git geçmişinde bozulduğu değişikliği bulun.",
        "Dosya kullanılmıyorsa silin ya da taramanın dışında bırakın. Kullanılıyorsa parantezi düzeltip projeyi yeniden tarayın; Ferry dosyanın geri kalanını yanlış okumuş olabilir.",
        "`php -l` hata bulmuyorsa Ferry dosyayı yanlış okumuştur ve bu dosyadaki bulgular eksik olabilir.",
      ],
      en: [
        "Check the file with `php -l` on the PHP version the project runs today; PHP reports the line of the error. Ferry points at the first bracket it couldn't match, and the real mistake can be higher up.",
        "Since the file doesn't compile on any version, the project most likely never loads it: it may be a half-finished copy, an old backup or a file broken in a merge. Find the change that broke it in the Git history.",
        "If the file isn't used, delete it or leave it out of the scan. If it is, fix the bracket and scan the project again; Ferry may have misread the rest of the file.",
        "If `php -l` finds no error, Ferry misread the file and its findings for it may be incomplete.",
      ],
    },
    before: "if ($user) {\n    echo $user->name;\n",
    after: "if ($user) {\n    echo $user->name;\n}",
  },
  "deprecated.reflection_method": {
    steps: {
      tr: [
        "`$param->getClass()` yerine türü `getType()` ile okuyun: tür `ReflectionNamedType` ve yerleşik değilse `new ReflectionClass($type->getName())`, değilse null.",
        "`getName()` `self` ya da `parent` döndürebilir; eski `getClass()` bunları sınıfın kendisine çeviriyordu. Bunları `$param->getDeclaringClass()` ve onun `getParentClass()` sonucuyla çözün.",
        "`isArray()` ve `isCallable()`'ı Ferry kendisi çevirir. Elle yazmanız gerekirse: `$type instanceof ReflectionNamedType && $type->getName() === 'array'` (ya da `'callable'`); `?array` için de true verir.",
        "`ReflectionFunction::isDisabled()` artık her zaman false döndürür; çağrının yerine `false` yazın. Ferry bunu nesne bir değişkendeyken kendisi yapar, `(new ReflectionFunction($f))->isDisabled()` gibi ifadeleri size bırakır. PHP 8'de devre dışı bırakılan fonksiyonlar hiç tanımlanmaz, `function_exists()` onlar için false döner.",
      ],
      en: [
        "Instead of `$param->getClass()`, read the type with `getType()`: when it's a `ReflectionNamedType` and not built in, `new ReflectionClass($type->getName())`, otherwise null.",
        "`getName()` can return `self` or `parent`, which the old `getClass()` turned into the class itself. Resolve them with `$param->getDeclaringClass()` and its `getParentClass()`.",
        "Ferry converts `isArray()` and `isCallable()` itself. To write it by hand: `$type instanceof ReflectionNamedType && $type->getName() === 'array'` (or `'callable'`); it's true for `?array` too.",
        "`ReflectionFunction::isDisabled()` always returns false now; write `false` in place of the call. Ferry does this when the object is in a variable and leaves expressions such as `(new ReflectionFunction($f))->isDisabled()` to you. On PHP 8 disabled functions aren't defined at all, and `function_exists()` returns false for them.",
      ],
    },
    before: "$class = $param->getClass();",
    after: "$type = $param->getType();\n$class = $type instanceof ReflectionNamedType && !$type->isBuiltin()\n    ? new ReflectionClass($type->getName())\n    : null;",
  },
  "deprecated.xml_set_object": {
    steps: {
      tr: [
        "`xml_set_*_handler()` çağrılarına metot adı yerine nesneyle birlikte bir çağrılabilir verin (`[$handler, 'start']`), sonra `xml_set_object()` satırını silin.",
        "Eski PHP metot adlarını ayrıştırma başladığında ayarlı olan nesnede, yani en son `xml_set_object()` ile verilende çalıştırıyordu. Aynı ayrıştırıcıda birden fazla `xml_set_object()` varsa çağrılabilirlere o son nesneyi verin.",
        "Ferry bunu kendisi yapar; ayrıştırıcıda birden fazla `xml_set_object()` varsa ya da metot adlarından biri global bir fonksiyonun da adıysa (örneğin `end`) size bırakır.",
      ],
      en: [
        "Pass each `xml_set_*_handler()` call a callable with the object (`[$handler, 'start']`) instead of a method name, then delete the `xml_set_object()` line.",
        "Old PHP ran the method names on the object set when parsing started, that is the one from the last `xml_set_object()`. With several `xml_set_object()` calls on one parser, give the callables that last object.",
        "Ferry does this itself, and leaves it to you when the parser has several `xml_set_object()` calls or a method name is also a global function's name (such as `end`).",
      ],
    },
    before: "xml_set_object($parser, $handler);\nxml_set_element_handler($parser, 'start', 'end');",
    after: "xml_set_element_handler($parser, [$handler, 'start'], [$handler, 'end']);",
  },
  "deprecated.xml_handler_string": {
    steps: {
      tr: [
        "Metot adını nesneyle birlikte verin: `[$handler, 'text']`. Nesne, ayrıştırma başladığında `xml_set_object()` ile ayarlı olandır.",
        "Bir işleyiciyi kaldırmak için boş metin yerine `null` verin; PHP 8.4 boş metni de kullanımdan kalkmış sayar.",
        "Global bir fonksiyonun da adı olan metot adlarına dikkat edin (örneğin `end`): nesne ayarlıyken PHP 7.4 nesnenin metodunu çağırıyordu, PHP 8.4 ve sonrası global fonksiyonu çağırır. Bunları da `[$handler, 'end']` biçiminde yazın.",
      ],
      en: [
        "Pass the method name with its object: `[$handler, 'text']`. The object is the one `xml_set_object()` had set when parsing started.",
        "To remove a handler, pass `null` instead of an empty string; PHP 8.4 deprecates the empty string too.",
        "Watch for method names that are also global functions (such as `end`): with an object set, PHP 7.4 called the object's method, while PHP 8.4 and later call the global function. Write those as `[$handler, 'end']` too.",
      ],
    },
    before: "xml_set_character_data_handler($parser, 'text');",
    after: "xml_set_character_data_handler($parser, [$handler, 'text']);",
  },
  "deprecated.mysqli_get_client_info": {
    steps: {
      tr: [
        "`$db->get_client_info()` ve `mysqli_get_client_info($db)` yerine argümansız `mysqli_get_client_info()` yazın. İstemci kütüphanesinin sürümü bağlantıya bağlı değildir, sonuç aynıdır.",
        "Değişkenin gerçekten bir mysqli nesnesi olduğunu kontrol edin; başka bir sınıfın aynı adlı metodu ise dokunmayın.",
      ],
      en: [
        "Replace `$db->get_client_info()` and `mysqli_get_client_info($db)` with `mysqli_get_client_info()` without arguments. The client library's version doesn't depend on the connection, so the result is the same.",
        "Check that the variable really is a mysqli object; if it's another class's method of the same name, leave it alone.",
      ],
    },
    before: "$client = $db->get_client_info();",
    after: "$client = mysqli_get_client_info();",
  },
  "deprecated.mysqli_init": {
    steps: {
      tr: [
        "mysqli'den türeyen sınıfta `parent::init()` ya da `$this->init()` yerine `parent::__construct()` yazın. Argümansız kurucu bağlanmaz, `init()` gibi yalnızca nesneyi hazırlar; ardından gelen `options()` ve `real_connect()` çağrıları olduğu gibi kalır.",
        "Sınıf dışında, `init()` metodunu çağırmak yerine nesneyi `mysqli_init()` fonksiyonuyla oluşturun; kullanımdan kalkan yalnızca metot.",
      ],
      en: [
        "In a class that extends mysqli, replace `parent::init()` or `$this->init()` with `parent::__construct()`. Without arguments the constructor doesn't connect; like `init()`, it only prepares the object, and the `options()` and `real_connect()` calls after it stay as they are.",
        "Outside a class, create the object with the `mysqli_init()` function instead of calling the `init()` method; only the method is deprecated.",
      ],
    },
    before: "public function __construct()\n{\n    parent::init();\n    $this->real_connect('db', 'user', 'secret');\n}",
    after: "public function __construct()\n{\n    parent::__construct();\n    $this->real_connect('db', 'user', 'secret');\n}",
  },
  "deprecated.implicit_argument": {
    steps: {
      tr: [
        "`readdir()`, `rewinddir()` ve `closedir()`'a `opendir()`'ın döndürdüğü handle'ı verin; argümansız ya da null ile son açılan dizini kullanıyorlardı. Ferry dosyada tek `opendir()` varsa bunu kendisi yapar.",
        "Argümansız `mb_check_encoding()` yalnızca mbstring'in girdiyi çevirirken (örneğin `mb_parse_str()` içinde) bulduğu geçersiz baytları bildiriyordu; bunlar yoksa hep true döndürüyordu.",
        "İsteği denetlemek istediyseniz dizileri açıkça verin: `mb_check_encoding($_GET, 'UTF-8') && mb_check_encoding($_POST, 'UTF-8')`.",
      ],
      en: [
        "Pass `readdir()`, `rewinddir()` and `closedir()` the handle `opendir()` returned; without an argument or with null they used the last opened directory. Ferry does this itself when the file has a single `opendir()`.",
        "`mb_check_encoding()` without arguments only reported invalid bytes mbstring found while converting input (for example in `mb_parse_str()`); otherwise it always returned true.",
        "If you meant to check the request, pass the arrays: `mb_check_encoding($_GET, 'UTF-8') && mb_check_encoding($_POST, 'UTF-8')`.",
      ],
    },
    before: "$dir = opendir($path);\nwhile (($entry = readdir()) !== false) {",
    after: "$dir = opendir($path);\nwhile (($entry = readdir($dir)) !== false) {",
  },
  "removed.magic_variable": {
    steps: {
      tr: [
        "`$HTTP_RAW_POST_DATA` yerine `file_get_contents('php://input')` yazın; isteğin gövdesini verir. İkisi de `multipart/form-data` isteklerinde boştur.",
        "`$php_errormsg` yalnızca `track_errors` açıkken dolardı ve son hatanın metnini tutardı. Hata verebilen çağrıdan önce `error_clear_last();`, sonra `error_get_last()['message'] ?? null` yazın.",
        "`track_errors` kapalıysa (varsayılan) `$php_errormsg` hiç tanımlanmazdı; o zaman yerine `null` yazın. Ferry bu ikisini seçenek olarak sunar. `$HTTP_RAW_POST_DATA` ise yalnızca global kapsamda vardı; onu `global` ile almayan fonksiyonlarda Ferry yerine kendisi `null` yazar.",
      ],
      en: [
        "Replace `$HTTP_RAW_POST_DATA` with `file_get_contents('php://input')`, which gives the request body. Both are empty for `multipart/form-data` requests.",
        "`$php_errormsg` was only set with `track_errors` on and held the text of the last error. Write `error_clear_last();` before the call that can fail and `error_get_last()['message'] ?? null` after it.",
        "With `track_errors` off (the default), `$php_errormsg` was never set; write `null` in its place then. Ferry offers both as options. `$HTTP_RAW_POST_DATA` only existed in the global scope, so in functions that don't take it with `global`, Ferry writes `null` itself.",
      ],
    },
    before: "$data = @file_get_contents($url);\nif ($data === false) {\n    log_error($php_errormsg);\n}",
    after: "error_clear_last();\n$data = @file_get_contents($url);\nif ($data === false) {\n    log_error(error_get_last()['message'] ?? null);\n}",
  },
  "removed.assert_quiet_eval": {
    steps: {
      tr: [
        "`assert_options(ASSERT_QUIET_EVAL, …)` satırını silin; Ferry bunu kendisi yapar. Ayar yalnızca PHP 8'in artık çalıştırmadığı metin assert'lerini etkiliyordu.",
        "Sabit başka bir yerde geçiyorsa (bir diziye ya da değişkene konuyorsa) onu o koddan çıkarın. Yerine 5 yazmayın: 5, PHP 7.4'te `ASSERT_QUIET_EVAL`'ın, PHP 8'de `ASSERT_EXCEPTION`'ın değeri.",
        "Metin olarak yazılmış assert'leri (`assert('$x > 0')`) ifadeye çevirin: `assert($x > 0)`.",
      ],
      en: [
        "Delete the `assert_options(ASSERT_QUIET_EVAL, …)` line; Ferry does this itself. The setting only affected string assertions, which PHP 8 no longer evaluates.",
        "Where the constant appears elsewhere (put in an array or a variable), take it out of that code. Don't write 5 instead: 5 is `ASSERT_QUIET_EVAL` on PHP 7.4 and `ASSERT_EXCEPTION` on PHP 8.",
        "Turn assertions written as strings (`assert('$x > 0')`) into expressions: `assert($x > 0)`.",
      ],
    },
    before: "$options = [ASSERT_ACTIVE, ASSERT_QUIET_EVAL];",
    after: "$options = [ASSERT_ACTIVE];",
  },
  "removed.filter_sanitize_magic_quotes": {
    steps: {
      tr: [
        "`FILTER_SANITIZE_MAGIC_QUOTES` yerine `FILTER_SANITIZE_ADD_SLASHES` yazın; sonuç aynıdır, ikisi de `addslashes()` gibi çalışır.",
        "Ferry bu adı kendisi değiştirir. Değiştirmediyse aynı çağrıyı başka bir bulgunun düzeltmesi yeniden yazıyordur (örneğin `INPUT_REQUEST`); önce o bulguyu düzeltip yeniden tarayın.",
        "`FILTER_SANITIZE_ADD_SLASHES` PHP 7.3'te geldi; proje daha eski bir sürümde çalışıyorsa değişikliği hedef sürüme geçerken yapın.",
      ],
      en: [
        "Write `FILTER_SANITIZE_ADD_SLASHES` instead of `FILTER_SANITIZE_MAGIC_QUOTES`; the result is the same, both work like `addslashes()`.",
        "Ferry renames it itself. If it didn't, another finding's fix rewrites the same call (for example `INPUT_REQUEST`); fix that finding first and scan again.",
        "`FILTER_SANITIZE_ADD_SLASHES` came in PHP 7.3; if the project runs an older version, make the change when you move to the target.",
      ],
    },
    before: "$name = filter_var($name, FILTER_SANITIZE_MAGIC_QUOTES);",
    after: "$name = filter_var($name, FILTER_SANITIZE_ADD_SLASHES);",
  },
  "removed.money_format": {
    steps: {
      tr: [
        "Proje parasal yerel ayar seçmiyorsa Ferry `%i` ve `%n` biçimlerini `sprintf('%.2f', $v)`'ye kendisi çevirir; C yerel ayarında sonuç aynıdır.",
        "Proje `setlocale(LC_MONETARY, …)` ile bir yerel ayar seçiyorsa eski çıktı para birimini ve ayırıcıları o yerel ayara göre yazıyordu. intl eklentisiyle `NumberFormatter` kullanın: `(new NumberFormatter('tr_TR', NumberFormatter::CURRENCY))->formatCurrency($v, 'TRY')`.",
        "`NumberFormatter`'ın çıktısı `money_format()`'ınkiyle birebir aynı değildir (boşluklar ve simgenin yeri değişebilir); çıktıyı karşılaştıran testleri güncelleyin.",
        "Para birimi simgesi gerekmiyorsa `number_format($v, 2, ',', '.')` ile ayırıcıları kendiniz seçin.",
      ],
      en: [
        "If the project sets no monetary locale, Ferry turns the `%i` and `%n` formats into `sprintf('%.2f', $v)` itself; in the C locale the result is the same.",
        "If the project picks a locale with `setlocale(LC_MONETARY, …)`, the old output wrote the currency and separators for that locale. Use `NumberFormatter` from the intl extension: `(new NumberFormatter('tr_TR', NumberFormatter::CURRENCY))->formatCurrency($v, 'TRY')`.",
        "`NumberFormatter`'s output isn't exactly `money_format()`'s (spacing and the symbol's position can differ); update tests that compare the output.",
        "If you don't need a currency symbol, choose the separators yourself with `number_format($v, 2, ',', '.')`.",
      ],
    },
    before: "setlocale(LC_MONETARY, 'tr_TR');\necho money_format('%n', $total);",
    after: "$money = new NumberFormatter('tr_TR', NumberFormatter::CURRENCY);\necho $money->formatCurrency($total, 'TRY');",
  },
  "deprecated.numberformatter_type_currency": {
    steps: {
      tr: [
        "`$fmt->format($v, NumberFormatter::TYPE_CURRENCY)` yerine `$fmt->formatCurrency($v, 'EUR')` yazın; para birimini ISO 4217 koduyla verin ve biçimlendiriciyi `NumberFormatter::CURRENCY` stiliyle oluşturun.",
        "`parse($text, NumberFormatter::TYPE_CURRENCY)` yerine `parseCurrency($text, $currency)` yazın; okunan para birimi kodu ikinci argümana yazılır.",
        "PHP 8.0'dan beri `format()` ve `parse()` bu türü kabul etmiyor. Sabit yalnızca bir değer olarak saklanıyorsa Ferry'nin seçenekleri onu yeni metoda ya da `false`'a çevirmeyi önerir.",
      ],
      en: [
        "Replace `$fmt->format($v, NumberFormatter::TYPE_CURRENCY)` with `$fmt->formatCurrency($v, 'EUR')`; give the currency as an ISO 4217 code and create the formatter with the `NumberFormatter::CURRENCY` style.",
        "Replace `parse($text, NumberFormatter::TYPE_CURRENCY)` with `parseCurrency($text, $currency)`; the currency code it reads is written to the second argument.",
        "Since PHP 8.0, `format()` and `parse()` don't accept this type. Where the constant is only stored as a value, Ferry's options offer to change it to the new method or to `false`.",
      ],
    },
    before: "$text = $fmt->format($price, NumberFormatter::TYPE_CURRENCY);",
    after: "$text = $fmt->formatCurrency($price, 'EUR');",
  },
  "deprecated.splfileinfo_bad_state_ex": {
    steps: {
      tr: [
        "Bu metot her çağrıldığında hata fırlatır: PHP 7'de `LogicException`, PHP 8'de `Error`. Yani sonrasındaki kod hiç çalışmıyordu.",
        "Sonraki kodun çalışması gerekiyorsa çağrıyı silin (*Çağrıyı sil* seçeneği).",
        "Hata isteniyorsa kendiniz fırlatın (*Hatayı koru* seçeneği): `throw new \\Error('The parent constructor was not called: the object is in an invalid state');`. PHP 7'de `LogicException` yakalayan kod varsa onu `\\Error` yakalayacak şekilde değiştirin.",
      ],
      en: [
        "This method throws every time it's called: `LogicException` on PHP 7, `Error` on PHP 8. So the code after it never ran.",
        "If the code after it should run, delete the call (the *Delete the call* option).",
        "If the error is wanted, throw it yourself (the *Keep the error* option): `throw new \\Error('The parent constructor was not called: the object is in an invalid state');`. If code catches `LogicException` from it on PHP 7, change it to catch `\\Error`.",
      ],
    },
    before: "$file->_bad_state_ex();",
    after: "throw new \\Error('The parent constructor was not called: the object is in an invalid state');",
  },
  "server.php_version_pin": {
    steps: {
      tr: [
        "Düzeltmeler ekranında **Sürüm dosyaları**'nı seçin: Ferry Dockerfile, CI ve Compose dosyalarındaki PHP sürümünü hedefe taşır ve kodla aynı önizleme, yedek ve geri alma ile yazar.",
        "CI'daki sürüm listelerine hedef eklenir, eski sürümler kalır; digest ile sabitlenmiş imajlara dokunulmaz, onları kendiniz güncelleyin.",
        "Tam yama sürümü yazan satırlarda (`8.5.0`) kullandığınız yama sürümünü yazın.",
        "Sunucunun ya da barındırma panelinin PHP sürümünü ayrıca değiştirin; Ferry yalnızca projedeki dosyaları günceller.",
      ],
      en: [
        "On the Fixes screen, choose **Version files**: Ferry moves the PHP version in Dockerfile, CI and Compose files to the target and writes it with the same preview, backup and undo as the code.",
        "Version lists in CI get the target added and keep the old versions; images pinned by digest aren't touched, so update those yourself.",
        "Where a line names an exact release (`8.5.0`), write the patch release you use.",
        "Change the PHP version of the server or hosting panel too; Ferry only updates files in the project.",
      ],
    },
    before: "FROM php:7.4-apache",
    after: "FROM php:8.5-apache",
  },
  "server.composer1_lock": {
    steps: {
      tr: [
        "Composer 2'yi kurun ya da güncelleyin: `composer self-update --2`. CI ve Dockerfile'da da Composer 2 kullanın (örneğin `composer:2` imajı).",
        "`composer update --lock` çalıştırın: Composer 2 kilitli sürümleri değiştirmeden dosyayı kendi biçiminde yeniden yazar.",
        "Composer 1'e bağlı eklentiler Composer 2'de çalışmaz; Ferry bunları ayrı bulgu olarak gösterir, önce onları güncelleyin.",
        "Ardından paketleri hedef PHP sürümünü destekleyen sürümlere yükseltin; Bağımlılıklar ekranı hangilerinin desteklemediğini gösterir.",
      ],
      en: [
        "Install or update Composer 2: `composer self-update --2`. Use Composer 2 in CI and Dockerfiles too (for example the `composer:2` image).",
        "Run `composer update --lock`: Composer 2 rewrites the file in its own format without changing the locked versions.",
        "Plugins tied to Composer 1 don't work on Composer 2; Ferry lists them as separate findings, so update those first.",
        "Then move packages to releases that support the target PHP; the Dependencies screen shows which ones don't.",
      ],
    },
  },
  "server.php_value_outside_mod_php": {
    steps: {
      tr: [
        "Ayarları `.htaccess` ile aynı klasördeki bir `.user.ini` dosyasına `ad = değer` biçiminde taşıyın: `php_value upload_max_filesize 16M` → `upload_max_filesize = 16M`, `php_flag display_errors off` → `display_errors = Off`. PHP-FPM ve CGI bu dosyayı okur.",
        "`.htaccess`'teki satırları `<IfModule mod_php.c>` … `</IfModule>` içine alın; böylece mod_php kullanan sunucular da ayarları almaya devam eder ve PHP-FPM'de 500 hatası çıkmaz.",
        "PHP `.user.ini`'yi varsayılan olarak beş dakikada bir yeniden okur (`user_ini.cache_ttl`); değişiklik hemen görünmezse bekleyin ya da PHP-FPM'i yeniden başlatın.",
        "`php_admin_value` ve `php_admin_flag` `.user.ini`'de kullanılamaz; bunları sunucunun PHP-FPM havuz yapılandırmasına yazın.",
      ],
      en: [
        "Move the settings to a `.user.ini` file in the same folder as `.htaccess`, as `name = value`: `php_value upload_max_filesize 16M` → `upload_max_filesize = 16M`, `php_flag display_errors off` → `display_errors = Off`. PHP-FPM and CGI read this file.",
        "Wrap the lines in `.htaccess` in `<IfModule mod_php.c>` … `</IfModule>`, so servers that use mod_php keep the settings and PHP-FPM doesn't answer with a 500 error.",
        "PHP rereads `.user.ini` every five minutes by default (`user_ini.cache_ttl`); if the change doesn't show at once, wait or restart PHP-FPM.",
        "`php_admin_value` and `php_admin_flag` can't go in `.user.ini`; put those in the server's PHP-FPM pool configuration.",
      ],
    },
    before: "php_value upload_max_filesize 16M\nphp_flag display_errors off",
    after: "<IfModule mod_php.c>\n    php_value upload_max_filesize 16M\n    php_flag display_errors off\n</IfModule>",
  },
  "server.old_php_module": {
    steps: {
      tr: [
        "`<IfModule mod_php7.c>`'yi `<IfModule mod_php.c>` yapın; PHP 8'in Apache modülü bu adı kullanır.",
        "Sunucu geçiş sırasında bir süre eski sürümle çalışacaksa bloğu ikiye çoğaltın, biri eski adla, biri `mod_php.c` ile; ayarlar iki sürümde de uygulanır.",
        "Sunucu PHP-FPM kullanıyorsa bu blok hiç uygulanmaz; içindeki ayarları `.user.ini` dosyasına taşıyın.",
      ],
      en: [
        "Change `<IfModule mod_php7.c>` to `<IfModule mod_php.c>`; PHP 8's Apache module uses that name.",
        "If the server keeps running the old version for a while during the move, duplicate the block, one with the old name and one with `mod_php.c`, so the settings apply on both.",
        "If the server uses PHP-FPM, this block never applies; move the settings inside it to a `.user.ini` file.",
      ],
    },
    before: "<IfModule mod_php7.c>\n    php_value memory_limit 256M\n</IfModule>",
    after: "<IfModule mod_php.c>\n    php_value memory_limit 256M\n</IfModule>",
  },
  "framework.wordpress_version": {
    steps: {
      tr: [
        "Yedek alın, sonra WordPress'i yönetim panelindeki Güncellemeler sayfasından ya da WP-CLI ile güncelleyin: `wp core update`.",
        "`wp-includes/version.php` WordPress'in kendi dosyasıdır; sürüm numarasını elle değiştirmeyin.",
        "Eklentileri ve temaları da güncelleyin. Ferry `wp-content` altındaki eklenti ve tema kodunu da tarar; güncellemeden sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup, then update WordPress from the Updates page in the dashboard or with WP-CLI: `wp core update`.",
        "`wp-includes/version.php` is WordPress's own file; don't change the version number by hand.",
        "Update plugins and themes too. Ferry also scans plugin and theme code under `wp-content`; scan the project again after updating.",
      ],
    },
  },
  "framework.laravel_version": {
    steps: {
      tr: [
        "Önerilen satırdaki komutla Laravel'i hedef PHP sürümünü destekleyen sürüme yükseltin.",
        "Birden fazla ana sürüm atlanıyorsa sürüm sürüm ilerleyin ve her adımda Laravel'in resmi yükseltme kılavuzunu izleyin.",
        "Laravel'e bağlı paketleri de güncelleyin; Bağımlılıklar ekranı hangilerinin hedefi desteklemediğini gösterir.",
        "Testleri çalıştırın; Gerçek test ekranı onları eski ve yeni sürümde karşılaştırabilir.",
      ],
      en: [
        "Upgrade Laravel to a release that supports the target PHP with the command in the suggested line.",
        "If you skip more than one major release, go one at a time and follow Laravel's official upgrade guide at each step.",
        "Update the packages that depend on Laravel too; the Dependencies screen shows which of them don't support the target.",
        "Run the tests; the Real test screen can compare them on the old and new version.",
      ],
    },
  },
  "behavior.inherited_static_variable": {
    steps: {
      tr: [
        "Alt sınıfta ezilmeyen bir metottaki `static $x`, PHP 8.1'den beri bütün sınıflar için tek kopyadır. PHP 8.0 ve öncesinde her alt sınıfın kendi kopyası vardı; örneğin `B::instance()` artık `A`'nın nesnesini döndürür.",
        "Değer her sınıf için ayrı kalmalıysa sınıf adını anahtar yapın: `static $instances = []; return $instances[static::class] ??= new static();`.",
        "Ortak tek bir değer zaten istenen buysa (sınıftan bağımsız bir önbellek gibi) değişiklik gerekmez.",
      ],
      en: [
        "A `static $x` in a method the subclass doesn't override is a single copy for every class since PHP 8.1. On PHP 8.0 and before, each subclass had its own copy; `B::instance()`, for example, now returns `A`'s object.",
        "If the value has to stay separate per class, key it by class name: `static $instances = []; return $instances[static::class] ??= new static();`.",
        "If one shared value is what was meant (a cache that doesn't depend on the class, say), nothing needs to change.",
      ],
    },
    before: "public static function instance()\n{\n    static $instance = null;\n    return $instance ??= new static();\n}",
    after: "public static function instance()\n{\n    static $instances = [];\n    return $instances[static::class] ??= new static();\n}",
  },
  "behavior.func_get_args_current_value": {
    steps: {
      tr: [
        "PHP 5'te `func_get_args()` parametrelerin çağrıdaki değerini veriyordu. PHP 7.0'dan beri, fonksiyon parametreyi daha önce değiştirdiyse güncel değerini verir.",
        "Eski değer gerekiyorsa `func_get_args()`'ı fonksiyonun başında, parametreler değişmeden önce bir değişkene alın. Ferry bunu proje bugün PHP 5'te çalışıyorsa kendisi yapar.",
        "Proje zaten PHP 7 ya da sonrasında çalışıyorsa bugünkü davranış değişmez ve bulgu yok sayılabilir.",
      ],
      en: [
        "On PHP 5, `func_get_args()` gave the values the parameters were called with. Since PHP 7.0 it gives the current value when the function changed a parameter first.",
        "If the old value is needed, read `func_get_args()` into a variable at the start of the function, before any parameter changes. Ferry does this itself when the project runs PHP 5 today.",
        "If the project already runs PHP 7 or later, today's behavior doesn't change and the finding can be ignored.",
      ],
    },
    before: "function save($data)\n{\n    $data = trim($data);\n    log_call(func_get_args());\n}",
    after: "function save($data)\n{\n    $args = func_get_args();\n    $data = trim($data);\n    log_call($args);\n}",
  },
  "deprecated.libxml_disable_entity_loader": {
    steps: {
      tr: [
        "PHP 8 en az libxml 2.9 ile gelir ve bu sürüm dış varlıkları zaten yüklemez; çağrı bir şey yapmaz. Çağrıyı silin.",
        "Değer saklanıp sonra geri veriliyorsa (`$old = libxml_disable_entity_loader(true); … libxml_disable_entity_loader($old);`) iki satırı da silin.",
        "Kod `LIBXML_NOENT` ile dış varlıkları açıkça istiyorsa libxml onları yükler. Bunları yine engellemek istiyorsanız bir yükleyici verin: `libxml_set_external_entity_loader(static fn () => null);`.",
      ],
      en: [
        "PHP 8 ships with at least libxml 2.9, which doesn't load external entities anyway, so the call does nothing. Delete it.",
        "If the value is stored and restored later (`$old = libxml_disable_entity_loader(true); … libxml_disable_entity_loader($old);`), delete both lines.",
        "If the code asks for external entities with `LIBXML_NOENT`, libxml loads them. To keep blocking them, set a loader: `libxml_set_external_entity_loader(static fn () => null);`.",
      ],
    },
    before: "$old = libxml_disable_entity_loader(true);\n$doc->loadXML($xml);\nlibxml_disable_entity_loader($old);",
    after: "$doc->loadXML($xml);",
  },
  "behavior.locale_float": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `echo`, birleştirme ve `(string)` ondalık sayıları yerel ayardan bağımsız olarak noktayla yazar: `setlocale(LC_NUMERIC, 'de_DE')` olsa bile `1.5`. PHP 7 `1,5` yazıyordu.",
        "Kullanıcıya gösterilen sayıların ayırıcılarını açıkça seçin: `number_format($x, 2, ',', '.')` ya da intl ile `NumberFormatter`.",
        "`printf('%f')` hâlâ yerel ayara uyar; ayardan bağımsız biçim için `%F` yazın.",
        "Veritabanına, JSON'a ya da CSV'ye giden sayılar artık her zaman noktalı; virgülü düzeltmek için yazılmış `str_replace(',', '.', $x)` gibi satırlar gereksizleşir.",
      ],
      en: [
        "Since PHP 8.0, `echo`, concatenation and `(string)` write floats with a dot whatever the locale: `1.5` even with `setlocale(LC_NUMERIC, 'de_DE')`. PHP 7 wrote `1,5`.",
        "Choose the separators of numbers shown to users explicitly: `number_format($x, 2, ',', '.')` or `NumberFormatter` from intl.",
        "`printf('%f')` still follows the locale; write `%F` for a format that doesn't.",
        "Numbers going to a database, JSON or CSV always have a dot now, so lines written to fix the comma, such as `str_replace(',', '.', $x)`, become unnecessary.",
      ],
    },
    before: "setlocale(LC_NUMERIC, 'tr_TR');\necho 'Toplam: ' . $total;",
    after: "echo 'Toplam: ' . number_format($total, 2, ',', '.');",
  },
  "behavior.locale_conversions": {
    steps: {
      tr: [
        "Sayılar: PHP 8.0'dan beri ondalık sayılar yerel ayardan bağımsız olarak noktayla yazılır; gösterilen sayıları `number_format()` ya da `NumberFormatter` ile biçimlendirin.",
        "Harfler: PHP 8.2'den beri `strtolower()`, `strtoupper()`, `ucfirst()` ve benzerleri yerel ayara bakmaz, yalnızca ASCII harfleri değiştirir. ASCII dışı harfler için `mb_strtolower()` ve `mb_strtoupper()` kullanın.",
        "`mb_strtolower()` Türkçe kuralları bilmez: `IŞIK` → `işik` olur. Türkçe metinde önce `I` ve `İ`'yi değiştirin: `mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $s))`.",
      ],
      en: [
        "Numbers: since PHP 8.0 floats are written with a dot whatever the locale; format numbers you show with `number_format()` or `NumberFormatter`.",
        "Letters: since PHP 8.2 `strtolower()`, `strtoupper()`, `ucfirst()` and similar functions ignore the locale and change ASCII letters only. Use `mb_strtolower()` and `mb_strtoupper()` for other letters.",
        "`mb_strtolower()` doesn't know Turkish rules: `IŞIK` becomes `işik`. For Turkish text, replace `I` and `İ` first: `mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $s))`.",
      ],
    },
    before: "setlocale(LC_ALL, 'tr_TR');\n$slug = strtolower($title);",
    after: "$slug = mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $title));",
  },
  "deprecated.e_user_error": {
    steps: {
      tr: [
        "`trigger_error($msg, E_USER_ERROR)` yerine istisna fırlatın: `throw new \\ErrorException($msg, 0, E_USER_ERROR);`. İşleyicisi olmayan bir `E_USER_ERROR` betiği 255 koduyla durduruyordu; yakalanmayan istisna da aynı kodla durdurur.",
        "Bir hata işleyicisi bu hatayı yakalayıp betiği sürdürüyorsa `E_USER_WARNING` kullanın ve betiğin durması gerekiyorsa ardından `exit(255)` yazın.",
        "Ferry tek başına duran `trigger_error(…, E_USER_ERROR);` satırları için bu iki yolu seçenek olarak sunar. `E_USER_ERROR` başka bir yerde, örneğin bir hata maskesinde geçiyorsa onu elle kaldırın.",
      ],
      en: [
        "Throw an exception instead of `trigger_error($msg, E_USER_ERROR)`: `throw new \\ErrorException($msg, 0, E_USER_ERROR);`. An `E_USER_ERROR` with no handler stopped the script with code 255, and an uncaught exception stops it with the same code.",
        "If an error handler catches it and lets the script go on, use `E_USER_WARNING`, followed by `exit(255)` if the script has to stop.",
        "Ferry offers both as options for `trigger_error(…, E_USER_ERROR);` lines that stand alone. Where `E_USER_ERROR` appears elsewhere, such as in an error mask, remove it by hand.",
      ],
    },
    before: "trigger_error('Config missing', E_USER_ERROR);",
    after: "throw new \\ErrorException('Config missing', 0, E_USER_ERROR);",
  },
  "deprecated.false_to_array": {
    steps: {
      tr: [
        "Değeri `false` olan bir değişkene `$x[] = …` ya da `$x['k'] = …` ile yazmak PHP 8.1'de kullanımdan kalktı; değişkeni yazmadan önce `[]` ile başlatın. Ferry bunu değişken aynı akışta `false` aldığında kendisi yapar.",
        "Değer bir fonksiyondan `false` olarak gelebiliyorsa (`file()`, `strpos()`, veritabanı sonuçları) yazmadan önce denetleyin: `if ($rows === false) { $rows = []; }`.",
        "`null` ve tanımsız değişkenler etkilenmez, yalnızca `false`.",
      ],
      en: [
        "Writing to a variable that holds `false` with `$x[] = …` or `$x['k'] = …` is deprecated since PHP 8.1; initialize the variable with `[]` before writing. Ferry does this itself when the variable gets `false` in the same flow.",
        "If the value can come back `false` from a function (`file()`, `strpos()`, database results), check it before writing: `if ($rows === false) { $rows = []; }`.",
        "`null` and undefined variables aren't affected, only `false`.",
      ],
    },
    before: "$rows = $db->fetchAll($sql);\n$rows[] = $extra;",
    after: "$rows = $db->fetchAll($sql);\nif ($rows === false) {\n    $rows = [];\n}\n$rows[] = $extra;",
  },
  "removed.regex_class_range": {
    steps: {
      tr: [
        "PHP 7.3'teki PCRE2, `\\w`, `\\d` ya da `\\s` gibi bir sınıfla başlayan aralığı (`[\\w-.]`) derlemez; desen uyarı verip false döndürür. Eskiden tire düz bir karakter sayılıyordu.",
        "Tireyi kaçırın (`[\\w\\-.]`) ya da sınıfın sonuna yazın (`[\\w.-]`); ikisi de eski anlamı korur. Rector desen doğrudan bir `preg_*` çağrısında metin olarak yazılmışsa bunu kendisi yapar.",
        "Desen bir değişkenden ya da birleştirilmiş metinden geliyorsa onu tanımlandığı yerde düzeltin.",
      ],
      en: [
        "PCRE2 in PHP 7.3 doesn't compile a range that starts at a class such as `\\w`, `\\d` or `\\s` (`[\\w-.]`); the pattern warns and returns false. The hyphen used to count as a plain character.",
        "Escape the hyphen (`[\\w\\-.]`) or put it last (`[\\w.-]`); both keep the old meaning. Rector does this itself when the pattern is written as a string right in a `preg_*` call.",
        "If the pattern comes from a variable or joined strings, fix it where it's defined.",
      ],
    },
    before: "preg_match('/^[\\w-.]+$/', $name);",
    after: "preg_match('/^[\\w\\-.]+$/', $name);",
  },
  "removed.array_key_exists_object": {
    steps: {
      tr: [
        "`ArrayObject` ya da `ArrayIterator`'da (ve onlardan türeyen sınıflarda) `$obj->offsetExists($key)` yazın; değeri null olan anahtarlar dahil eski sonucu verir.",
        "Diğer nesnelerde `property_exists($obj, $key)` yazın. Tek fark: private ve protected özellikler için `property_exists()` true döndürür, eski çağrı sınıfın dışından false veriyordu.",
      ],
      en: [
        "On `ArrayObject` or `ArrayIterator` (and classes extending them), write `$obj->offsetExists($key)`; it gives the old result, keys with a null value included.",
        "On other objects, write `property_exists($obj, $key)`. The one difference: `property_exists()` returns true for private and protected properties, where the old call gave false from outside the class.",
      ],
    },
    before: "return array_key_exists($index, $this);",
    after: "return $this->offsetExists($index);",
  },
};
