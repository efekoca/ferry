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

const MAGIC_QUOTES_SETTER: Guide = {
  steps: {
    tr: [
      "Magic quotes PHP 5.4'te kaldırıldı; `set_magic_quotes_runtime()` ve takma adı `magic_quotes_runtime()` PHP 7.0'da tamamen silindi. PHP 5.4'ten beri onları false ile çağırmak hiçbir şey yapmıyordu, true ile çağırmak ise ölümcül hataydı.",
      "Çağrıyı silin. Ferry tek başına duran çağrıları kendisi siler; çağrı `if (get_magic_quotes_runtime()) { ... }` gibi hiç çalışmayan bir bloğun içindeyse bloğu bütünüyle siler.",
    ],
    en: [
      "Magic quotes were removed in PHP 5.4, and `set_magic_quotes_runtime()` and its alias `magic_quotes_runtime()` were deleted in PHP 7.0. Since PHP 5.4, calling them with false did nothing and calling them with true was a fatal error.",
      "Remove the call. Ferry removes calls that stand alone; when the call sits in a block that never runs, such as `if (get_magic_quotes_runtime()) { ... }`, it removes the whole block.",
    ],
  },
  before: "if (get_magic_quotes_runtime()) {\n    set_magic_quotes_runtime(0);\n}\n$data = fread($fp, $size);",
  after: "$data = fread($fp, $size);",
};

export const GUIDES: Record<string, Guide> = {
  "removed.set_magic_quotes_runtime": MAGIC_QUOTES_SETTER,
  "removed.magic_quotes_runtime": MAGIC_QUOTES_SETTER,
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
  "removed.builtin_constant": {
    steps: {
      tr: [
        "Bu sabit PHP'den kaldırıldı; onu kullanan satır hedef sürümde `Undefined constant` Error fırlatır.",
        "Sabit kaldırılan bir eklentiye aitse kodu o eklentinin yerine geçene taşırken karşılığını kullanın. `MYSQL_ASSOC`, `MYSQL_NUM`, `MYSQL_BOTH`, `MYSQL_CLIENT_COMPRESS` ve `MYSQL_CLIENT_SSL`'in `MYSQLI_` karşılıkları aynı değerleri taşır.",
        "Sabitin bir karşılığı yoksa kaldırılan özelliğe bağlı olan kodu kaldırın ya da yeniden yazın.",
      ],
      en: [
        "This constant was removed from PHP; a line that uses it throws an `Undefined constant` Error on the target version.",
        "If it belongs to a removed extension, use its counterpart when you move the code to what replaced the extension. `MYSQL_ASSOC`, `MYSQL_NUM`, `MYSQL_BOTH`, `MYSQL_CLIENT_COMPRESS` and `MYSQL_CLIENT_SSL` have `MYSQLI_` counterparts with the same values.",
        "If the constant has no counterpart, remove or rewrite the code that depends on the removed feature.",
      ],
    },
    before: "$row = mysql_fetch_array($result, MYSQL_ASSOC);",
    after: "$row = mysqli_fetch_array($result, MYSQLI_ASSOC);",
  },
  "removed.builtin_arity": {
    steps: {
      tr: [
        "PHP 8.0'dan beri yerleşik bir fonksiyona fazla ya da eksik argüman vermek ArgumentCountError fırlatır. PHP 7'de çoğu fonksiyon uyarı verip null ya da false döndürüyor ve hiç çalışmıyordu; bazıları ise fazla argümanı yok sayıp çalışıyordu (`mysqli_connect_errno($link)` gibi).",
        "Çağrının amacına göre fazla argümanı silin ya da eksik olanı ekleyin. Proje bugün PHP 8'den eski bir sürümde çalışıyorsa Ferry, bilinen durumlarda eski sonucu korumayı ya da fazla argümanı silmeyi seçenek olarak sunar.",
        "`openssl_seal()` ve `openssl_open()` artık şifre yöntemini ister; eski varsayılan RC4'tü. OpenSSL 3 RC4'ü varsayılan olarak kapattığı için PHP 8.5'in resmî imajında `'RC4'` vermek de false döndürdü. Yeni veri için `'AES-256-CBC'` ve bir IV kullanın, IV'yi de saklayıp `openssl_open()`'a verin.",
      ],
      en: [
        "Since PHP 8.0, passing too many or too few arguments to a built-in function throws ArgumentCountError. On PHP 7 most functions warned and returned null or false without running; some ignored the extra arguments and ran (such as `mysqli_connect_errno($link)`).",
        "Depending on what the call is for, remove the extra argument or add the missing one. When the project runs a version older than PHP 8 today, Ferry offers to keep the old result or drop the extra arguments where it knows them.",
        "`openssl_seal()` and `openssl_open()` now require the cipher; the old default was RC4. OpenSSL 3 disables RC4 by default, so passing `'RC4'` returned false too on PHP 8.5's official image. Use `'AES-256-CBC'` with an IV for new data, and store the IV to pass to `openssl_open()`.",
      ],
    },
    before: "openssl_seal($data, $sealed, $keys, $publicKeys);",
    after: "$iv = random_bytes(16);\nopenssl_seal($data, $sealed, $keys, $publicKeys, 'AES-256-CBC', $iv);",
  },
  "behavior.idn_default_variant": {
    steps: {
      tr: [
        "PHP 7.4'ten beri `idn_to_ascii()` ve `idn_to_utf8()` varsayılan olarak IDNA2003 yerine UTS #46 kurallarını kullanır. Çoğu alan adı aynı sonucu verir.",
        "Farklar şunlar: büyük harfli ASCII küçültülür (`FASS.de` → `fass.de`), emoji gibi semboller kabul edilir (`☕.example` → `xn--53h.example`), başta ya da sonda tire ve 3. ile 4. karakterde `--` taşıyan etiketler ile geçersiz `xn--` etiketleri false döner. `idn_to_utf8()` artık `xn--fa-hia.de` gibi adları `faß.de`'ye çözer.",
        "Sonuç PHP'nin kullandığı ICU sürümüne de bağlıdır: PHP 7.4'ün ICU 71'i `faß.de`'yi `fass.de`'ye çevirdi, PHP 8.5 resmî imajının ICU 76'sı `xn--fa-hia.de` verdi. `ς` ve görünmez birleştiriciler (ZWJ) için de aynı fark var.",
        "IDNA2003 PHP 8.0'da kaldırıldı; eski sonuca dönmenin yolu yok. Bu çağrılara giden alan adlarını hedef sunucuda deneyin, false dönenleri ve büyük-küçük harfe dayanan karşılaştırmaları kontrol edin. Varyantı açıkça yazmak sonucu değiştirmez.",
      ],
      en: [
        "Since PHP 7.4, `idn_to_ascii()` and `idn_to_utf8()` use UTS #46 instead of IDNA2003 by default. Most domains convert the same.",
        "The differences: uppercase ASCII is lowercased (`FASS.de` → `fass.de`), symbols such as emoji are accepted (`☕.example` → `xn--53h.example`), and labels with a leading or trailing hyphen, `--` in the 3rd and 4th characters, or an invalid `xn--` return false. `idn_to_utf8()` now decodes names like `xn--fa-hia.de` to `faß.de`.",
        "The result also depends on the ICU version PHP uses: PHP 7.4's ICU 71 turned `faß.de` into `fass.de`, while ICU 76 in PHP 8.5's official image gave `xn--fa-hia.de`. The same goes for `ς` and invisible joiners (ZWJ).",
        "IDNA2003 was removed in PHP 8.0, so there is no way back to the old result. Try the domains these calls get on the target server, and check the ones that return false and any comparison that relies on letter case. Writing the variant out changes nothing.",
      ],
    },
    before: "$host = idn_to_ascii($host);",
    after: "$host = idn_to_ascii($host, IDNA_DEFAULT, INTL_IDNA_VARIANT_UTS46);",
  },
  "behavior.error_reporting_zero": {
    steps: {
      tr: [
        "PHP 8'de `@` hata seviyesini 0'a indirmez; ölümcül hata bitlerini (`E_ERROR`, `E_PARSE`, `E_CORE_ERROR`, `E_COMPILE_ERROR`, `E_USER_ERROR`, `E_RECOVERABLE_ERROR`) açık bırakır. `error_reporting() == 0` kontrolü bu yüzden susturulan hatalarda artık doğru olmaz.",
        "Kontrol `set_error_handler()` ile kurulan bir işleyicinin içindeyse hata numarasına bakın: `if (!(error_reporting() & $errno)) { return false; }`.",
        "Hata numarası yoksa ölümcül bitleri maskeleyin: `(error_reporting() & ~(E_ERROR | E_PARSE | E_CORE_ERROR | E_COMPILE_ERROR | E_USER_ERROR | E_RECOVERABLE_ERROR)) === 0`. Bu kontrol PHP 5.6'dan 8.5'e kadar `@` altında doğru, normalde yanlış verdi. Ferry ikisini de kendisi yazar.",
      ],
      en: [
        "On PHP 8, `@` no longer drops the error level to 0; it keeps the fatal bits (`E_ERROR`, `E_PARSE`, `E_CORE_ERROR`, `E_COMPILE_ERROR`, `E_USER_ERROR`, `E_RECOVERABLE_ERROR`). So an `error_reporting() == 0` check is no longer true for silenced errors.",
        "When the check is inside a handler installed with `set_error_handler()`, test the error number: `if (!(error_reporting() & $errno)) { return false; }`.",
        "Without an error number, mask out the fatal bits: `(error_reporting() & ~(E_ERROR | E_PARSE | E_CORE_ERROR | E_COMPILE_ERROR | E_USER_ERROR | E_RECOVERABLE_ERROR)) === 0`. On PHP 5.6 through 8.5 this was true under `@` and false otherwise. Ferry writes both itself.",
      ],
    },
    before: "if (error_reporting() === 0) {\n    return;\n}",
    after: "if (!(error_reporting() & $errno)) {\n    return;\n}",
  },
  "deprecated.pg_implicit_connection": {
    steps: {
      tr: [
        "PHP 8.1'den beri pgsql fonksiyonlarını bağlantı vermeden çağırmak kullanımdan kalktı; bu çağrılar son açılan bağlantıyı kullanıyordu.",
        "`pg_connect()`'in döndürdüğü bağlantıyı ilk argüman olarak verin. Sınıf bağlantıyı tek bir özellikte tutuyorsa Ferry o özelliği kendisi ekler.",
        "`pg_query_params()`, `pg_prepare()` ve `pg_execute()` iki argümanla çağrıldığında ilk argüman sorgu ya da ifade adıdır; bağlantıyı onun önüne ekleyin, yerine koymayın.",
      ],
      en: [
        "Since PHP 8.1, calling pgsql functions without a connection is deprecated; these calls used the connection opened last.",
        "Pass the connection `pg_connect()` returned as the first argument. When the class keeps its connection in one property, Ferry adds that property itself.",
        "When `pg_query_params()`, `pg_prepare()` and `pg_execute()` get two arguments, the first is the query or statement name; add the connection before it, don't replace it.",
      ],
    },
    before: "pg_query('SET search_path TO app');",
    after: "pg_query($this->connection, 'SET search_path TO app');",
  },
  "removed.builtin_ini": {
    steps: {
      tr: [
        "Bu ayar PHP'den kaldırıldı. `ini_set()` onu değiştirmeden false döndürür, `ini_get()` false verir; ayarın yaptığı iş artık yapılmaz.",
        "Ferry ayarı yazan ya da okuyan basit çağrıları kendisi kaldırır ya da false yapar. Kalan çağrılarda, ayarın sağladığı şey hâlâ gerekiyorsa onu sağlayan güncel yolu kullanın.",
        "`magic_quotes_runtime` PHP 5.4'ten beri yok; okunan veri zaten olduğu gibi gelir. `session.hash_function` ve `session.hash_bits_per_character` PHP 7.1'de kaldırıldı; oturum kimliği artık PHP'nin varsayılanıyla üretilir.",
      ],
      en: [
        "PHP removed this setting. `ini_set()` returns false without changing anything and `ini_get()` returns false; whatever the setting did no longer happens.",
        "Ferry removes simple calls that write or read it, or turns them into false. For the rest, if what the setting provided is still needed, use the current way to get it.",
        "`magic_quotes_runtime` has been gone since PHP 5.4; data read comes back as it is anyway. `session.hash_function` and `session.hash_bits_per_character` were removed in PHP 7.1; session IDs now come from PHP's default.",
      ],
    },
  },
  "deprecated.mhash": {
    steps: {
      tr: [
        "mhash fonksiyonları PHP 8.1'de, `MHASH_*` sabitleri PHP 8.5'te kullanımdan kalktı. hash eklentisi aynı algoritmaları verir.",
        "`mhash(MHASH_SHA1, $data)` yerine `hash('sha1', $data, true)`, anahtarlı çağrı yerine `hash_hmac('sha1', $data, $key, true)` yazın; ikisi de ham bayt döndürür. Ferry sabit algoritmalı çağrıları ve `mhash_keygen_s2k()`'yı kendisi çevirir.",
        "Algoritma bir değişkenden geliyorsa `MHASH_*` adını hash adına çevirin: çoğu küçük harfle aynıdır (`sha256`, `md5`, `whirlpool`); `MHASH_TIGER` `tiger192,3`, `MHASH_HAVAL256` `haval256,3`, `MHASH_SNEFRU256` `snefru` olur. Kullanılabilen adları `hash_algos()` listeler.",
        "`mhash_count()` ve `mhash_get_hash_name()` ile algoritma listesi kuran kod `hash_algos()` ile kurulabilir.",
      ],
      en: [
        "The mhash functions were deprecated in PHP 8.1 and the `MHASH_*` constants in PHP 8.5. The hash extension gives the same algorithms.",
        "Write `hash('sha1', $data, true)` instead of `mhash(MHASH_SHA1, $data)`, and `hash_hmac('sha1', $data, $key, true)` for a keyed call; both return raw bytes. Ferry converts calls with a constant algorithm and `mhash_keygen_s2k()` itself.",
        "When the algorithm comes from a variable, map the `MHASH_*` name to the hash name: most are the same in lower case (`sha256`, `md5`, `whirlpool`); `MHASH_TIGER` is `tiger192,3`, `MHASH_HAVAL256` is `haval256,3` and `MHASH_SNEFRU256` is `snefru`. `hash_algos()` lists the names available.",
        "Code that builds a list of algorithms with `mhash_count()` and `mhash_get_hash_name()` can use `hash_algos()` instead.",
      ],
    },
    before: "$mac = mhash(MHASH_SHA256, $data, $key);",
    after: "$mac = hash_hmac('sha256', $data, $key, true);",
  },
  "deprecated.gmstrftime": {
    steps: {
      tr: [
        "`gmstrftime()` PHP 8.1'de kullanımdan kalktı. Ferry yerel ayardan bağımsız biçimleri `gmdate()`'e kendisi çevirir: `%Y→Y, %m→m, %d→d, %H→H, %M→i, %S→s`.",
        "Biçim bir değişkenden geliyorsa çeviriyi o değişkenin değerleri için yapın. `%x`, `%X` ve `%c` yerel ayara bağlıdır; C yerel ayarında `%x` `m/d/y`, `%X` `H:i:s` demektir.",
        "Ay ve gün adları yerel dilde gerekiyorsa `IntlDateFormatter`'ı saat dilimi `'UTC'` olacak şekilde kullanın; desen eşlemesi `strftime()` rehberindekiyle aynıdır.",
      ],
      en: [
        "`gmstrftime()` was deprecated in PHP 8.1. Ferry converts locale-independent formats to `gmdate()` itself: `%Y→Y, %m→m, %d→d, %H→H, %M→i, %S→s`.",
        "When the format comes from a variable, convert the values it takes. `%x`, `%X` and `%c` depend on the locale; in the C locale `%x` means `m/d/y` and `%X` means `H:i:s`.",
        "If month and day names are needed in a language, use `IntlDateFormatter` with the time zone `'UTC'`; the pattern mapping is the same as in the `strftime()` guide.",
      ],
    },
    before: "echo gmstrftime('%Y-%m-%d %H:%M:%S', $ts);",
    after: "echo gmdate('Y-m-d H:i:s', $ts);",
  },
  "behavior.stream_set_option_missing": {
    steps: {
      tr: [
        "PHP 7.4'ten beri bir akış sarmalayıcısından `include` ya da `require` yapılırken PHP `stream_set_option()` metodunu çağırır; metot yoksa her seferinde `stream_set_option is not implemented!` uyarısı verir.",
        "Sarmalayıcı sınıfa false döndüren bir `stream_set_option($option, $arg1, $arg2)` metodu ekleyin. Metot yokken PHP de false kabul ediyordu; `stream_set_blocking()` ve `stream_set_timeout()` gibi çağrıların sonucu değişmez, yalnızca uyarı kalkar.",
        "Ferry sınıf aynı dosyadaysa metodu kendisi ekler; başka dosyadaki sınıflara elle ekleyin.",
      ],
      en: [
        "Since PHP 7.4, PHP calls the `stream_set_option()` method when you `include` or `require` from a stream wrapper; without it, it warns `stream_set_option is not implemented!` every time.",
        "Add a `stream_set_option($option, $arg1, $arg2)` method that returns false to the wrapper class. PHP took a missing method as false too, so calls like `stream_set_blocking()` and `stream_set_timeout()` return the same; only the warning goes.",
        "Ferry adds the method itself when the class is in the same file; add it by hand to classes in other files.",
      ],
    },
    before: "class TemplateStream\n{\n    public function stream_open($path, $mode, $options, &$opened)\n    {\n        return true;\n    }\n}",
    after: "class TemplateStream\n{\n    public function stream_open($path, $mode, $options, &$opened)\n    {\n        return true;\n    }\n\n    public function stream_set_option($option, $arg1, $arg2)\n    {\n        return false;\n    }\n}",
  },
  "deprecated.ldap_connect_port": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `ldap_connect()`'e host ve portu ayrı vermek kullanımdan kalktı; tek bir URI verin.",
        "PHP iki argümanla `ldap://host:port` adresini kuruyordu; host zaten `ldap://`, `ldaps://` ya da `ldapi://` ile başlıyorsa portu yok sayıp hostu olduğu gibi kullanıyordu. Ferry aynı kuralı yazar. Host her zaman düz bir adsa `'ldap://' . $host . ':' . $port` yeterlidir.",
        "Bağlantı şifreli olmalıysa `ldaps://host:636` verin ya da bağlandıktan sonra `ldap_start_tls()` çağırın.",
      ],
      en: [
        "Since PHP 8.3, passing host and port separately to `ldap_connect()` is deprecated; pass one URI.",
        "With two arguments PHP built `ldap://host:port`; when the host already started with `ldap://`, `ldaps://` or `ldapi://`, it ignored the port and used the host as it was. Ferry writes the same rule. If the host is always a plain name, `'ldap://' . $host . ':' . $port` is enough.",
        "If the connection has to be encrypted, pass `ldaps://host:636` or call `ldap_start_tls()` after connecting.",
      ],
    },
    before: "$link = ldap_connect($host, 389);",
    after: "$link = ldap_connect('ldap://' . $host . ':389');",
  },
  "review.variable_variable": {
    steps: {
      tr: [
        "`$$a[k]` PHP 5'te `${$a[k]}` (adı dizideki değerden gelen değişken), PHP 7'den beri `${$a}[k]` (adını `$a`'nın verdiği değişkenin `k` elemanı) olarak okunur.",
        "Proje bugün PHP 5'te çalışıyorsa Ferry PHP 5 okumasını, PHP 7'de çalışıyorsa PHP 7 okumasını süslü parantezle yazar. Sürüm bilinmiyorsa iki seçeneği gösterir; `$a` bir diziyse PHP 5 okuması önerilir.",
        "Hangisinin kastedildiğine `$a`'nın ne tuttuğuna bakarak karar verin: dizi tutuyorsa PHP 5 okuması, değişken adı tutuyorsa PHP 7 okuması doğrudur.",
      ],
      en: [
        "`$$a[k]` reads as `${$a[k]}` on PHP 5 (the variable named by the array value) and as `${$a}[k]` since PHP 7 (element `k` of the variable `$a` names).",
        "When the project runs PHP 5 today, Ferry writes the PHP 5 reading with braces; when it runs PHP 7, the PHP 7 one. When the version is unknown it shows both; the PHP 5 reading is recommended when `$a` is an array.",
        "Decide from what `$a` holds: an array means the PHP 5 reading, a variable name means the PHP 7 one.",
      ],
    },
    before: "$$names[$i] = 1;",
    after: "${$names[$i]} = 1;",
  },
  "deprecated.http_response_header": {
    steps: {
      tr: [
        "PHP 8.5'te `$http_response_header` değişkeni kullanımdan kalktı. PHP 8.4'ten beri aynı başlıkları `http_get_last_response_headers()` döndürür; istek yapılmadıysa null döner.",
        "Okuma, değişkeni dolduran isteğin hemen ardındaysa değişkeni bu çağrıyla değiştirin. Ferry arada başka bir istek yapabilecek kod yoksa bunu kendisi yapar.",
        "İstekle okuma arasında başka çağrılar ya da bloklar varsa başlıkları isteğin hemen ardından bir değişkene alın ve sonraki okumalarda onu kullanın.",
      ],
      en: [
        "PHP 8.5 deprecated the `$http_response_header` variable. Since PHP 8.4, `http_get_last_response_headers()` returns the same headers, or null when no request was made.",
        "When the read comes right after the request that filled the variable, replace the variable with that call. Ferry does this itself when nothing in between could make another request.",
        "When other calls or blocks sit between the request and the read, store the headers in a variable right after the request and read that later.",
      ],
    },
    before: "$body = file_get_contents($url);\nlog_body($body);\nforeach ($http_response_header as $line) {",
    after: "$body = file_get_contents($url);\n$headers = http_get_last_response_headers() ?? [];\nlog_body($body);\nforeach ($headers as $line) {",
  },
  "behavior.locale_case": {
    steps: {
      tr: [
        "PHP 8.2'den beri `strtolower()`, `strtoupper()`, `ucfirst()`, `ucwords()` ve `lcfirst()` yerel ayardan etkilenmez, yalnızca ASCII harfleri çevirir. Bu `setlocale()` çağrısı artık o fonksiyonları değiştirmez.",
        "ASCII dışı harfler için `mb_strtolower()` ve `mb_strtoupper()` kullanın. `mb_strtolower()` Türkçe kuralları bilmez; önce `I` ve `İ`'yi değiştirin: `mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $s))`.",
        "`setlocale()` tarih, sayı biçimi ya da `strcoll()` gibi başka işler için de gerekiyorsa çağrıyı tutun.",
      ],
      en: [
        "Since PHP 8.2, `strtolower()`, `strtoupper()`, `ucfirst()`, `ucwords()` and `lcfirst()` ignore the locale and change ASCII letters only. This `setlocale()` call no longer affects them.",
        "Use `mb_strtolower()` and `mb_strtoupper()` for other letters. `mb_strtolower()` doesn't know Turkish rules; replace `I` and `İ` first: `mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $s))`.",
        "Keep the `setlocale()` call if it's also needed for other things, such as dates, number formats or `strcoll()`.",
      ],
    },
    before: "setlocale(LC_CTYPE, 'tr_TR.UTF-8');\n$lower = strtolower($name);",
    after: "$lower = mb_strtolower(str_replace(['I', 'İ'], ['ı', 'i'], $name));",
  },
  "deprecated.oci8_alias": {
    steps: {
      tr: [
        "OCI8'in eski `oci*` adları kullanımdan kalktı; Ferry bunları `oci_*` adlarına kendisi çevirir.",
        "`ocifetchinto()`'nun karşılığı yok. Ferry onu OCI8'in yaptığını yapan bir kapanışla `oci_fetch_array()`'e çevirir: mod `OCI_ASSOC` ya da `OCI_NUM` içermiyorsa `OCI_NUM` eklenir, satır yalnızca okuma başarılıysa değişir, sonuç sütun sayısıdır.",
        "Elle yazarken çoğu döngü için `while ($row = oci_fetch_array($stmt, OCI_ASSOC + OCI_RETURN_NULLS))` yeterlidir. Döngüden sonraki kod `$row`'un son satırı tuttuğuna güveniyorsa okumayı ayrı bir değişkene alın.",
      ],
      en: [
        "OCI8's old `oci*` names are deprecated; Ferry renames them to the `oci_*` names itself.",
        "`ocifetchinto()` has no counterpart. Ferry turns it into `oci_fetch_array()` through a closure that does what OCI8 did: `OCI_NUM` is added when the mode has neither `OCI_ASSOC` nor `OCI_NUM`, the row changes only on a successful fetch, and the result is the column count.",
        "By hand, `while ($row = oci_fetch_array($stmt, OCI_ASSOC + OCI_RETURN_NULLS))` is enough for most loops. If code after the loop relies on `$row` holding the last row, fetch into a separate variable.",
      ],
    },
    before: "while (ocifetchinto($stmt, $row, OCI_ASSOC)) {",
    after: "while ($row = oci_fetch_array($stmt, OCI_ASSOC)) {",
  },
  "behavior.compact_undefined_variable": {
    steps: {
      tr: [
        "PHP 7.3'ten beri `compact()` tanımsız bir değişken adında uyarı verir. Sonuç değişmez: o ad dizide yer almaz.",
        "Fonksiyonda hiç geçmeyen adları listeden çıkarın; Ferry bunu kendisi yapar ve hiçbir ad kalmazsa boş dizi yazar.",
        "Ad yalnızca bazı yollarda tanımlanıyorsa değişkeni başta tanımlayın; o zaman anahtar dizide o değerle yer alır, bunun kodu etkileyip etkilemediğine bakın.",
      ],
      en: [
        "Since PHP 7.3, `compact()` warns about an undefined variable name. The result doesn't change: the name is left out of the array.",
        "Remove the names the function never has; Ferry does this itself and writes an empty array when no name is left.",
        "If the name is set on some paths only, define the variable up front; the key then appears in the array with that value, so check whether that affects the code.",
      ],
    },
    before: "$this->wheres[] = compact('type', 'operator', 'query', 'boolean');",
    after: "$this->wheres[] = compact('type', 'query', 'boolean');",
  },
  "deprecated.final_private_method": {
    steps: {
      tr: [
        "PHP 8.0'dan beri private metotlar alt sınıfta ezilmiş sayılmaz, bu yüzden `final` onlarda bir şey yapmaz ve uyarı verir. Kurucu metot (`__construct`) bunun dışındadır.",
        "`final` sözcüğünü silin; Ferry bunu kendisi yapar. Alt sınıflar artık aynı adla kendi private metodunu tanımlayabilir; PHP 7'de bu derleme hatasıydı.",
      ],
      en: [
        "Since PHP 8.0, private methods aren't considered overridden in subclasses, so `final` does nothing on them and warns. The constructor (`__construct`) is the exception.",
        "Remove the `final` keyword; Ferry does this itself. Subclasses can now declare their own private method with the same name, which was a compile error on PHP 7.",
      ],
    },
    before: "final private function hashToken($token)",
    after: "private function hashToken($token)",
  },
  "removed.ldap_control_paged_result_response": {
    steps: {
      tr: [
        "PHP 8.0'da `ldap_control_paged_result()` ve `ldap_control_paged_result_response()` kaldırıldı. Sayfalama denetimi artık `LDAP_CONTROL_PAGEDRESULTS` ile verilir ve okunur.",
        "İsteği `ldap_set_option($link, LDAP_OPT_SERVER_CONTROLS, [['oid' => LDAP_CONTROL_PAGEDRESULTS, 'value' => ['size' => 100, 'cookie' => $cookie]]])` ile verin, yanıtı `ldap_parse_result($link, $result, $code, $dn, $message, $referrals, $controls)` ile okuyun; çerez `$controls[LDAP_CONTROL_PAGEDRESULTS]['value']['cookie']`'dedir. Ferry iki tarafı da böyle yazar.",
        "Ferry argümanları basit değişken olmayan çağrılara dokunmaz; onları aynı kalıpla elle yazın. Bu kalıpla OpenLDAP'ta 25 kayıt 10'arlık 3 sayfada eksiksiz geldi.",
      ],
      en: [
        "PHP 8.0 removed `ldap_control_paged_result()` and `ldap_control_paged_result_response()`. The paging control is now set and read through `LDAP_CONTROL_PAGEDRESULTS`.",
        "Send the request with `ldap_set_option($link, LDAP_OPT_SERVER_CONTROLS, [['oid' => LDAP_CONTROL_PAGEDRESULTS, 'value' => ['size' => 100, 'cookie' => $cookie]]])` and read the response with `ldap_parse_result($link, $result, $code, $dn, $message, $referrals, $controls)`; the cookie is in `$controls[LDAP_CONTROL_PAGEDRESULTS]['value']['cookie']`. Ferry writes both sides this way.",
        "Ferry leaves calls whose arguments aren't plain variables; write those by hand with the same pattern. With it, 25 entries came back complete in 3 pages of 10 on OpenLDAP.",
      ],
    },
    before: "ldap_control_paged_result_response($link, $result, $cookie);",
    after: "ldap_parse_result($link, $result, $code, $dn, $message, $referrals, $controls);\n$cookie = $controls[LDAP_CONTROL_PAGEDRESULTS]['value']['cookie'] ?? '';",
  },
  "deprecated.builtin_function": {
    steps: {
      tr: [
        "Bu fonksiyon hedef sürümde kullanımdan kalktı: hâlâ çalışır ama uyarı verir ve sonraki ana sürümde kaldırılabilir. Bulgunun önerilen satırı varsa yerine geçen çağrıyı gösterir.",
        "Yalnızca bir kaynağı kapatan fonksiyonlar (`enchant_broker_free()`, `curl_close()`, `imagedestroy()` gibi) PHP 8'den beri nesneyle çalışır; nesne son referansı gidince kendiliğinden kapanır. Çağrıyı silin ya da yerel değişkeni hemen bırakmak için `unset()` yazın.",
        "Fonksiyonun bir karşılığı yoksa onu kullanan özelliği hedef sürümde neyin karşıladığını PHP kılavuzundaki fonksiyon sayfasında arayın.",
      ],
      en: [
        "This function is deprecated on the target version: it still runs but warns, and a later major version may remove it. The finding's suggested line shows the replacement when there is one.",
        "Functions that only close a resource (such as `enchant_broker_free()`, `curl_close()` and `imagedestroy()`) work on objects since PHP 8, which close on their own when the last reference goes. Remove the call, or write `unset()` to drop a local variable right away.",
        "If the function has no counterpart, look for what replaces the feature on the function's page in the PHP manual.",
      ],
    },
    before: "enchant_broker_free_dict($dict);",
    after: "unset($dict);",
  },
  "deprecated.session_set_save_handler": {
    steps: {
      tr: [
        "Altı ayrı çağrılabilirle `session_set_save_handler()` çağırmak PHP 8.4'te kullanımdan kalktı; `SessionHandlerInterface` uygulayan bir nesne verilmeli.",
        "İşleyici zaten bir sınıfın metotlarıysa o sınıfa `implements SessionHandlerInterface` ekleyip nesneyi verin: `session_set_save_handler($handler, false)`. Eski biçim kapanışta oturumu kendisi kaydetmediği için ikinci argüman false olmalıdır.",
        "Ferry altı çağrılabiliri onları sırayla çağıran anonim bir sınıfa sarar; metotlar public olmalıdır. Kapanışlar, fonksiyon adları ve `[$nesne, 'metot']` biçimindeki çağrılabilirler için bunu kendisi yapar.",
      ],
      en: [
        "Calling `session_set_save_handler()` with six separate callables was deprecated in PHP 8.4; pass an object that implements `SessionHandlerInterface`.",
        "If the handler already is a class's methods, add `implements SessionHandlerInterface` to that class and pass the object: `session_set_save_handler($handler, false)`. The second argument is false because the old form didn't register a shutdown save.",
        "Ferry wraps the six callables in an anonymous class that calls them in turn; the methods have to be public. It does this itself for closures, function names and `[$object, 'method']` callables.",
      ],
    },
    before: "session_set_save_handler([$h, 'open'], [$h, 'close'], [$h, 'read'], [$h, 'write'], [$h, 'destroy'], [$h, 'gc']);",
    after: "class DbSession implements SessionHandlerInterface\n{\n    // open, close, read, write, destroy, gc\n}\nsession_set_save_handler($h, false);",
  },
  "removed.error_handler_context": {
    steps: {
      tr: [
        "PHP 8.0 hata işleyicisine beşinci argümanı (`$errcontext`) artık vermiyor. Bu parametre zorunluysa işleyici her hatada ArgumentCountError fırlatır.",
        "İşleyici parametreyi kullanmıyorsa silin, kullanıyorsa varsayılan değer verin: `$errcontext = array()`. İşleyici aynı dosyadaysa Ferry ikisini de kendisi yapar.",
        "Kod değişkenlerin değerlerine ihtiyaç duyuyorsa (hata kaydına eklemek gibi) bu bilgi PHP 8'de yok; çağrı zincirini ve argümanları `debug_backtrace()` verir.",
      ],
      en: [
        "PHP 8.0 no longer passes the fifth argument (`$errcontext`) to error handlers. If the parameter is required, the handler throws ArgumentCountError on every error.",
        "Remove the parameter if the handler doesn't use it, or give it a default if it does: `$errcontext = array()`. When the handler is in the same file, Ferry does both itself.",
        "If the code needs the variables' values (to add them to a log, say), PHP 8 doesn't provide them; `debug_backtrace()` gives the call chain and its arguments.",
      ],
    },
    before: "function handler($no, $str, $file, $line, $context) {",
    after: "function handler($no, $str, $file, $line, $context = array()) {",
  },
  "behavior.pdo_default_exceptions": {
    steps: {
      tr: [
        "PHP 8.0'dan beri PDO'nun varsayılan hata modu `PDO::ERRMODE_EXCEPTION`'dır: başarısız sorgu false döndürmek yerine `PDOException` fırlatır ve bu `=== false` kontrolü hiç çalışmaz.",
        "Eski davranışı korumak için bağlantıyı kurarken `PDO::ATTR_ERRMODE => PDO::ERRMODE_SILENT` verin ya da hemen ardından `$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_SILENT)` çağırın. Bu o bağlantıdaki bütün çağrıları etkiler.",
        "Ya da çağrıyı `try { ... } catch (PDOException $e) { ... }` içine alın; hata bilgisi `$e->getMessage()` ve `$e->errorInfo`'dadır.",
      ],
      en: [
        "Since PHP 8.0, PDO's default error mode is `PDO::ERRMODE_EXCEPTION`: a failed query throws `PDOException` instead of returning false, so this `=== false` check never runs.",
        "To keep the old behaviour, pass `PDO::ATTR_ERRMODE => PDO::ERRMODE_SILENT` when connecting, or call `$pdo->setAttribute(PDO::ATTR_ERRMODE, PDO::ERRMODE_SILENT)` right after. This affects every call on that connection.",
        "Or wrap the call in `try { ... } catch (PDOException $e) { ... }`; the error details are in `$e->getMessage()` and `$e->errorInfo`.",
      ],
    },
    before: "if ($pdo->exec($sql) === false) {\n    log_error($pdo->errorInfo());\n}",
    after: "try {\n    $pdo->exec($sql);\n} catch (PDOException $e) {\n    log_error($e->errorInfo);\n}",
  },
  "behavior.indirect_access_order": {
    steps: {
      tr: [
        "PHP 7 değişken değişkenleri soldan sağa okur: `$obj->$f[1]()` PHP 5'te `$obj->{$f[1]}()`, PHP 7'den beri `($obj->$f)[1]()` demektir.",
        "Proje bugün PHP 5'te çalışıyorsa Ferry PHP 5 okumasını süslü parantezle yazar. Sürüm bilinmiyorsa bunu seçenek olarak sunar; `$f` bir diziyse önerilir.",
        "Kod PHP 7'de zaten çalışıyorsa okuma değişmemiştir; dokunmayın.",
      ],
      en: [
        "PHP 7 reads variable variables left to right: `$obj->$f[1]()` meant `$obj->{$f[1]}()` on PHP 5 and means `($obj->$f)[1]()` since PHP 7.",
        "When the project runs PHP 5 today, Ferry writes the PHP 5 reading with braces. When the version is unknown it offers it as an option, recommended when `$f` is an array.",
        "If the code already runs on PHP 7, its reading hasn't changed; leave it.",
      ],
    },
    before: "return $plugin->$function[1]($args);",
    after: "return $plugin->{$function[1]}($args);",
  },
  "deprecated.null_offset": {
    steps: {
      tr: [
        "PHP 8.5'ten beri dizi anahtarı olarak `null` kullanmak kullanımdan kalktı. Dizilerde `null` anahtarı her zaman `''`'e çevriliyordu.",
        "Değişken kesin bir diziyse `[null]` yerine `['']` yazın; sonuç aynıdır. Ferry bunu dizinin hemen önce oluşturulduğu yerlerde kendisi yapar.",
        "Değer `ArrayAccess` uygulayan bir nesneyse `null` olduğu gibi `offsetGet()`'e gider; `''` vermek davranışı değiştirebilir. Nesnenin `null`'u nasıl ele aldığına bakıp ona göre karar verin.",
      ],
      en: [
        "Since PHP 8.5, using `null` as an array key is deprecated. On arrays, a `null` key always became `''`.",
        "If the variable is surely an array, write `['']` instead of `[null]`; the result is the same. Ferry does this itself where the array was created just before.",
        "If the value is an object implementing `ArrayAccess`, `null` reaches `offsetGet()` as it is, and `''` may change what happens. Check how the object handles `null` and decide from that.",
      ],
    },
    before: "unset($this->domains[$domain][null]);",
    after: "unset($this->domains[$domain]['']);",
  },
  "removed.__autoload": {
    steps: {
      tr: [
        "PHP 8.0'da `__autoload()` kaldırıldı; bu adla fonksiyon tanımlamak derleme hatasıdır.",
        "Proje başka bir yükleyici kaydetmiyorsa Ferry fonksiyonu `spl_autoload_register(function ($class) { ... })` biçimine çevirir.",
        "Proje kendi yükleyicisini `spl_autoload_register()` ya da Composer'ın `autoload.php`'siyle kuruyorsa PHP 7 `__autoload()`'ı zaten yok sayıyordu; Ferry fonksiyonu siler. Kod `__autoload()`'ı adıyla çağırıyorsa ya da `spl_autoload_register('__autoload')` ile kaydediyorsa fonksiyonu yeniden adlandırıp o yerleri de değiştirin.",
      ],
      en: [
        "PHP 8.0 removed `__autoload()`; declaring a function with that name is a compile error.",
        "When the project registers no other autoloader, Ferry turns the function into `spl_autoload_register(function ($class) { ... })`.",
        "When the project sets up its own autoloader with `spl_autoload_register()` or Composer's `autoload.php`, PHP 7 already ignored `__autoload()`, so Ferry removes it. If code calls `__autoload()` by name or registers it with `spl_autoload_register('__autoload')`, rename the function and change those places too.",
      ],
    },
    before: "function __autoload($class) {\n    require 'classes/' . $class . '.php';\n}",
    after: "spl_autoload_register(function ($class) {\n    require 'classes/' . $class . '.php';\n});",
  },
  "deprecated.builtin_method": {
    steps: {
      tr: [
        "Bu metot hedef sürümde kullanımdan kalktı; bulgunun önerilen satırı varsa yerine geçeni gösterir.",
        "PHP 8.5'te `SplObjectStorage::attach()`, `detach()` ve `contains()` yerine aynı işi yapan `offsetSet()`, `offsetUnset()` ve `offsetExists()` kullanılır. Ferry `parent::` çağrılarını kendisi çevirir.",
        "`ReflectionParameter::getClass()` PHP 8.0'da kullanımdan kalktı. Türü okuyun: tür `ReflectionNamedType` ve yerleşik değilse `new ReflectionClass($type->getName())`. Tür `self` ya da `parent` ise adı bildiren sınıftan çözün.",
      ],
      en: [
        "This method is deprecated on the target version; the finding's suggested line shows the replacement when there is one.",
        "On PHP 8.5, `SplObjectStorage::attach()`, `detach()` and `contains()` give way to `offsetSet()`, `offsetUnset()` and `offsetExists()`, which do the same. Ferry converts `parent::` calls itself.",
        "`ReflectionParameter::getClass()` was deprecated in PHP 8.0. Read the type instead: when it's a `ReflectionNamedType` and not built in, `new ReflectionClass($type->getName())`. Resolve `self` and `parent` from the declaring class.",
      ],
    },
    before: "$class = $param->getClass();",
    after: "$type = $param->getType();\n$class = $type instanceof ReflectionNamedType && !$type->isBuiltin() ? new ReflectionClass($type->getName()) : null;",
  },
  "deprecated.partial_callable": {
    steps: {
      tr: [
        "PHP 8.2'de `'self::m'`, `'parent::m'`, `'static::m'` ve `['self', 'm']` gibi çağrılabilirler kullanımdan kalktı.",
        "Hemen çalıştırılan çağrılarda Ferry bunları `self::m(...)` biçimine, `is_callable()` içindekileri `parent::class . '::m'` biçimine çevirir; ikisi de aynı kapsamda aynı sonucu verir.",
        "Çağrılabilir saklanıp başka yerde çalıştırılıyorsa sınıf adını açıkça yazın: `[self::class, 'm']` ya da nesne metodu için `[$this, 'm']`.",
      ],
      en: [
        "PHP 8.2 deprecated callables such as `'self::m'`, `'parent::m'`, `'static::m'` and `['self', 'm']`.",
        "For calls that run them right away, Ferry writes `self::m(...)`, and inside `is_callable()` it writes `parent::class . '::m'`; both give the same result from the same scope.",
        "When the callable is stored and run somewhere else, write the class out: `[self::class, 'm']`, or `[$this, 'm']` for an instance method.",
      ],
    },
    before: "set_error_handler('self::handle');",
    after: "set_error_handler([self::class, 'handle']);",
  },
  "removed.recode": {
    steps: {
      tr: [
        "recode eklentisi PHP 7.4'te kaldırıldı.",
        "Ferry istek metni sabit ve bilinen bir karakter kümesi çiftiyse çağrıyı `iconv()`'a çevirir. İstek bir değişkenden geliyorsa `iconv($from, $to, $text)` ya da `mb_convert_encoding($text, $to, $from)` kullanın.",
        "iconv dönüştüremediği bir karakterde false döndürür; `//TRANSLIT` ya da `//IGNORE` eki bunu değiştirir. Karakter kümesi adlarını hedef sunucuda deneyin.",
      ],
      en: [
        "PHP 7.4 removed the recode extension.",
        "When the request is a fixed, known pair of character sets, Ferry turns the call into `iconv()`. When it comes from a variable, use `iconv($from, $to, $text)` or `mb_convert_encoding($text, $to, $from)`.",
        "iconv returns false on a character it can't convert; the `//TRANSLIT` or `//IGNORE` suffix changes that. Try the character set names on the target server.",
      ],
    },
    before: "$out = recode_string($from . '..utf-8', $text);",
    after: "$out = iconv($from, 'UTF-8', $text);",
  },
  "behavior.openssl_default_cipher": {
    steps: {
      tr: [
        "PHP 8.1'den beri `openssl_pkcs7_encrypt()` ve `openssl_cms_encrypt()` şifre verilmezse RC2-40 yerine AES-128-CBC kullanır. Çok eski alıcılar AES'i açamayabilir.",
        "Alıcılar AES'i destekliyorsa değişiklik gerekmez; varsayılanı açıkça yazmak için son argüman olarak `OPENSSL_CIPHER_AES_128_CBC` verin.",
        "RC2-40'a bağlı bir alıcı için `OPENSSL_CIPHER_RC2_40` verilebilir, ama RC2-40 zayıftır ve OpenSSL 3 RC2'yi varsayılan olarak kapattığı için çağrı başarısız olabilir; alıcıyı güncellemek daha doğrudur.",
      ],
      en: [
        "Since PHP 8.1, `openssl_pkcs7_encrypt()` and `openssl_cms_encrypt()` use AES-128-CBC instead of RC2-40 when no cipher is given. Very old recipients may not open AES.",
        "If the recipients support AES, nothing needs to change; to write the default out, pass `OPENSSL_CIPHER_AES_128_CBC` as the last argument.",
        "A recipient tied to RC2-40 could get `OPENSSL_CIPHER_RC2_40`, but RC2-40 is weak and OpenSSL 3 disables RC2 by default, so the call may fail; updating the recipient is the better fix.",
      ],
    },
    before: "openssl_pkcs7_encrypt($in, $out, $cert, $headers);",
    after: "openssl_pkcs7_encrypt($in, $out, $cert, $headers, 0, OPENSSL_CIPHER_AES_128_CBC);",
  },
  "removed.redeclared_builtin": {
    steps: {
      tr: [
        "Bu ad hedef PHP'de yerleşik bir fonksiyona ya da sınıfa ait; aynı adla tanım yapmak `Cannot redeclare` ya da `Cannot declare class` hatası verir.",
        "Fonksiyonsa ve aynı işi yapıyorsa tanımı `if (!function_exists('ad')) { ... }` içine alın; Ferry bunu kendisi yapar. O zaman hedef sürümde PHP'nin kendi fonksiyonu çalışır, sonuçların aynı olduğundan emin olun.",
        "Sınıfsa (PHP 8.0'daki `XMLParser` gibi) onu kullanıldığı her yerle birlikte yeniden adlandırın ya da bir namespace içine taşıyın. Rector'un `RenameClassRector` kuralı bütün kullanımları birlikte değiştirebilir.",
      ],
      en: [
        "This name belongs to a built-in function or class on the target PHP; declaring it again fails with `Cannot redeclare` or `Cannot declare class`.",
        "If it's a function that does the same job, wrap the declaration in `if (!function_exists('name')) { ... }`; Ferry does this itself. PHP's own function then runs on the target version, so make sure the results match.",
        "If it's a class (such as `XMLParser` in PHP 8.0), rename it together with every place that uses it, or move it into a namespace. Rector's `RenameClassRector` can change all the uses at once.",
      ],
    },
    before: "class XMLParser {",
    after: "class LegacyXmlParser {",
  },
  "removed.magic_method_signature": {
    steps: {
      tr: [
        "PHP 8.0'dan beri sihirli metotların imzası denetlenir: `__set_state()` statik olmalı ve bir dizi almalı, `__sleep()`, `__wakeup()` ve `__serialize()` argüman almaz, `__unserialize()` bir dizi alır. Uymayan sınıf derlenmez.",
        "Parametreleri ve `static` sözcüğünü bu imzaya uydurun.",
        "Gövde imzayla birlikte çalışamıyorsa (statik olması gereken `__set_state()` içinde `$this` kullanmak gibi) metot sihirli metot olarak zaten hiç çalışmıyordu. Onu başka bir adla yeniden adlandırıp çağrıldığı yerleri değiştirin.",
      ],
      en: [
        "Since PHP 8.0, magic method signatures are checked: `__set_state()` must be static and take an array, `__sleep()`, `__wakeup()` and `__serialize()` take no arguments, and `__unserialize()` takes an array. A class that doesn't match doesn't compile.",
        "Make the parameters and the `static` keyword match that signature.",
        "If the body can't work with the signature (such as `$this` inside a `__set_state()` that must be static), the method never worked as a magic method anyway. Rename it and change the places that call it.",
      ],
    },
    before: "public function __set_state() {\n    return $this->get_string();\n}",
    after: "public function export_string() {\n    return $this->get_string();\n}",
  },
  "deprecated.trait_static_access": {
    steps: {
      tr: [
        "PHP 8.1'den beri bir trait'in statik üyesine trait adıyla doğrudan erişmek (`Trait::metot()`, `Trait::$x`) kullanımdan kalktı.",
        "Trait'i kullanan bir sınıfın adıyla erişin: `Sinif::metot()`. Kod trait'in kendi içindeyse `static::` ya da `self::` kullanın.",
        "Trait'i kullanan uygun bir sınıf yoksa metodu bir sınıfa ya da düz bir fonksiyona taşıyıp trait'in onu çağırmasını sağlayın.",
      ],
      en: [
        "Since PHP 8.1, reaching a trait's static member through the trait's name (`Trait::method()`, `Trait::$x`) is deprecated.",
        "Go through a class that uses the trait: `SomeClass::method()`. Inside the trait itself, use `static::` or `self::`.",
        "If no class that uses the trait fits, move the method into a class or a plain function and have the trait call that.",
      ],
    },
    before: "$themes = SkinThemesTrait::getSkinThemeMap();",
    after: "$themes = SkinModule::getSkinThemeMap();",
  },
  "deprecated.filter_sanitize_string": {
    steps: {
      tr: [
        "PHP 8.1'de `FILTER_SANITIZE_STRING` ve `FILTER_SANITIZE_STRIPPED` kullanımdan kalktı. Filtre etiketleri siliyor, tek ve çift tırnağı `&#39;` ile `&#34;` olarak kodluyordu.",
        "Tek değerde Ferry seçenek sunar. `filter_var_array()` ya da `filter_input_array()` gibi dizi çağrılarında aynı dönüşümü her elemana uygulayın: `str_replace([\"'\", '\"'], ['&#39;', '&#34;'], strip_tags($v))`.",
        "Değer HTML'e yazılacaksa çıkışta `htmlspecialchars()` kullanmak daha güvenlidir; o zaman `&`, `<` ve `>` de kodlanır.",
      ],
      en: [
        "PHP 8.1 deprecated `FILTER_SANITIZE_STRING` and `FILTER_SANITIZE_STRIPPED`. The filter stripped tags and encoded single and double quotes as `&#39;` and `&#34;`.",
        "For a single value, Ferry offers options. For array calls such as `filter_var_array()` and `filter_input_array()`, apply the same conversion to each element: `str_replace([\"'\", '\"'], ['&#39;', '&#34;'], strip_tags($v))`.",
        "If the value ends up in HTML, `htmlspecialchars()` on output is safer; it also encodes `&`, `<` and `>`.",
      ],
    },
    before: "$clean = filter_var_array($input, FILTER_SANITIZE_STRING);",
    after: "array_walk_recursive($input, function (&$v) {\n    $v = str_replace([\"'\", '\"'], ['&#39;', '&#34;'], strip_tags($v));\n});",
  },
  "deprecated.openssl_free_key": {
    steps: {
      tr: [
        "PHP 8.0'dan beri OpenSSL anahtarları nesnedir ve son referans gidince kendiliğinden serbest kalır; `openssl_free_key()` hiçbir şey yapmaz ve kullanımdan kalktı.",
        "Çağrıyı silin. Ferry tek başına duran çağrıları kendisi siler; çağrı süslü parantezsiz bir `if` gövdesiyse `if`'i de silin.",
        "Anahtarı hemen bırakmak istiyorsanız değişkene null atayın ya da `unset()` kullanın.",
      ],
      en: [
        "Since PHP 8.0, OpenSSL keys are objects that free themselves when the last reference goes; `openssl_free_key()` does nothing and is deprecated.",
        "Remove the call. Ferry removes calls that stand alone; when the call is the body of an `if` without braces, remove the `if` too.",
        "To let go of the key right away, set the variable to null or `unset()` it.",
      ],
    },
    before: "if ($key !== false)\n    openssl_free_key($key);",
    after: "$key = null;",
  },
  "server.composer_plugin_api": {
    steps: {
      tr: [
        "Bu Composer eklentisi yalnızca Composer 1'in eklenti arayüzünü (`composer-plugin-api` 1.x) destekliyor; Composer 2 onu yüklemez.",
        "Önerilen satırdaki `composer update` komutuyla eklentiyi Composer 2'yi destekleyen sürüme yükseltin. Yeni sürümü yoksa ve projenin ona ihtiyacı kalmadıysa `composer remove` ile kaldırın.",
        "Composer 2.2'den beri eklentilerin `composer.json`'daki `config.allow-plugins` listesinde izinli olması gerekir; güncellemeden sonra Composer sorarsa izin verin.",
      ],
      en: [
        "This Composer plugin supports only Composer 1's plugin API (`composer-plugin-api` 1.x); Composer 2 won't load it.",
        "Update it to a release that supports Composer 2 with the `composer update` command in the suggested line. If there is none and the project no longer needs it, remove it with `composer remove`.",
        "Since Composer 2.2, plugins have to be allowed in `config.allow-plugins` in `composer.json`; allow it if Composer asks after the update.",
      ],
    },
  },
  "deprecated.constant_redeclared": {
    steps: {
      tr: [
        "Aynı sabit aynı dosyada ikinci kez tanımlanıyor. İkinci tanım her zaman uyarı verip yok sayıldı; PHP 8.5'ten beri kullanımdan kalktı.",
        "İkinci tanımı silin ya da `if (!defined('AD')) { ... }` içine alın. Kod ikinci değere güveniyorsa o değer hiç kullanılmadı demektir; sabiti yeniden adlandırın.",
      ],
      en: [
        "The same constant is declared a second time in this file. The second declaration always warned and was ignored; since PHP 8.5 it's deprecated.",
        "Remove the second declaration or wrap it in `if (!defined('NAME')) { ... }`. If code relies on the second value, that value was never used; rename the constant.",
      ],
    },
    before: "define('CACHE_DIR', '/tmp/a');\ndefine('CACHE_DIR', '/tmp/b');",
    after: "define('CACHE_DIR', '/tmp/a');",
  },
  "removed.builtin_override_signature": {
    steps: {
      tr: [
        "Yerleşik bir sınıfın metodunu ezen bu metodun imzası hedef sürümdekiyle uyuşmuyor; sınıf derlenmez.",
        "`SoapClient::__doRequest()` PHP 8.5'te altıncı bir parametre aldı; sonuna varsayılanı null olan bir parametre ekleyin. `SplFileObject::fwrite()`'ın `$length`'i PHP 8.5'te `?int` oldu. Ferry bu ikisini kendisi düzeltir.",
        "`tidy::repairString()` ve `repairFile()` PHP 8.0'da statik oldu; ezen metot da `static` olmalı. Gövde `$this` kullanıyorsa metodu başka bir adla yeniden adlandırın.",
      ],
      en: [
        "This method overrides a built-in class's method with a signature that doesn't match the target version's, so the class doesn't compile.",
        "`SoapClient::__doRequest()` gained a sixth parameter in PHP 8.5; add one at the end with a null default. `SplFileObject::fwrite()`'s `$length` became `?int` in PHP 8.5. Ferry fixes both itself.",
        "`tidy::repairString()` and `repairFile()` became static in PHP 8.0; the overriding method must be `static` too. If its body uses `$this`, rename the method.",
      ],
    },
    before: "function __doRequest($request, $location, $action, $version, $one_way = 0)",
    after: "function __doRequest($request, $location, $action, $version, $one_way = 0, $uriParserClass = null)",
  },
  "framework.drupal_version": {
    steps: {
      tr: [
        "Yedek alın ve güncellemeyi önce bir kopyada deneyin.",
        "Drupal 8 ve sonrası Composer ile güncellenir: `composer update \"drupal/core-*\" --with-all-dependencies`, ardından `drush updatedb` ve `drush cache:rebuild`.",
        "Drupal 7 yerinde yeni bir ana sürüme yükseltilemez; Drupal 10 ya da 11'e geçiş Migrate modülleriyle yapılan bir taşımadır. O zamana kadar bulgunun gösterdiği Drupal 7 sürümüne güncelleyin.",
        "Modülleri ve temaları da güncelleyin, sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup and try the update on a copy first.",
        "Drupal 8 and later update through Composer: `composer update \"drupal/core-*\" --with-all-dependencies`, then `drush updatedb` and `drush cache:rebuild`.",
        "Drupal 7 can't be upgraded in place to a new major version; moving to Drupal 10 or 11 is a migration done with the Migrate modules. Until then, update to the Drupal 7 release the finding names.",
        "Update modules and themes too, then scan the project again.",
      ],
    },
  },
  "framework.joomla_version": {
    steps: {
      tr: [
        "Yedek alın, sonra Joomla'yı yönetim panelindeki Joomla Güncelleme (Joomla! Update) bileşeninden güncelleyin.",
        "Ana sürüm atlarken önce bulunduğunuz serinin son sürümüne çıkın (Joomla 4'e geçmek için 3.10, 5'e geçmek için 4.4); bu sürümler geçişten önce uyumluluk denetimini gösterir.",
        "Eklentileri ve şablonları hedef Joomla sürümüyle uyumlu olanlarla değiştirin, sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup, then update Joomla from the Joomla Update (Joomla! Update) component in the administrator.",
        "When skipping a major version, first move to the last release of your series (3.10 before Joomla 4, 4.4 before 5); those releases show the compatibility check before the jump.",
        "Replace extensions and templates with ones compatible with the target Joomla release, then scan the project again.",
      ],
    },
  },
  "framework.moodle_version": {
    steps: {
      tr: [
        "Veritabanını, `moodledata` klasörünü ve kodu yedekleyin; siteyi bakım moduna alın.",
        "Yeni Moodle sürümünün kodunu eskisinin yerine koyun; `config.php`'yi ve sonradan kurduğunuz eklentileri koruyun. Git kullanıyorsanız ilgili `MOODLE_XXX_STABLE` dalına geçin.",
        "Yükseltmeyi `php admin/cli/upgrade.php` ile çalıştırın ve bakım modunu kapatın.",
        "Her Moodle sürümü yalnızca belirli bir sürümden ve sonrasından yükseltilebilir; aradaki fark büyükse sürüm notlarına bakıp ara sürümlerden geçin.",
      ],
      en: [
        "Back up the database, the `moodledata` folder and the code; put the site in maintenance mode.",
        "Put the new Moodle release's code in place of the old; keep `config.php` and the plugins you added. With Git, switch to the matching `MOODLE_XXX_STABLE` branch.",
        "Run the upgrade with `php admin/cli/upgrade.php` and turn maintenance mode off.",
        "Each Moodle release can only be upgraded from a certain release onward; if the gap is large, check the release notes and go through the releases in between.",
      ],
    },
  },
  "framework.mediawiki_version": {
    steps: {
      tr: [
        "Veritabanını, `LocalSettings.php`'yi, `images` klasörünü ve uzantıları yedekleyin.",
        "Yeni MediaWiki sürümünü ayrı bir klasöre açın; `LocalSettings.php`'yi, `images` klasörünü ve uzantıları oraya taşıyın.",
        "Veritabanını `php maintenance/run.php update` ile güncelleyin; 1.40'tan eski sürümlerde komut `php maintenance/update.php`'dir.",
        "Uzantıları yeni sürümle uyumlu sürümlerine güncelleyin, sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Back up the database, `LocalSettings.php`, the `images` folder and the extensions.",
        "Unpack the new MediaWiki release into its own folder and move `LocalSettings.php`, the `images` folder and the extensions there.",
        "Update the database with `php maintenance/run.php update`; before 1.40 the command is `php maintenance/update.php`.",
        "Update the extensions to releases that work with the new version, then scan the project again.",
      ],
    },
  },
  "framework.phpbb_version": {
    steps: {
      tr: [
        "Yedek alın; eklentileri (extensions) kapatın ve varsayılan stile geçin.",
        "phpBB'nin sitesindeki otomatik güncelleme paketini yükleyin ve güncellemeyi tarayıcıda `install/app.php/update` adresinden çalıştırın. Tam paketle güncellerken `config.php`'yi, `files`, `images` ve `store` klasörlerini, kendi eklenti ve stillerinizi koruyun.",
        "Bitince `install` klasörünü silin, eklentileri yeniden açın ve projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup; disable extensions and switch to the default style.",
        "Upload the automatic update package from phpBB's site and run the update in the browser at `install/app.php/update`. When updating with the full package, keep `config.php`, the `files`, `images` and `store` folders, and your own extensions and styles.",
        "Delete the `install` folder when done, re-enable extensions and scan the project again.",
      ],
    },
  },
  "framework.prestashop_version": {
    steps: {
      tr: [
        "Yedek alın ve siteyi bakım moduna alın.",
        "PrestaShop'un resmî yükseltme modülü `autoupgrade` (1-Click Upgrade) dosyaları ve veritabanını birlikte yükseltir. Büyük geçişlerden önce (1.6'dan 1.7'ye, 1.7'den 8'e) bulunduğunuz serinin son sürümüne çıkın.",
        "Modülleri ve temayı hedef sürümle uyumlu olanlarla değiştirin; 1.6 temaları 1.7 ve sonrasında çalışmaz. Sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup and put the shop in maintenance mode.",
        "PrestaShop's official upgrade module `autoupgrade` (1-Click Upgrade) upgrades the files and the database together. Before a big jump (1.6 to 1.7, 1.7 to 8), move to the last release of your series first.",
        "Replace modules and the theme with ones compatible with the target release; 1.6 themes don't work on 1.7 and later. Then scan the project again.",
      ],
    },
  },
  "framework.magento_version": {
    steps: {
      tr: [
        "Magento 1'in desteği Haziran 2020'de bitti ve PHP 8 için resmî bir sürümü yok. Topluluğun sürdürdüğü OpenMage (Magento LTS) Magento 1 veritabanıyla çalışır ve PHP 8'i destekler; diğer yol Magento 2'ye taşınmaktır.",
        "Magento 2'de sürümü Composer ile yükseltin: `composer require magento/product-community-edition=<sürüm> --no-update`, `composer update`, ardından `bin/magento setup:upgrade`.",
        "Eklentileri hedef sürümle uyumlu olanlarla değiştirin ve projeyi yeniden tarayın.",
      ],
      en: [
        "Magento 1 support ended in June 2020 and it has no official release for PHP 8. The community-maintained OpenMage (Magento LTS) works with a Magento 1 database and supports PHP 8; the other way is moving to Magento 2.",
        "On Magento 2, upgrade with Composer: `composer require magento/product-community-edition=<version> --no-update`, `composer update`, then `bin/magento setup:upgrade`.",
        "Replace extensions with ones compatible with the target release and scan the project again.",
      ],
    },
  },
  "framework.codeigniter_version": {
    steps: {
      tr: [
        "CodeIgniter 3'te `system` klasörünü en son 3.1.x sürümününkiyle değiştirin; `application` klasörünüz olduğu gibi kalır. Kullanım kılavuzundaki yükseltme notlarını sırayla uygulayın.",
        "CodeIgniter 4 ayrı bir çatıdır; 3'ten 4'e geçmek kodu yeniden yazmayı gerektirir.",
        "Güncelledikten sonra projeyi yeniden tarayın.",
      ],
      en: [
        "On CodeIgniter 3, replace the `system` folder with the latest 3.1.x release's; your `application` folder stays as it is. Apply the upgrade notes in the user guide in order.",
        "CodeIgniter 4 is a separate framework; moving from 3 to 4 means rewriting the code.",
        "Scan the project again after the update.",
      ],
    },
  },
  "framework.cakephp_version": {
    steps: {
      tr: [
        "Aynı ana sürüm içinde CakePHP'yi Composer ile güncelleyin: `composer update cakephp/cakephp`. Çatı `lib/Cake` klasöründeyse o klasörü yeni sürümünkiyle değiştirin.",
        "Ana sürüm atlarken CakePHP'nin geçiş kılavuzunu izleyin; 3'ten sonraki geçişlerde `cakephp/upgrade` aracı kodun çoğunu kendisi günceller. 2'den 3'e geçiş büyük ölçüde yeniden yazmadır.",
        "Eklentileri hedef sürümle uyumlu olanlara güncelleyin ve projeyi yeniden tarayın.",
      ],
      en: [
        "Within a major version, update CakePHP with Composer: `composer update cakephp/cakephp`. If the framework lives in `lib/Cake`, replace that folder with the new release's.",
        "When moving to a new major version, follow CakePHP's migration guide; from 3 onward the `cakephp/upgrade` tool updates most of the code itself. Moving from 2 to 3 is largely a rewrite.",
        "Update plugins to releases compatible with the target and scan the project again.",
      ],
    },
  },
  "framework.yii_version": {
    steps: {
      tr: [
        "Yii 2'yi Composer ile güncelleyin: `composer update yiisoft/yii2`; her sürüm için paketin `UPGRADE.md` dosyasındaki notları izleyin.",
        "Yii 1.1'de çatıyı `composer update yiisoft/yii` ile ya da `framework` klasörünü en son 1.1.x sürümününkiyle değiştirerek güncelleyin.",
        "Yii 1'den 2'ye geçiş yeniden yazmadır. Güncelledikten sonra projeyi yeniden tarayın.",
      ],
      en: [
        "Update Yii 2 with Composer: `composer update yiisoft/yii2`, following the notes in the package's `UPGRADE.md` for each release.",
        "On Yii 1.1, update the framework with `composer update yiisoft/yii` or by replacing the `framework` folder with the latest 1.1.x release's.",
        "Moving from Yii 1 to 2 is a rewrite. Scan the project again after the update.",
      ],
    },
  },
  "framework.symfony_version": {
    steps: {
      tr: [
        "`composer.json`'daki `symfony/*` kısıtlarını ve varsa `extra.symfony.require` değerini hedef sürüme çekip `composer update \"symfony/*\" --with-all-dependencies` çalıştırın.",
        "Ana sürüm atlamadan önce bulunduğunuz sürümdeki kullanımdan kaldırma uyarılarını giderin; değişiklikler Symfony'nin `UPGRADE-X.Y.md` dosyalarında listelenir.",
        "Testleri çalıştırın ve projeyi yeniden tarayın.",
      ],
      en: [
        "Raise the `symfony/*` constraints in `composer.json`, and `extra.symfony.require` if present, to the target release, then run `composer update \"symfony/*\" --with-all-dependencies`.",
        "Before a major jump, clear the deprecation warnings of your current release; Symfony's `UPGRADE-X.Y.md` files list the changes.",
        "Run the tests and scan the project again.",
      ],
    },
  },
  "framework.opencart_version": {
    steps: {
      tr: [
        "Yedek alın. Yeni OpenCart sürümünün dosyalarını yükleyin; `config.php` ve `admin/config.php` dosyalarınızı koruyun.",
        "Tarayıcıda `install` klasöründeki yükseltme adımlarını çalıştırın, bitince `install` klasörünü silin.",
        "OpenCart 3 eklentileri ve temaları 4'te çalışmaz; uyumlu sürümlerini kurun ve projeyi yeniden tarayın.",
      ],
      en: [
        "Take a backup. Upload the new OpenCart release's files, keeping your `config.php` and `admin/config.php`.",
        "Run the upgrade steps under the `install` folder in the browser, then delete the `install` folder.",
        "OpenCart 3 extensions and themes don't work on 4; install compatible versions and scan the project again.",
      ],
    },
  },
};
