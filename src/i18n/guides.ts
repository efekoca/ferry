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
        "eval'e giden kodun nereden geldiğini bulun. Şablon ya da önbellek dosyalarından geliyorsa o dosyaları da gözden geçirin; Ferry `.phtml` ve `.inc` gibi PHP dosyalarını tarar, Twig gibi başka biçimleri okumaz.",
        "PHP 7.0'dan beri eval içindeki sözdizimi hatası false döndürmüyor, ParseError fırlatıyor: `if (eval(…) === false)` denetimlerini `try { … } catch (\\ParseError $e) { … }` ile değiştirin.",
        "PHP 8.0'dan beri değerlendirilen koddaki bazı uyarılar Error'a dönüştü (tanımsız sabitler, yanlış argüman sayısı); şablonların geçtiği yolları hedef sürümün testlerinde çalıştırın.",
      ],
      en: [
        "Find where the code passed to eval comes from. If it comes from template or cache files, review those too; Ferry scans PHP files such as `.phtml` and `.inc`, but not other formats such as Twig.",
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
};
