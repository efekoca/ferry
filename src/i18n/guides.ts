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
        "İkinci argümanı `(array)` ile diziye çevirin: `array_key_exists($key, (array) $obj)`. Eski çağrının baktığı özellikleri verir; `ArrayObject`'te içindeki dizi dahil, sonuç PHP 5.6–7.4 ile aynı. `property_exists()` aynı şey değil, private ve protected özelliklerde de true döndürür.",
        "Anahtar sayı olabiliyorsa (`'0'`, `12` ya da sayı tutabilen bir değişken) sonucu kontrol edin: eski PHP nesnede sayısal anahtarı hiç bulmuyordu, `(array)` ile bulunur.",
      ],
      en: [
        "Turn the second argument into an array with `(array)`: `array_key_exists($key, (array) $obj)`. It holds the properties the old call searched, an `ArrayObject`'s storage included, so the result matches PHP 5.6–7.4. `property_exists()` is not the same: it also returns true for private and protected properties.",
        "If the key can be a number (`'0'`, `12` or a variable that may hold one), check the result: old PHP never found a numeric key in an object, and with `(array)` it is found.",
      ],
    },
    before: "return array_key_exists($index, $this);",
    after: "return array_key_exists($index, (array) $this);",
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
  "behavior.bind_column_lob": {
    steps: {
      tr: [
        "PHP 8.1'den beri `PDO::PARAM_LOB` ile bağlanan sütun her zaman bir akıştır (stream). Değeri doğrudan metin gibi kullanan kod (`strlen()`, `echo`, karşılaştırma) artık akış tutamacıyla çalışır.",
        "Değeri metin olarak okuyun: `$data = is_resource($lob) ? stream_get_contents($lob) : $lob;`. Bu satır iki davranışta da aynı metni verir.",
        "Değer küçükse ve akışa ihtiyaç yoksa sütunu `PDO::PARAM_STR` ile bağlamak da olur.",
      ],
      en: [
        "Since PHP 8.1 a column bound with `PDO::PARAM_LOB` is always a stream. Code that uses the value as a string directly (`strlen()`, `echo`, comparisons) now works on the stream handle.",
        "Read the value as a string: `$data = is_resource($lob) ? stream_get_contents($lob) : $lob;`. This line gives the same string under both behaviours.",
        "If the value is small and no stream is needed, binding the column with `PDO::PARAM_STR` works too.",
      ],
    },
    before: "$stmt->bindColumn(1, $lob, PDO::PARAM_LOB);\n$stmt->fetch(PDO::FETCH_BOUND);\necho strlen($lob);",
    after: "$stmt->bindColumn(1, $lob, PDO::PARAM_LOB);\n$stmt->fetch(PDO::FETCH_BOUND);\n$data = is_resource($lob) ? stream_get_contents($lob) : $lob;\necho strlen($data);",
  },
  "behavior.concat_precedence": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `+`, `-`, `<<` ve `>>`, `.`'dan önce hesaplanır. PHP 7'de `\"Toplam: \" . $a + $b` önce birleştirip sonra topluyordu; PHP 8'de önce toplar.",
        "Ne kastedildiğini parantezle yazın. Ferry eski sonucu veren parantezleri kendisi ekler; kalan durumlarda (aynı ifadede `&` ya da `|` varsa gibi) parantezi elle koyun.",
        "Eski sonuç çoğu zaman bir hataydı (metnin sayıya çevrilmesi). Kastedilen toplama işlemiyse `\"Toplam: \" . ($a + $b)` yazın.",
      ],
      en: [
        "Since PHP 8.0, `+`, `-`, `<<` and `>>` are evaluated before `.`. On PHP 7, `\"Total: \" . $a + $b` concatenated first and then added; PHP 8 adds first.",
        "Write what you mean with parentheses. Ferry adds the ones that keep the old result itself; in the remaining cases (when the same expression has `&` or `|`, say) add them by hand.",
        "The old result was usually a bug (the text turned into a number). If the sum was meant, write `\"Total: \" . ($a + $b)`.",
      ],
    },
    before: "echo \"Total: \" . $a + $b;",
    after: "echo \"Total: \" . ($a + $b);",
  },
  "behavior.curl_certinfo_format": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `curl_getinfo($ch, CURLINFO_CERTINFO)` sonucundaki `Subject` ve `Issuer` alanları dizi değil, `C = US, O = Örnek, CN = example.com` gibi metindir.",
        "Bu alanlardan bir değer okuyan kod (`$cert['Subject']['CN']` gibi) metni ayrıştırmalı ya da sertifikayı `openssl_x509_parse()` ile okumalıdır; o fonksiyon alanları dizi olarak verir.",
      ],
      en: [
        "Since PHP 7.0, the `Subject` and `Issuer` fields in the `curl_getinfo($ch, CURLINFO_CERTINFO)` result are strings such as `C = US, O = Example, CN = example.com`, not arrays.",
        "Code that reads a value from them (such as `$cert['Subject']['CN']`) has to parse the string, or read the certificate with `openssl_x509_parse()`, which returns the fields as arrays.",
      ],
    },
    before: "$cn = $info[0]['Subject']['CN'];",
    after: "$cn = openssl_x509_parse($info[0]['Cert'])['subject']['CN'];",
  },
  "behavior.filesystem_iterator_dots": {
    steps: {
      tr: [
        "PHP 8.2'den beri `FilesystemIterator`'a bayrak verildiğinde `SKIP_DOTS` kendiliğinden eklenmez; döngü `.` ve `..` girdilerini de görür. PHP 8.1'de aynı çağrı onları atlıyordu.",
        "Eski davranış için bayraklara `FilesystemIterator::SKIP_DOTS` ekleyin: `FilesystemIterator::KEY_AS_PATHNAME | FilesystemIterator::SKIP_DOTS`.",
      ],
      en: [
        "Since PHP 8.2, `SKIP_DOTS` is no longer added when flags are passed to `FilesystemIterator`, so the loop sees `.` and `..` too. On PHP 8.1 the same call skipped them.",
        "For the old behaviour, add `FilesystemIterator::SKIP_DOTS` to the flags: `FilesystemIterator::KEY_AS_PATHNAME | FilesystemIterator::SKIP_DOTS`.",
      ],
    },
    before: "new FilesystemIterator($dir, FilesystemIterator::KEY_AS_PATHNAME);",
    after: "new FilesystemIterator($dir, FilesystemIterator::KEY_AS_PATHNAME | FilesystemIterator::SKIP_DOTS);",
  },
  "behavior.gettype_closed_resource": {
    steps: {
      tr: [
        "PHP 7.2'den beri `gettype()` kapatılmış bir kaynak için `'unknown type'` yerine `'resource (closed)'` döndürür.",
        "Karşılaştırmadaki metni `'resource (closed)'` yapın. Kod iki sürümde de çalışacaksa iki değeri birden kontrol edin.",
      ],
      en: [
        "Since PHP 7.2, `gettype()` returns `'resource (closed)'` instead of `'unknown type'` for a closed resource.",
        "Change the string in the comparison to `'resource (closed)'`. If the code has to run on both versions, check for both values.",
      ],
    },
    before: "if (gettype($fp) === 'unknown type') {",
    after: "if (gettype($fp) === 'resource (closed)') {",
  },
  "behavior.hex_numeric_string": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `'0x1A'` gibi metinler sayısal değildir: `is_numeric('0x1A')` false döner, `'0x1A' == 26` de false'tur. PHP 5.6'da ikisi de true'ydu.",
        "Onaltılık metni sayıya çevirmek için `hexdec()` kullanın; `0x` önekini kabul eder (`hexdec('0x1A')` 26 verir). Karşılaştırmadan önce iki tarafı da sayıya çevirin.",
      ],
      en: [
        "Since PHP 7.0, strings like `'0x1A'` aren't numeric: `is_numeric('0x1A')` returns false and `'0x1A' == 26` is false too. On PHP 5.6 both were true.",
        "Use `hexdec()` to turn the hex string into a number; it accepts the `0x` prefix (`hexdec('0x1A')` gives 26). Convert both sides before comparing.",
      ],
    },
    before: "if ($value == 26) {",
    after: "if (hexdec($value) == 26) {",
  },
  "behavior.int_as_char_code": {
    steps: {
      tr: [
        "PHP 7'de `strpos()` gibi fonksiyonlar tamsayı aranan değeri karakter kodu sayıyordu (65 = `A`). PHP 8.0'dan beri rakamları metin olarak arar; sonuç sessizce değişir.",
        "Karakter kodu kastediliyorsa `chr()` ile sarın; Ferry bunu sabit sayılar için kendisi yapar. Negatif sayılarda ve değişkenlerde değerin ne olduğuna bakıp `chr()` ya da `(string)` yazın.",
      ],
      en: [
        "On PHP 7, functions such as `strpos()` read an integer needle as a character code (65 = `A`). Since PHP 8.0 they search for the digits as a string, so the result changes silently.",
        "If a character code was meant, wrap it in `chr()`; Ferry does this itself for literal numbers. For negative numbers and variables, check what the value is and write `chr()` or `(string)`.",
      ],
    },
    before: "$end = strpos($data, 0);",
    after: "$end = strpos($data, chr(0));",
  },
  "behavior.json_decode_null_assoc": {
    steps: {
      tr: [
        "PHP 7.2'den beri `json_decode()`'a `assoc` olarak null verildiğinde `JSON_OBJECT_AS_ARRAY` bayrağı dikkate alınır ve nesneler yerine diziler döner. PHP 7.1'de null, false gibi davranıyordu.",
        "Nesne isteniyorsa `assoc`'u false yapın; dizi isteniyorsa true yazıp bayrağı kaldırın. İkisi de bütün sürümlerde aynı sonucu verir.",
      ],
      en: [
        "Since PHP 7.2, when `json_decode()` gets null as `assoc`, the `JSON_OBJECT_AS_ARRAY` flag is honoured and arrays come back instead of objects. On PHP 7.1, null acted like false.",
        "If objects are wanted, make `assoc` false; if arrays are wanted, write true and drop the flag. Both give the same result on every version.",
      ],
    },
    before: "json_decode($json, null, 512, JSON_OBJECT_AS_ARRAY);",
    after: "json_decode($json, false, 512, JSON_OBJECT_AS_ARRAY);",
  },
  "behavior.list_assignment_order": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `list()` soldan sağa atar. `list($a[], $a[]) = [1, 2]` PHP 5.6'da `$a`'yı `[2, 1]`, PHP 7'de `[1, 2]` yapar.",
        "Sıra önemliyse elemanları açıkça yazın ya da anahtarlı `list()` kullanın: `list(1 => $a[], 0 => $a[]) = [1, 2]` PHP 7.1'den beri `[2, 1]` verir.",
      ],
      en: [
        "Since PHP 7.0, `list()` assigns left to right. `list($a[], $a[]) = [1, 2]` makes `$a` `[2, 1]` on PHP 5.6 and `[1, 2]` on PHP 7.",
        "If the order matters, write the elements out or use a keyed `list()`: `list(1 => $a[], 0 => $a[]) = [1, 2]` gives `[2, 1]` since PHP 7.1.",
      ],
    },
    before: "list($stack[], $stack[]) = $pair;",
    after: "$stack[] = $pair[1];\n$stack[] = $pair[0];",
  },
  "behavior.mb_ereg_named_capture": {
    steps: {
      tr: [
        "PHP 7.3'ten beri `mb_ereg*()` fonksiyonları adlandırılmış grupları (`(?<ad>...)`) destekler: adlar eşleşme dizisine eklenir, `mb_ereg_replace()` da değiştirme metninde `\\k<ad>` gibi yeni sözdizimini yorumlar.",
        "Eşleşme dizisini sırayla dolaşan ya da eleman sayısına bakan kodu kontrol edin; adlı anahtarlar artık dizide. Değiştirme metninde `\\k` geçiyorsa ve düz metin olarak kastediliyorsa ters bölüyü kaçırın.",
      ],
      en: [
        "Since PHP 7.3, the `mb_ereg*()` functions support named groups (`(?<name>...)`): the names are added to the match array, and `mb_ereg_replace()` reads new syntax such as `\\k<name>` in the replacement.",
        "Check code that walks the match array in order or counts its elements; the named keys are in it now. If the replacement contains `\\k` meant as plain text, escape the backslash.",
      ],
    },
  },
  "behavior.mysqli_connect_result": {
    steps: {
      tr: [
        "PHP 8.1'den beri `mysqli::connect()` başarılı olunca null yerine true döndürür. `=== null` ile başarıyı kontrol eden kod artık her bağlantıyı başarısız sayar.",
        "Sonucu true ile karşılaştırın ya da hatayı `$mysqli->connect_errno` ile kontrol edin; ikincisi bütün sürümlerde aynı çalışır.",
      ],
      en: [
        "Since PHP 8.1, `mysqli::connect()` returns true instead of null on success. Code that checks for success with `=== null` now treats every connection as failed.",
        "Compare the result with true, or check for an error with `$mysqli->connect_errno`; the second works the same on every version.",
      ],
    },
    before: "if ($db->connect($host, $user, $pass) === null) {",
    after: "$db->connect($host, $user, $pass);\nif (!$db->connect_errno) {",
  },
  "behavior.mysqli_max_length": {
    steps: {
      tr: [
        "PHP 8.1'den beri mysqli alan bilgisindeki `max_length` her zaman 0'dır. Eskiden sonuçtaki en uzun değerin uzunluğunu veriyordu.",
        "Değer gerekiyorsa sonucu dolaşıp hesaplayın: `max(array_map('strlen', array_column($rows, $name)))`.",
      ],
      en: [
        "Since PHP 8.1, `max_length` in mysqli field metadata is always 0. It used to give the length of the longest value in the result.",
        "If you need it, compute it from the rows: `max(array_map('strlen', array_column($rows, $name)))`.",
      ],
    },
    before: "$width = mysqli_fetch_field_direct($result, 0)->max_length;",
    after: "$rows = mysqli_fetch_all($result, MYSQLI_ASSOC);\n$width = max(array_map('strlen', array_column($rows, 'name')));",
  },
  "behavior.negative_array_key": {
    steps: {
      tr: [
        "PHP 8.0'dan beri negatif bir anahtardan sonraki otomatik anahtar 0 değil, bir fazlasıdır: `[-5 => 'a', 'b']` PHP 7.4'te `[-5, 0]`, PHP 8'de `[-5, -4]` anahtarlarını verir.",
        "Sonraki elemanın anahtarı önemliyse açıkça yazın: `[-5 => 'a', 0 => 'b']`.",
      ],
      en: [
        "Since PHP 8.0, the automatic key after a negative key is that key plus one, not 0: `[-5 => 'a', 'b']` gives the keys `[-5, 0]` on PHP 7.4 and `[-5, -4]` on PHP 8.",
        "If the next element's key matters, write it out: `[-5 => 'a', 0 => 'b']`.",
      ],
    },
    before: "$levels = [-1 => 'debug', 'info', 'warning'];",
    after: "$levels = [-1 => 'debug', 0 => 'info', 1 => 'warning'];",
  },
  "behavior.negative_index_assignment": {
    steps: {
      tr: [
        "PHP 8.3'ten beri boş bir diziye negatif anahtar atandıktan sonraki `[]` ataması 0 değil, bir fazlasını alır: `$a = []; $a[-5] = 'x'; $a[] = 'y';` PHP 8.2'de `y`'yi 0'a, PHP 8.3'te -4'e koyar.",
        "Anahtar önemliyse `$a[] = ...` yerine anahtarı açıkça yazın.",
      ],
      en: [
        "Since PHP 8.3, an `[]` assignment after a negative key on an empty array takes that key plus one, not 0: `$a = []; $a[-5] = 'x'; $a[] = 'y';` puts `y` at 0 on PHP 8.2 and at -4 on PHP 8.3.",
        "If the key matters, write it out instead of `$a[] = ...`.",
      ],
    },
    before: "$a = [];\n$a[-5] = 'x';\n$a[] = 'y';",
    after: "$a = [];\n$a[-5] = 'x';\n$a[0] = 'y';",
  },
  "behavior.parenthesized_reference": {
    steps: {
      tr: [
        "PHP 7.0'dan beri fazladan parantez bir şey değiştirmez: `array_pop((g()))` gibi bir çağrı, referans bekleyen parametreye fonksiyon sonucu verdiği için uyarı verir. PHP 5'te parantez bu uyarıyı gizliyordu.",
        "Sonucu önce bir değişkene atayıp değişkeni verin.",
      ],
      en: [
        "Since PHP 7.0, extra parentheses change nothing: a call like `array_pop((g()))` warns because it passes a function result to a by-reference parameter. On PHP 5 the parentheses hid that notice.",
        "Assign the result to a variable first and pass the variable.",
      ],
    },
    before: "$last = array_pop((explode('/', $path)));",
    after: "$parts = explode('/', $path);\n$last = array_pop($parts);",
  },
  "behavior.password_algo_integer": {
    steps: {
      tr: [
        "PHP 7.4'ten beri `PASSWORD_*` sabitleri tamsayı değil metindir: `PASSWORD_BCRYPT` 7.0'da `1`, 7.4'ten beri `'2y'`'dir. Bir sayıyla karşılaştırma artık eşleşmez.",
        "Sayı yerine sabitin kendisiyle karşılaştırın. `password_get_info()` ya da `password_needs_rehash()` çıktısıyla çalışan kod da sabitleri kullanmalı.",
      ],
      en: [
        "Since PHP 7.4, the `PASSWORD_*` constants are strings instead of integers: `PASSWORD_BCRYPT` was `1` on 7.0 and is `'2y'` since 7.4. Comparing with a number no longer matches.",
        "Compare with the constant itself instead of the number. Code that works with the output of `password_get_info()` or `password_needs_rehash()` should use the constants too.",
      ],
    },
    before: "if (password_get_info($hash)['algo'] === 1) {",
    after: "if (password_get_info($hash)['algo'] === PASSWORD_BCRYPT) {",
  },
  "behavior.password_hash_salt": {
    steps: {
      tr: [
        "`password_hash()`'in `salt` seçeneği PHP 8.0'da kaldırıldı; verilirse uyarıyla yok sayılır ve rastgele bir tuz kullanılır.",
        "Seçeneği silin. Aynı parolanın her seferinde aynı özeti vermesine dayanan kod (özetleri karşılaştırmak gibi) `password_verify()` kullanmalıdır.",
      ],
      en: [
        "The `salt` option of `password_hash()` was removed in PHP 8.0; when given, it's ignored with a warning and a random salt is used.",
        "Remove the option. Code that relies on the same password always giving the same hash (comparing hashes, say) should use `password_verify()`.",
      ],
    },
    before: "password_hash($password, PASSWORD_DEFAULT, ['salt' => $salt, 'cost' => 10]);",
    after: "password_hash($password, PASSWORD_DEFAULT, ['cost' => 10]);",
  },
  "behavior.pcntl_failure_value": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `pcntl_sigwaitinfo()` ve `pcntl_sigtimedwait()` başarısız olunca -1 yerine false döndürür; `=== -1` kontrolü artık hiç doğru olmaz.",
        "Sonucu `=== false` ile karşılaştırın. Kod eski sürümlerde de çalışacaksa iki değeri birden kontrol edin.",
      ],
      en: [
        "Since PHP 8.4, `pcntl_sigwaitinfo()` and `pcntl_sigtimedwait()` return false instead of -1 on failure, so an `=== -1` check is never true anymore.",
        "Compare the result with `=== false`. If the code also runs on older versions, check for both values.",
      ],
    },
    before: "if (pcntl_sigtimedwait([SIGTERM], $info, 5) === -1) {",
    after: "if (pcntl_sigtimedwait([SIGTERM], $info, 5) === false) {",
  },
  "behavior.printf_bare_precision": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `%.f`, `%.E` ya da `%.g` gibi rakamsız bir hassasiyet 0 demektir; önceden varsayılan hassasiyete (6) dönülüyordu. Metin belirteçleri (`%.s`) değişmedi.",
        "Eski sonuç için hassasiyeti açıkça yazın: `%.6f`. Ferry sabit biçim metinlerinde bunu kendisi yapar; biçim bir değişkenden geliyorsa o değeri düzeltin.",
      ],
      en: [
        "Since PHP 8.5, a precision without digits such as `%.f`, `%.E` or `%.g` means 0; it used to fall back to the default precision (6). String conversions (`%.s`) didn't change.",
        "For the old result, write the precision out: `%.6f`. Ferry does this itself in literal formats; when the format comes from a variable, fix that value.",
      ],
    },
    before: "printf('%.f', $ratio);",
    after: "printf('%.6f', $ratio);",
  },
  "behavior.reference_callback": {
    steps: {
      tr: [
        "Bu fonksiyon geri çağırmasına referans vermez. PHP 8.0'dan beri referans bekleyen bir geri çağırma her çağrıda uyarı verir; değişiklik zaten hiçbir zaman diziye yansımıyordu.",
        "Parametredeki `&`'yi silin. Diziyi yerinde değiştirmek istiyorsanız `array_walk()` (referans verir) ya da sonucu atayan `array_map()` kullanın.",
      ],
      en: [
        "This function doesn't pass references to its callback. Since PHP 8.0, a callback that expects a reference warns on every call; the change never reached the array anyway.",
        "Remove the `&` from the parameter. To change the array in place, use `array_walk()` (which passes references) or `array_map()` and assign its result.",
      ],
    },
    before: "array_map(function (&$v) { $v = trim($v); }, $items);",
    after: "$items = array_map(function ($v) { return trim($v); }, $items);",
  },
  "behavior.regex_empty_min_quantifier": {
    steps: {
      tr: [
        "PHP 8.4'teki PCRE2 10.44 ve sonrasında `{,3}` \"en fazla 3\" anlamına gelen bir niceleyicidir. Önceden düz metindi: PHP 8.3'te `/a{,3}/` `a{,3}` metnini buluyordu, PHP 8.5'te `aa`'yı buluyor.",
        "Niceleyici kastediliyorsa `{0,3}` yazın; bütün sürümlerde aynı çalışır. Düz metin kastediliyorsa süslü parantezi kaçırın: `\\{,3}`.",
      ],
      en: [
        "With PCRE2 10.44 and later in PHP 8.4, `{,3}` is a quantifier meaning \"up to 3\". It used to be literal text: on PHP 8.3 `/a{,3}/` found the text `a{,3}`, on PHP 8.5 it finds `aa`.",
        "If a quantifier was meant, write `{0,3}`, which works the same everywhere. If the text was meant, escape the brace: `\\{,3}`.",
      ],
    },
    before: "preg_match('/^\\d{,3}$/', $value);",
    after: "preg_match('/^\\d{0,3}$/', $value);",
  },
  "behavior.session_key_pipe": {
    steps: {
      tr: [
        "Varsayılan oturum biçimi (`session.serialize_handler = php`) anahtarları `|` ile ayırır. İçinde `|` olan bir anahtar oturumun sessizce kaydedilmemesine yol açıyordu; PHP 8.5'ten beri uyarı verir.",
        "Anahtardan `|`'yi çıkarın ya da değerleri tek bir dizi anahtarının altında tutun. `session.serialize_handler`'ı `php_serialize` yapmak da sorunu kaldırır, ama mevcut oturumları okunmaz hâle getirir.",
      ],
      en: [
        "The default session format (`session.serialize_handler = php`) separates keys with `|`. A key containing `|` made the session silently fail to save; since PHP 8.5 it warns.",
        "Take the `|` out of the key, or keep the values under one array key. Switching `session.serialize_handler` to `php_serialize` also removes the problem, but makes existing sessions unreadable.",
      ],
    },
    before: "$_SESSION[$module . '|' . $id] = $state;",
    after: "$_SESSION[$module][$id] = $state;",
  },
  "behavior.sleep_non_array": {
    steps: {
      tr: [
        "`__sleep()` özellik adlarından oluşan bir dizi döndürmelidir. Başka bir şey döndürünce `serialize()` uyarı verip `N;` yazar, yani nesne null olarak saklanır; PHP 8.0'da bu uyarı notice'ten warning'e çıktı.",
        "Saklanacak özelliklerin adlarını dizi olarak döndürün. Hiçbir özellik saklanmayacaksa `return [];` yazın; o zaman nesne sınıfıyla birlikte, özelliksiz olarak saklanır.",
      ],
      en: [
        "`__sleep()` must return an array of property names. When it returns anything else, `serialize()` warns and writes `N;`, so the object is stored as null; PHP 8.0 raised that notice to a warning.",
        "Return the names of the properties to keep as an array. If none should be kept, write `return [];`; the object is then stored with its class and no properties.",
      ],
    },
    before: "public function __sleep() {\n    return null;\n}",
    after: "public function __sleep() {\n    return ['id', 'name'];\n}",
  },
  "behavior.spl_autoload_register_throw": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `spl_autoload_register()` hata durumunda her zaman istisna fırlatır; ikinci argüman olarak verilen false yok sayılır ve uyarı verir.",
        "İkinci argümanı silin. Kayıt hatasını yakalamak istiyorsanız çağrıyı `try { ... } catch (TypeError $e) { ... }` içine alın.",
      ],
      en: [
        "Since PHP 8.0, `spl_autoload_register()` always throws on failure; false as the second argument is ignored and warns.",
        "Remove the second argument. To catch a registration failure, wrap the call in `try { ... } catch (TypeError $e) { ... }`.",
      ],
    },
    before: "spl_autoload_register('my_loader', false);",
    after: "spl_autoload_register('my_loader');",
  },
  "behavior.substr_false": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `substr()` kesim metnin sonuna denk gelince false yerine boş metin döndürür; PHP 8.0'dan beri hiç false döndürmez.",
        "Karşılaştırmayı `=== ''` ile yapın. Proje bugün PHP 5'te çalışıyorsa Ferry bunu kendisi yazar; diğer projelerde kodun boş metin ve false için aynı şeyi yapmasını bekleyip beklemediğine bakın.",
      ],
      en: [
        "Since PHP 7.0, `substr()` returns an empty string instead of false when the cut falls at the end of the string, and since PHP 8.0 it never returns false.",
        "Compare with `=== ''`. When the project runs PHP 5 today, Ferry writes this itself; elsewhere, check whether the code expected the same thing for an empty string and false.",
      ],
    },
    before: "if (substr($path, $len) === false) {",
    after: "if (substr($path, $len) === '') {",
  },
  "behavior.trailing_open_tag": {
    steps: {
      tr: [
        "PHP 7.4'ten beri dosyanın en sonundaki, ardından satır sonu gelmeyen bir `<?php` açılış etiketidir; önceden metin olarak çıktıya yazılıyordu.",
        "Etiket çıktıya gitmek için yazılmadıysa onu silin. Çıktıya gitmesi gerekiyorsa `echo '<?php';` ile yazın.",
      ],
      en: [
        "Since PHP 7.4, a `<?php` at the very end of a file with no newline after it is an opening tag; it used to be printed as text.",
        "If the tag wasn't meant as output, remove it. If it was, print it with `echo '<?php';`.",
      ],
    },
  },
  "behavior.unserialize_allowed_classes_type": {
    steps: {
      tr: [
        "`unserialize()`'ın `allowed_classes` seçeneği PHP 7.1'den beri yalnızca dizi ya da bool kabul eder. Metin verilince false döner; PHP 8.4'ten beri ValueError fırlatır.",
        "Sınıf adını bir diziye koyun: `['allowed_classes' => ['Foo']]`. Hiçbir sınıfa izin verilmeyecekse false yazın.",
      ],
      en: [
        "`unserialize()`'s `allowed_classes` option only accepts an array or a bool since PHP 7.1. A string makes it return false, and since PHP 8.4 it throws a ValueError.",
        "Put the class name in an array: `['allowed_classes' => ['Foo']]`. To allow no classes, write false.",
      ],
    },
    before: "unserialize($data, ['allowed_classes' => 'Foo']);",
    after: "unserialize($data, ['allowed_classes' => ['Foo']]);",
  },
  "behavior.wide_shift": {
    steps: {
      tr: [
        "PHP 7.0'dan beri tamsayı genişliği (64 bit) ya da daha fazlası kadar kaydırmak her zaman 0 verir, negatif sayıyı sağa kaydırmak -1 verir. PHP 5.6'da sonuç işlemciye bağlıydı: `1 << 64` 1 veriyordu.",
        "Kaydırma miktarını sınırlayın ya da büyük sayılar için `gmp_mul()` ve `gmp_pow()` kullanın.",
      ],
      en: [
        "Since PHP 7.0, shifting by the integer width (64 bits) or more always gives 0, and shifting a negative number right gives -1. On PHP 5.6 the result depended on the CPU: `1 << 64` gave 1.",
        "Limit the shift amount, or use `gmp_mul()` and `gmp_pow()` for large numbers.",
      ],
    },
    before: "$mask = 1 << $bits;",
    after: "$mask = $bits >= 64 ? 0 : 1 << $bits;",
  },
  "deprecated.array_object_object": {
    steps: {
      tr: [
        "PHP 8.5'te `ArrayObject` ya da `ArrayIterator`'a nesne vermek kullanımdan kalktı; nesnenin özellikleri bir dizi gibi kullanılıyordu.",
        "Bir dizi verin: genel özellikler için `get_object_vars($object)`. Sayılar ve değerler aynı kalır; nesnenin kendisi artık değişmez, değişiklikler dizide kalır.",
      ],
      en: [
        "PHP 8.5 deprecated passing an object to `ArrayObject` or `ArrayIterator`; the object's properties were used as an array.",
        "Pass an array: `get_object_vars($object)` for the public properties. Counts and values stay the same; the object itself no longer changes, the changes stay in the array.",
      ],
    },
    before: "$view = new ArrayObject($settings);",
    after: "$view = new ArrayObject(get_object_vars($settings));",
  },
  "deprecated.array_pointer_on_object": {
    steps: {
      tr: [
        "PHP 8.1'den beri `key()`, `current()`, `next()`, `prev()`, `reset()` ve `end()`'i bir nesne üzerinde çağırmak kullanımdan kalktı.",
        "Önce nesneyi diziye çevirin: `$props = get_mangled_object_vars($object);` ve işaretçi fonksiyonlarını o dizide kullanın. Yalnızca genel özellikler gerekiyorsa `get_object_vars()` yeterlidir.",
      ],
      en: [
        "Since PHP 8.1, calling `key()`, `current()`, `next()`, `prev()`, `reset()` and `end()` on an object is deprecated.",
        "Turn the object into an array first: `$props = get_mangled_object_vars($object);`, and use the pointer functions on that array. `get_object_vars()` is enough when only public properties are needed.",
      ],
    },
    before: "$first = reset($this);",
    after: "$props = get_object_vars($this);\n$first = reset($props);",
  },
  "deprecated.assert_options": {
    steps: {
      tr: [
        "PHP 8.3'te `assert_options()`, `ASSERT_*` sabitleri ve `assert.active` gibi `assert.*` ini ayarları kullanımdan kalktı. Ferry varsayılanı yazan ve hiçbir şey değiştirmeyen çağrıları kendisi siler.",
        "Doğrulamaları açıp kapatmak için `zend.assertions` ayarını kullanın: geliştirmede 1, üretimde -1. Çalışırken yalnızca 0 ile 1 arasında değiştirilebilir; -1 için `php.ini` gerekir.",
        "PHP 8.0'dan beri başarısız `assert()` varsayılan olarak `AssertionError` fırlatır; uyarı ya da geri çağırma isteyen kod bunu bir `try`/`catch` ile karşılamalıdır.",
      ],
      en: [
        "PHP 8.3 deprecated `assert_options()`, the `ASSERT_*` constants and `assert.*` ini settings such as `assert.active`. Ferry removes calls that write the default and change nothing itself.",
        "To turn assertions on or off, use the `zend.assertions` setting: 1 in development, -1 in production. At runtime it can only switch between 0 and 1; -1 needs `php.ini`.",
        "Since PHP 8.0 a failed `assert()` throws `AssertionError` by default; code that wanted a warning or a callback should handle it with a `try`/`catch`.",
      ],
    },
    before: "assert_options(ASSERT_ACTIVE, 0);",
    after: "; php.ini\nzend.assertions = -1",
  },
  "deprecated.bool_sort_comparator": {
    steps: {
      tr: [
        "Karşılaştırma fonksiyonundan true ya da false döndürmek PHP 8.0'da kullanımdan kalktı; sıfırdan küçük, sıfıra eşit ya da büyük bir tamsayı döndürülmeli.",
        "`$a > $b` yerine `$a <=> $b` yazın. Ferry basit karşılaştırmalarda bunu kendisi yapar; `==` ya da birden çok koşul içeren fonksiyonları elle çevirin.",
        "PHP 8.0'dan beri sıralama kararlıdır: eşit sayılan elemanlar ilk sıralarını korur. Eski sıralamaya güvenen testler farklı çıkabilir.",
      ],
      en: [
        "Returning true or false from a comparison function was deprecated in PHP 8.0; return an integer below, equal to or above zero.",
        "Write `$a <=> $b` instead of `$a > $b`. Ferry does this itself for simple comparisons; convert functions with `==` or several conditions by hand.",
        "Since PHP 8.0 sorting is stable: elements that compare equal keep their original order. Tests that relied on the old order can differ.",
      ],
    },
    before: "usort($rows, fn($a, $b) => $a['n'] > $b['n']);",
    after: "usort($rows, fn($a, $b) => $a['n'] <=> $b['n']);",
  },
  "deprecated.closure_bind": {
    steps: {
      tr: [
        "Statik bir kapanışa nesne bağlamak ya da `$this` kullanan bir kapanıştan `$this`'i ayırmak her zaman uyarı verip null döndürüyordu; PHP 8.5'te kullanımdan kalktı, PHP 9'da hata olacak.",
        "Nesne bağlanacaksa kapanıştan `static` sözcüğünü kaldırın. Kapanış `$this` kullanıyorsa ona null bağlamak yerine nesneyi bağlayın ya da kapanışı nesnesiz yazın.",
        "Kod bu çağrının sonucunu (null) kullanmıyorsa çağrı zaten hiçbir şey yapmıyordu; silebilirsiniz.",
      ],
      en: [
        "Binding an object to a static closure, or unbinding `$this` from a closure that uses it, always warned and returned null; PHP 8.5 deprecated it and PHP 9 will make it an error.",
        "If an object should be bound, remove `static` from the closure. If the closure uses `$this`, bind an object instead of null, or write the closure without it.",
        "If the code doesn't use this call's result (null), the call never did anything; you can delete it.",
      ],
    },
    before: "$f = static function () { return $this->id; };\n$g = $f->bindTo($user);",
    after: "$f = function () { return $this->id; };\n$g = $f->bindTo($user);",
  },
  "deprecated.com_typelib_case_insensitive": {
    steps: {
      tr: [
        "Bir tür kitaplığını büyük-küçük harfe duyarsız kaydetmek PHP 7.4'te kullanımdan kalktı ve PHP 8.0'da kaldırıldı.",
        "`com_load_typelib()`'in ikinci argümanını silin ve sabitleri tür kitaplığındaki yazımıyla kullanın.",
      ],
      en: [
        "Registering a type library case-insensitively was deprecated in PHP 7.4 and removed in PHP 8.0.",
        "Remove `com_load_typelib()`'s second argument and use the constants as the type library spells them.",
      ],
    },
    before: "com_load_typelib('Word.Application', false);",
    after: "com_load_typelib('Word.Application');",
  },
  "deprecated.ctype_non_string": {
    steps: {
      tr: [
        "PHP 8.1'den beri `ctype_*()` fonksiyonlarına tamsayı vermek kullanımdan kalktı. -128 ile 255 arasındaki tamsayılar karakter kodu sayılıyordu: `ctype_digit(53)` `'5'` karakterine bakıp true, `ctype_digit(5)` false veriyordu; daha büyükler metin gibi okunuyordu.",
        "Sayının rakamları kastediliyorsa `(string)` ile metne çevirin; karakter kodu kastediliyorsa `chr()` kullanın.",
      ],
      en: [
        "Since PHP 8.1, passing an integer to the `ctype_*()` functions is deprecated. Integers from -128 to 255 were taken as character codes: `ctype_digit(53)` looked at the character `'5'` and gave true, `ctype_digit(5)` gave false; larger ones were read as strings.",
        "If the number's digits were meant, convert it with `(string)`; if a character code was meant, use `chr()`.",
      ],
    },
    before: "if (ctype_digit($id)) {",
    after: "if (ctype_digit((string) $id)) {",
  },
  "deprecated.curlopt_binarytransfer": {
    steps: {
      tr: [
        "`CURLOPT_BINARYTRANSFER` PHP 5.1.3'ten beri hiçbir şey yapmıyor ve PHP 8.4'te kullanımdan kalktı.",
        "Seçeneği silin. Ferry bunu `curl_setopt()` çağrılarında ve curl seçenek dizilerinde kendisi yapar; sonucu kullanılan bir `curl_setopt()` çağrısında yerine true yazabilirsiniz, çünkü çağrı her zaman başarılıydı.",
      ],
      en: [
        "`CURLOPT_BINARYTRANSFER` has done nothing since PHP 5.1.3 and was deprecated in PHP 8.4.",
        "Remove the option. Ferry does this itself in `curl_setopt()` calls and curl option arrays; where a `curl_setopt()` call's result is used, you can write true instead, since the call always succeeded.",
      ],
    },
    before: "$ok = curl_setopt($ch, CURLOPT_BINARYTRANSFER, true);",
    after: "$ok = true;",
  },
  "deprecated.curlpipe_http1": {
    steps: {
      tr: [
        "`CURLPIPE_HTTP1` curl 7.62'den beri hiçbir şey yapmıyor ve PHP 7.4'ten beri kullanımdan kalktı.",
        "Bugünkü davranış için `CURLPIPE_NOTHING`, HTTP/2 çoğullama için `CURLPIPE_MULTIPLEX` yazın. Ferry maskelerden ve tek başına kullanılan yerlerden bunu kendisi yapar; bir değeri onunla karşılaştıran kodu elle değiştirin.",
      ],
      en: [
        "`CURLPIPE_HTTP1` has done nothing since curl 7.62 and has been deprecated since PHP 7.4.",
        "Write `CURLPIPE_NOTHING` for today's behaviour or `CURLPIPE_MULTIPLEX` for HTTP/2 multiplexing. Ferry does this itself in masks and where it stands alone; change code that compares a value with it by hand.",
      ],
    },
    before: "curl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_HTTP1);",
    after: "curl_multi_setopt($mh, CURLMOPT_PIPELINING, CURLPIPE_NOTHING);",
  },
  "deprecated.date_period_iso_string": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `new DatePeriod()`'u ISO 8601 metniyle çağırmak kullanımdan kalktı.",
        "PHP 8.3'te gelen `DatePeriod::createFromISO8601String()`'i kullanın; aynı metni ve seçenekleri alır.",
      ],
      en: [
        "Since PHP 8.4, calling `new DatePeriod()` with an ISO 8601 string is deprecated.",
        "Use `DatePeriod::createFromISO8601String()`, added in PHP 8.3; it takes the same string and options.",
      ],
    },
    before: "$period = new DatePeriod('R4/2012-07-01T00:00:00Z/P7D');",
    after: "$period = DatePeriod::createFromISO8601String('R4/2012-07-01T00:00:00Z/P7D');",
  },
  "deprecated.date_wakeup": {
    steps: {
      tr: [
        "Tarih sınıflarının `__wakeup()` metodu PHP 8.5'te kullanımdan kalktı. Bir `DateTime`, `DateTimeImmutable`, `DateTimeZone` ya da `DatePeriod` nesnesinde doğrudan çağrılınca her zaman Error fırlatıyordu, `DateInterval`'da hiçbir şey yapmıyordu.",
        "Çağrıyı silin; Ferry canlı bir tarih nesnesindeki tek başına duran çağrıları kendisi siler. Alt sınıfta `__wakeup()` tanımlıysa o mantığı `__unserialize()`'a taşıyın.",
      ],
      en: [
        "The date classes' `__wakeup()` method was deprecated in PHP 8.5. Called directly on a `DateTime`, `DateTimeImmutable`, `DateTimeZone` or `DatePeriod` object it always threw Error; on `DateInterval` it did nothing.",
        "Delete the call; Ferry deletes standalone calls on a live date object itself. If a subclass defines `__wakeup()`, move that logic to `__unserialize()`.",
      ],
    },
    before: "public function __wakeup()\n{\n    $this->zone = new DateTimeZone($this->name);\n}",
    after: "public function __unserialize(array $data): void\n{\n    $this->name = $data['name'];\n    $this->zone = new DateTimeZone($this->name);\n}",
  },
  "deprecated.dba_fetch_order": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `dba_fetch()`'e tutamacı üçüncü argüman olarak vermek kullanımdan kalktı.",
        "Argümanların sırasını değiştirin: `dba_fetch($key, $dba, $skip)`.",
      ],
      en: [
        "Since PHP 8.3, passing the handle as the third argument of `dba_fetch()` is deprecated.",
        "Swap the arguments: `dba_fetch($key, $dba, $skip)`.",
      ],
    },
    before: "dba_fetch($key, 0, $dba);",
    after: "dba_fetch($key, $dba, 0);",
  },
  "deprecated.dba_key_split_null": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `dba_key_split()`'e null ya da false vermek kullanımdan kalktı; bu değerler için hep false dönüyordu.",
        "Çağrıdan önce değeri kontrol edin ve null ya da false için doğrudan false kullanın.",
      ],
      en: [
        "Since PHP 8.4, passing null or false to `dba_key_split()` is deprecated; it always returned false for them.",
        "Check the value before the call and use false directly for null or false.",
      ],
    },
    before: "$parts = dba_key_split($key);",
    after: "$parts = $key === null || $key === false ? false : dba_key_split($key);",
  },
  "deprecated.debug_info_null": {
    steps: {
      tr: [
        "PHP 8.5'te `__debugInfo()`'dan null döndürmek kullanımdan kalktı.",
        "Boş bir dizi döndürün; `var_dump()` çıktısı aynı kalır (PHP 7.4'ten 8.5'e kadar her ikisi de özelliksiz bir nesne gösterdi).",
      ],
      en: [
        "PHP 8.5 deprecated returning null from `__debugInfo()`.",
        "Return an empty array; the `var_dump()` output stays the same (on PHP 7.4 through 8.5 both showed an object with no properties).",
      ],
    },
    before: "public function __debugInfo() {\n    return null;\n}",
    after: "public function __debugInfo() {\n    return [];\n}",
  },
  "deprecated.e_strict": {
    steps: {
      tr: [
        "`E_STRICT` PHP 8.4'te kullanımdan kalktı; bu hata seviyesi PHP 8'den beri hiç üretilmiyor.",
        "Maskelerden çıkarın: `E_ALL | E_STRICT` yerine `E_ALL`. Ferry maskelerden kendisi çıkarır; tek başına geçtiği ya da bir değerle karşılaştırıldığı yerlerde değerini (2048) yazabilir ya da kontrolü silebilirsiniz.",
      ],
      en: [
        "`E_STRICT` was deprecated in PHP 8.4; this error level hasn't been raised since PHP 8.",
        "Take it out of masks: `E_ALL` instead of `E_ALL | E_STRICT`. Ferry removes it from masks itself; where it stands alone or is compared with a value, write its value (2048) or remove the check.",
      ],
    },
    before: "error_reporting(E_ALL | E_STRICT);",
    after: "error_reporting(E_ALL);",
  },
  "deprecated.ffi_static_call": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `FFI::cast()`, `FFI::new()` ve `FFI::type()`'ı statik çağırmak kullanımdan kalktı.",
        "Bir FFI nesnesi oluşturup metotları onun üzerinden çağırın: `$ffi = FFI::cdef(); $ffi->new('int');`.",
      ],
      en: [
        "Since PHP 8.3, calling `FFI::cast()`, `FFI::new()` and `FFI::type()` statically is deprecated.",
        "Create an FFI object and call the methods on it: `$ffi = FFI::cdef(); $ffi->new('int');`.",
      ],
    },
    before: "$n = FFI::new('int');",
    after: "$ffi = FFI::cdef();\n$n = $ffi->new('int');",
  },
  "deprecated.finfo_buffer_context": {
    steps: {
      tr: [
        "`finfo_buffer()`'ın yok sayılan `$context` argümanı PHP 8.5'te kullanımdan kalktı.",
        "Argümanı silin; sonuç değişmez.",
      ],
      en: [
        "PHP 8.5 deprecated `finfo_buffer()`'s ignored `$context` argument.",
        "Remove the argument; the result doesn't change.",
      ],
    },
    before: "finfo_buffer($finfo, $data, FILEINFO_MIME_TYPE, $context);",
    after: "finfo_buffer($finfo, $data, FILEINFO_MIME_TYPE);",
  },
  "deprecated.get_defined_functions_arg": {
    steps: {
      tr: [
        "Devre dışı fonksiyonları da almak için false vermek PHP 8.0'da, argümanın kendisi PHP 8.5'te kullanımdan kalktı. PHP 8.0'dan beri devre dışı fonksiyonlar listede zaten yok.",
        "Argümanı silin.",
      ],
      en: [
        "Passing false to include disabled functions was deprecated in PHP 8.0, and the argument itself in PHP 8.5. Since PHP 8.0 disabled functions aren't in the list anyway.",
        "Remove the argument.",
      ],
    },
    before: "$functions = get_defined_functions(true);",
    after: "$functions = get_defined_functions();",
  },
  "deprecated.hash_option_type": {
    steps: {
      tr: [
        "PHP 8.4'ten beri hash fonksiyonlarının seçeneklerine yanlış türde değer vermek kullanımdan kalktı; örneğin murmur ve xxh algoritmalarının `seed`'i tamsayı olmalı.",
        "Değeri doğru türe çevirin: `['seed' => (int) $seed]`.",
      ],
      en: [
        "Since PHP 8.4, passing values of the wrong type to hash function options is deprecated; the `seed` of the murmur and xxh algorithms, for example, must be an integer.",
        "Convert the value to the right type: `['seed' => (int) $seed]`.",
      ],
    },
    before: "hash('murmur3a', $data, false, ['seed' => '42']);",
    after: "hash('murmur3a', $data, false, ['seed' => 42]);",
  },
  "deprecated.implicit_float_to_int": {
    steps: {
      tr: [
        "PHP 8.1'den beri ondalıklı bir sayının dizi anahtarı ya da `%` işleneni olarak sessizce tamsayıya çevrilmesi kullanımdan kalktı. Sonuç aynı: `$a[1.5]` 1 anahtarını, `7.5 % 2` 1'i kullanıyor.",
        "Çeviriyi açıkça yazın: `$a[(int) $x]`, `(int) $x % 2`. Kesir kısmı önemliyse `intdiv()`, `fmod()` ya da `round()` kullanın.",
      ],
      en: [
        "Since PHP 8.1, a fractional number silently turning into an integer as an array key or a `%` operand is deprecated. The result is the same: `$a[1.5]` uses the key 1 and `7.5 % 2` gives 1.",
        "Write the conversion out: `$a[(int) $x]`, `(int) $x % 2`. If the fraction matters, use `intdiv()`, `fmod()` or `round()`.",
      ],
    },
    before: "$buckets[$value / 10][] = $value;",
    after: "$buckets[(int) ($value / 10)][] = $value;",
  },
  "deprecated.intlcal_roll_bool": {
    steps: {
      tr: [
        "PHP 8.1'den beri `IntlCalendar::roll()`'a bool vermek kullanımdan kalktı; true bir birim ileri, false bir birim geri demekti.",
        "true yerine 1, false yerine -1 yazın.",
      ],
      en: [
        "Since PHP 8.1, passing a bool to `IntlCalendar::roll()` is deprecated; true meant one unit up and false one unit down.",
        "Write 1 instead of true and -1 instead of false.",
      ],
    },
    before: "$cal->roll(IntlCalendar::FIELD_MONTH, true);",
    after: "$cal->roll(IntlCalendar::FIELD_MONTH, 1);",
  },
  "deprecated.intlcal_set": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `IntlCalendar::set()`'i ve `intlcal_set()`'i tarih parçalarıyla çağırmak kullanımdan kalktı.",
        "Tarih için `setDate($year, $month, $day)`, tarih ve saat için `setDateTime($year, $month, $day, $hour, $minute, $second)` kullanın; ikisi de PHP 8.3'te geldi. Ferry `intlcal_set()` fonksiyon biçimini sonucu kullanılmadığında kendisi çevirir; sonuç kullanılıyorsa yeni metotların değer döndürmediğini hesaba katın.",
      ],
      en: [
        "Since PHP 8.4, calling `IntlCalendar::set()` and `intlcal_set()` with date parts is deprecated.",
        "Use `setDate($year, $month, $day)` for a date and `setDateTime($year, $month, $day, $hour, $minute, $second)` for date and time; both came in PHP 8.3. Ferry converts the `intlcal_set()` function form itself when its result isn't used; where it is, keep in mind the new methods return nothing.",
      ],
    },
    before: "$cal->set(2024, 0, 31);",
    after: "$cal->setDate(2024, 0, 31);",
  },
  "deprecated.intlgregcal_create_instance": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `new IntlGregorianCalendar()`'ı ve `intlgregcal_create_instance()`'ı tarih parçalarıyla çağırmak kullanımdan kalktı.",
        "PHP 8.3'te gelen `IntlGregorianCalendar::createFromDate()` ya da `createFromDateTime()`'ı kullanın. Ferry fonksiyon biçimini kendisi çevirir.",
      ],
      en: [
        "Since PHP 8.4, calling `new IntlGregorianCalendar()` and `intlgregcal_create_instance()` with date parts is deprecated.",
        "Use `IntlGregorianCalendar::createFromDate()` or `createFromDateTime()`, added in PHP 8.3. Ferry converts the function form itself.",
      ],
    },
    before: "$cal = new IntlGregorianCalendar(2024, 0, 31);",
    after: "$cal = IntlGregorianCalendar::createFromDate(2024, 0, 31);",
  },
  "deprecated.invalid_digits": {
    steps: {
      tr: [
        "PHP 7.4'ten beri `base_convert()`, `bindec()`, `octdec()` ve `hexdec()`'e tabanda olmayan rakamlar vermek kullanımdan kalktı; o rakamlar yok sayılır ve sonuç onlarsız hesaplanır.",
        "Girdiyi çağrıdan önce doğrulayın ya da temizleyin. Baştaki `0x`, `0b`, `0o` önekleri ve boşluklar hâlâ kabul edilir.",
      ],
      en: [
        "Since PHP 7.4, passing digits that aren't in the base to `base_convert()`, `bindec()`, `octdec()` and `hexdec()` is deprecated; those digits are ignored and the result is computed without them.",
        "Validate or clean the input before the call. Leading `0x`, `0b` and `0o` prefixes and whitespace are still accepted.",
      ],
    },
    before: "$n = base_convert($input, 16, 10);",
    after: "if (!ctype_xdigit($input)) {\n    throw new InvalidArgumentException('Not hex');\n}\n$n = base_convert($input, 16, 10);",
  },
  "deprecated.ldap_exop_response": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `ldap_exop()`'un yanıt argümanları (`$response_data`, `$response_oid`) kullanımdan kalktı.",
        "Yanıtı almak için PHP 8.3'te gelen `ldap_exop_sync()`'i kullanın; aynı argümanları alır ve true ya da false döndürür.",
      ],
      en: [
        "Since PHP 8.4, `ldap_exop()`'s response arguments (`$response_data`, `$response_oid`) are deprecated.",
        "Use `ldap_exop_sync()`, added in PHP 8.3, to get the response; it takes the same arguments and returns true or false.",
      ],
    },
    before: "ldap_exop($link, LDAP_EXOP_WHO_AM_I, null, null, $data, $oid);",
    after: "ldap_exop_sync($link, LDAP_EXOP_WHO_AM_I, null, null, $data, $oid);",
  },
  "deprecated.ldap_rebind_empty": {
    steps: {
      tr: [
        "PHP 8.0'dan beri geri çağırmayı kaldırmak için boş metin vermek kullanımdan kalktı.",
        "Boş metin yerine null verin.",
      ],
      en: [
        "Since PHP 8.0, passing an empty string to remove the callback is deprecated.",
        "Pass null instead of the empty string.",
      ],
    },
    before: "ldap_set_rebind_proc($link, '');",
    after: "ldap_set_rebind_proc($link, null);",
  },
  "deprecated.mb_strimwidth_negative": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `mb_strimwidth()`'e negatif genişlik vermek kullanımdan kalktı; negatif genişlik metnin sonundan sayılıyordu.",
        "Genişliği önceden hesaplayın: `mb_strwidth($s) + $width` (negatif `$width` için) ve sonucu pozitif olarak verin.",
      ],
      en: [
        "Since PHP 8.3, passing a negative width to `mb_strimwidth()` is deprecated; a negative width counted from the end of the string.",
        "Compute the width first: `mb_strwidth($s) + $width` (for a negative `$width`), and pass the result as a positive number.",
      ],
    },
    before: "mb_strimwidth($title, 0, -5, '…');",
    after: "mb_strimwidth($title, 0, max(0, mb_strwidth($title) - 5), '…');",
  },
  "deprecated.mysqli_driver_version": {
    steps: {
      tr: [
        "`mysqli_driver::$driver_version` PHP 8.1'de kullanımdan kalktı; yıllardır güncellenmiyordu.",
        "İstemci kitaplığının sürümü için `mysqli_get_client_version()` ya da `mysqli_get_client_info()` kullanın. Sürüm yalnızca gösteriliyorsa satırı silebilirsiniz.",
      ],
      en: [
        "`mysqli_driver::$driver_version` was deprecated in PHP 8.1; it hadn't been updated in years.",
        "Use `mysqli_get_client_version()` or `mysqli_get_client_info()` for the client library's version. If the value is only displayed, you can remove the line.",
      ],
    },
    before: "echo (new mysqli_driver())->driver_version;",
    after: "echo mysqli_get_client_version();",
  },
  "deprecated.mysqli_get_client_info_arg": {
    steps: {
      tr: [
        "PHP 8.1'den beri `mysqli_get_client_info()`'ya bağlantı vermek kullanımdan kalktı; sonuç zaten bağlantıya bağlı değildi.",
        "Argümanı silin.",
      ],
      en: [
        "Since PHP 8.1, passing a connection to `mysqli_get_client_info()` is deprecated; the result never depended on it.",
        "Remove the argument.",
      ],
    },
    before: "$info = mysqli_get_client_info($link);",
    after: "$info = mysqli_get_client_info();",
  },
  "deprecated.mysqli_refresh": {
    steps: {
      tr: [
        "`mysqli_refresh()`, `mysqli::refresh()` ve `MYSQLI_REFRESH_*` sabitleri PHP 8.4'te kullanımdan kalktı.",
        "Aynı işi `FLUSH` sorgularıyla yapın: `MYSQLI_REFRESH_TABLES` için `FLUSH TABLES`, `MYSQLI_REFRESH_LOG` için `FLUSH LOGS`, `MYSQLI_REFRESH_GRANT` için `FLUSH PRIVILEGES`. Ferry bilinen bayraklar için bunu kendisi yazar.",
        "`MYSQLI_REFRESH_SLAVE`, `REPLICA` ve `MASTER` gibi bayrakların karşılığı sunucu sürümüne göre değişir (`RESET REPLICA`, `RESET MASTER` gibi); sunucunun belgesine bakın.",
      ],
      en: [
        "PHP 8.4 deprecated `mysqli_refresh()`, `mysqli::refresh()` and the `MYSQLI_REFRESH_*` constants.",
        "Do the same with `FLUSH` queries: `FLUSH TABLES` for `MYSQLI_REFRESH_TABLES`, `FLUSH LOGS` for `MYSQLI_REFRESH_LOG`, `FLUSH PRIVILEGES` for `MYSQLI_REFRESH_GRANT`. Ferry writes this itself for the flags it knows.",
        "Flags such as `MYSQLI_REFRESH_SLAVE`, `REPLICA` and `MASTER` map to statements that depend on the server version (`RESET REPLICA`, `RESET MASTER` and so on); check the server's documentation.",
      ],
    },
    before: "$db->refresh(MYSQLI_REFRESH_TABLES);",
    after: "$db->query('FLUSH TABLES');",
  },
  "deprecated.null_to_internal": {
    steps: {
      tr: [
        "PHP 8.1'den beri yerleşik fonksiyonların null kabul etmeyen parametrelerine null vermek kullanımdan kalktı; PHP 9'da TypeError olacak.",
        "Eski sonucu korumak için null yerine türün boş değerini yazın: metin için `''`, sayı için `0`, bool için false. Ferry düz `null` argümanlarını kendisi çevirir; `strict_types` açık dosyalarda bunu yapmaz, çünkü orada null zaten TypeError'dı.",
        "Değer bir değişkenden geliyorsa `$value ?? ''` ya da `(string) $value` yazın.",
      ],
      en: [
        "Since PHP 8.1, passing null to non-nullable parameters of built-in functions is deprecated; PHP 9 will make it a TypeError.",
        "To keep the old result, write the type's empty value instead of null: `''` for strings, `0` for numbers, false for bools. Ferry converts plain `null` arguments itself; it doesn't in files with `strict_types`, where null was already a TypeError.",
        "When the value comes from a variable, write `$value ?? ''` or `(string) $value`.",
      ],
    },
    before: "$len = strlen($name);",
    after: "$len = strlen($name ?? '');",
  },
  "deprecated.openssl_pkey_derive_length": {
    steps: {
      tr: [
        "`openssl_pkey_derive()`'ın `$key_length` argümanı PHP 8.5'te kullanımdan kalktı.",
        "Argümanı silin ve gerekiyorsa sonucu kısaltın: EC anahtarlarıyla `openssl_pkey_derive($pub, $priv, 16)`, `substr(openssl_pkey_derive($pub, $priv), 0, 16)` ile aynı baytları verdi.",
      ],
      en: [
        "PHP 8.5 deprecated `openssl_pkey_derive()`'s `$key_length` argument.",
        "Remove the argument and cut the result if needed: with EC keys, `openssl_pkey_derive($pub, $priv, 16)` gave the same bytes as `substr(openssl_pkey_derive($pub, $priv), 0, 16)`.",
      ],
    },
    before: "$secret = openssl_pkey_derive($peer, $key, 16);",
    after: "$secret = substr(openssl_pkey_derive($peer, $key), 0, 16);",
  },
  "deprecated.openssl_pkey_free": {
    steps: {
      tr: [
        "`openssl_pkey_free()` ve `openssl_x509_free()` PHP 8.0'dan beri hiçbir şey yapmıyor ve kullanımdan kalktı; anahtar nesnesi son referansı gidince kendiliğinden serbest kalır.",
        "Çağrıyı silin; Ferry tek başına duran çağrıları kendisi siler. Sonucu kullanılıyorsa yerine `true` yazın, ya da anahtarı hemen bırakmak için değişkene null atayın.",
      ],
      en: [
        "`openssl_pkey_free()` and `openssl_x509_free()` have done nothing since PHP 8.0, which deprecated them; the key object frees itself when the last reference goes.",
        "Delete the call; Ferry deletes standalone calls itself. Where its result is used, write `true`, or set the variable to null to let go of the key right away.",
      ],
    },
    before: "$freed = openssl_pkey_free($key);",
    after: "$key = null;\n$freed = true;",
  },
  "deprecated.output_in_ob_handler": {
    steps: {
      tr: [
        "PHP 8.5'ten beri bir çıktı işleyicisinin içinde `echo` ya da `print` ile çıktı üretmek kullanımdan kalktı.",
        "Yazılacak metni işleyicinin döndürdüğü değere ekleyin: `return $buffer . $extra;`.",
      ],
      en: [
        "Since PHP 8.5, producing output with `echo` or `print` inside an output handler is deprecated.",
        "Add the text to the value the handler returns: `return $buffer . $extra;`.",
      ],
    },
    before: "ob_start(function ($buffer) {\n    echo '<!-- cached -->';\n    return $buffer;\n});",
    after: "ob_start(function ($buffer) {\n    return '<!-- cached -->' . $buffer;\n});",
  },
  "deprecated.pdo_fetch_serialize": {
    steps: {
      tr: [
        "`PDO::FETCH_SERIALIZE` `Serializable` arayüzüne dayanıyor ve PHP 8.1'de kullanımdan kalktı. Tek başına hiçbir etkisi yoktu: PDO varsayılan modu kullanıyordu, bu yüzden `PDO::FETCH_DEFAULT` sonucu korur. Ferry bunu kendisi yazar.",
        "`FETCH_CLASS` ile birlikte kullanılıyorsa sütunu okuyup kendiniz `unserialize()` edin.",
      ],
      en: [
        "`PDO::FETCH_SERIALIZE` relies on the `Serializable` interface and was deprecated in PHP 8.1. On its own it had no effect: PDO used the default mode, so `PDO::FETCH_DEFAULT` keeps the result. Ferry writes this itself.",
        "Combined with `FETCH_CLASS`, fetch the column and `unserialize()` it yourself.",
      ],
    },
    before: "$objects = $stmt->fetchAll(PDO::FETCH_CLASS | PDO::FETCH_SERIALIZE, 'Item');",
    after: "$objects = array_map('unserialize', $stmt->fetchAll(PDO::FETCH_COLUMN));",
  },
  "deprecated.pgsql_escaped_question": {
    steps: {
      tr: [
        "PDO_PGSQL artık `$$` ile tırnaklanmış metinleri tanır; bu yüzden onların içindeki soru işaretini `??` ile kaçırmak PHP 8.4'ten beri kullanımdan kalktı.",
        "`$$...$$` içindeki `??`'yi tek `?` yapın. Tırnak dışındaki `??` (yer tutucu kaçışı) olduğu gibi kalır.",
      ],
      en: [
        "PDO_PGSQL now understands `$$`-quoted strings, so escaping question marks inside them as `??` has been deprecated since PHP 8.4.",
        "Change `??` inside `$$...$$` to a single `?`. A `??` outside the quotes (escaping a placeholder) stays as it is.",
      ],
    },
    before: "$pdo->prepare('SELECT $$a ?? b$$, ?');",
    after: "$pdo->prepare('SELECT $$a ? b$$, ?');",
  },
  "deprecated.phar_setstub_length": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `Phar::setStub()`'a bir kaynak ve uzunluk vermek kullanımdan kalktı.",
        "Stub'ı metin olarak okuyup verin: `$phar->setStub(stream_get_contents($fp, $length));`.",
      ],
      en: [
        "Since PHP 8.3, passing a resource and a length to `Phar::setStub()` is deprecated.",
        "Read the stub as a string and pass that: `$phar->setStub(stream_get_contents($fp, $length));`.",
      ],
    },
    before: "$phar->setStub($fp, 100);",
    after: "$phar->setStub(stream_get_contents($fp, 100));",
  },
  "deprecated.pow_zero_negative": {
    steps: {
      tr: [
        "PHP 8.4'ten beri sıfırın negatif bir kuvvetini almak kullanımdan kalktı; sonuç `INF`'ti ve PHP 9'da DivisionByZeroError olacak.",
        "`INF` gerçekten isteniyorsa PHP 8.4'te gelen `fpow()`'u kullanın: `fpow(0, -1)` `INF` verir. Değilse taban sıfırken ne olması gerektiğini ayrıca ele alın.",
      ],
      en: [
        "Since PHP 8.4, raising zero to a negative power is deprecated; the result was `INF`, and PHP 9 will throw DivisionByZeroError.",
        "If `INF` is really wanted, use `fpow()`, added in PHP 8.4: `fpow(0, -1)` gives `INF`. Otherwise handle a zero base separately.",
      ],
    },
    before: "$scale = $base ** -$exp;",
    after: "$scale = $base == 0 ? INF : $base ** -$exp;",
  },
  "deprecated.reflection_method_string": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `new ReflectionMethod('Sinif::metot')` kullanımdan kalktı.",
        "PHP 8.3'te gelen `ReflectionMethod::createFromMethodName('Sinif::metot')`'u ya da iki argümanlı `new ReflectionMethod('Sinif', 'metot')`'u kullanın.",
      ],
      en: [
        "Since PHP 8.4, `new ReflectionMethod('Class::method')` is deprecated.",
        "Use `ReflectionMethod::createFromMethodName('Class::method')`, added in PHP 8.3, or the two-argument `new ReflectionMethod('Class', 'method')`.",
      ],
    },
    before: "$m = new ReflectionMethod($callable);",
    after: "$m = ReflectionMethod::createFromMethodName($callable);",
  },
  "deprecated.sid_constant": {
    steps: {
      tr: [
        "`SID` sabiti, oturum kimliğini URL'de ya da formda taşıyan oturumlarla birlikte PHP 8.4'te kullanımdan kalktı.",
        "Oturum kimliğini çerezde tutun (`session.use_only_cookies = 1`, varsayılan) ve bağlantılara `SID` eklemeyi bırakın. Çerez kullanılırken `SID` zaten boş metindi.",
      ],
      en: [
        "PHP 8.4 deprecated the `SID` constant, along with sessions that carry the session ID in the URL or a form.",
        "Keep the session ID in a cookie (`session.use_only_cookies = 1`, the default) and stop adding `SID` to links. While cookies are used, `SID` was an empty string anyway.",
      ],
    },
    before: "echo '<a href=\"next.php?' . SID . '\">Next</a>';",
    after: "echo '<a href=\"next.php\">Next</a>';",
  },
  "deprecated.soap_ssl_method": {
    steps: {
      tr: [
        "`SoapClient`'ın `ssl_method` seçeneği PHP 8.1'de kullanımdan kalktı.",
        "TLS sürümünü bir akış bağlamıyla verin: `'stream_context' => stream_context_create(['ssl' => ['crypto_method' => STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT]])`.",
      ],
      en: [
        "PHP 8.1 deprecated `SoapClient`'s `ssl_method` option.",
        "Set the TLS version through a stream context: `'stream_context' => stream_context_create(['ssl' => ['crypto_method' => STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT]])`.",
      ],
    },
    before: "new SoapClient($wsdl, ['ssl_method' => SOAP_SSL_METHOD_TLS]);",
    after: "new SoapClient($wsdl, ['stream_context' => stream_context_create(['ssl' => ['crypto_method' => STREAM_CRYPTO_METHOD_TLSv1_2_CLIENT]])]);",
  },
  "deprecated.splfixedarray_wakeup": {
    steps: {
      tr: [
        "`SplFixedArray::__wakeup()` PHP 8.4'te kullanımdan kalktı; serileştirmeden geri açma artık `__unserialize()` ile yapılıyor ve çağrı nesnede hiçbir şeyi değiştirmiyor.",
        "Çağrıyı silin; Ferry bunu kendisi yapar. Alt sınıfta `__wakeup()` tanımlıysa o mantığı `__unserialize()`'a taşıyın.",
      ],
      en: [
        "PHP 8.4 deprecated `SplFixedArray::__wakeup()`; unserialization now goes through `__unserialize()`, and the call changes nothing on the object.",
        "Delete the call; Ferry does this itself. If a subclass defines `__wakeup()`, move that logic to `__unserialize()`.",
      ],
    },
    before: "$items = unserialize($data);\n$items->__wakeup();",
    after: "$items = unserialize($data);",
  },
  "deprecated.sqlite3_exceptions_off": {
    steps: {
      tr: [
        "`SQLite3::enableExceptions(false)` PHP 8.3'te kullanımdan kalktı; istisnalar ileride her zaman açık olacak.",
        "Çağrıyı silin ve false dönüşünü kontrol eden kodu `try { ... } catch (Exception $e) { ... }` ile değiştirin; SQLite3 hata durumunda `Exception` fırlatır.",
      ],
      en: [
        "PHP 8.3 deprecated `SQLite3::enableExceptions(false)`; exceptions will always be on.",
        "Remove the call and replace code that checks for a false result with `try { ... } catch (Exception $e) { ... }`; SQLite3 throws `Exception` on errors.",
      ],
    },
    before: "$db->enableExceptions(false);\nif ($db->exec($sql) === false) {\n    log_error($db->lastErrorMsg());\n}",
    after: "try {\n    $db->exec($sql);\n} catch (Exception $e) {\n    log_error($e->getMessage());\n}",
  },
  "deprecated.ssl_capture_session_meta": {
    steps: {
      tr: [
        "`capture_session_meta` SSL bağlam seçeneği PHP 7.0'da kullanımdan kalktı.",
        "Seçeneği silin ve bağlantının şifreleme bilgisini `stream_get_meta_data($stream)['crypto']`'dan okuyun.",
      ],
      en: [
        "PHP 7.0 deprecated the `capture_session_meta` SSL context option.",
        "Remove the option and read the connection's crypto details from `stream_get_meta_data($stream)['crypto']`.",
      ],
    },
    before: "$ctx = stream_context_create(['ssl' => ['capture_session_meta' => true]]);",
    after: "$ctx = stream_context_create(['ssl' => []]);\n$crypto = stream_get_meta_data($stream)['crypto'];",
  },
  "deprecated.string_decrement": {
    steps: {
      tr: [
        "Sayısal olmayan bir metni `--` ile azaltmak hiçbir şey yapmıyordu ve PHP 8.3'te kullanımdan kalktı: `$s = 'abc'; $s--;` sonrasında `$s` yine `'abc'`'dir.",
        "Eski davranışı korumak için `--`'yi silin. Azaltma gerçekten isteniyorsa PHP 8.3'te gelen `str_decrement()`'i kullanın; ama `str_decrement('abc')` `'abb'` verir, yani sonuç değişir.",
      ],
      en: [
        "Decrementing a non-numeric string with `--` did nothing and was deprecated in PHP 8.3: after `$s = 'abc'; $s--;`, `$s` is still `'abc'`.",
        "To keep the old behaviour, remove the `--`. If decrementing was really meant, use `str_decrement()`, added in PHP 8.3; but `str_decrement('abc')` gives `'abb'`, so the result changes.",
      ],
    },
    before: "$code = $row['code'];\n$code--;\nreturn $code;",
    after: "$code = $row['code'];\nreturn $code;",
  },
  "deprecated.string_increment": {
    steps: {
      tr: [
        "PHP 8.5'ten beri sayısal olmayan bir metni `++` ile artırmak kullanımdan kalktı.",
        "PHP 8.3'te gelen `str_increment()`'i kullanın; harf ve rakamlardan oluşan metinlerde aynı sonucu verir (`'Az'` → `'Ba'`). Ferry bilinen metinlerde bunu kendisi yapar.",
        "`str_increment()` boş metinde ve harf ya da rakam olmayan karakter içeren metinlerde ValueError fırlatır; `++` ise `''`'yi `'1'` yapıyordu. Değerin ne olabileceğine bakın.",
      ],
      en: [
        "Since PHP 8.5, incrementing a non-numeric string with `++` is deprecated.",
        "Use `str_increment()`, added in PHP 8.3; it gives the same result for strings of letters and digits (`'Az'` → `'Ba'`). Ferry does this itself for strings it knows.",
        "`str_increment()` throws a ValueError for an empty string and for strings with characters other than letters and digits, while `++` turned `''` into `'1'`. Check what the value can be.",
      ],
    },
    before: "$next = $id;\n$next++;",
    after: "$next = str_increment($id);",
  },
  "deprecated.underscore_class_name": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `_` sınıf adı olarak kullanımdan kalktı.",
        "Sınıfı ve onu kullanan her yeri yeniden adlandırın; Rector'un `RenameClassRector` kuralı hepsini birlikte değiştirebilir.",
      ],
      en: [
        "Since PHP 8.4, `_` as a class name is deprecated.",
        "Rename the class and every place that uses it; Rector's `RenameClassRector` can change them all at once.",
      ],
    },
    before: "class _ {",
    after: "class Translator {",
  },
  "deprecated.unregister_autoload_call": {
    steps: {
      tr: [
        "Bütün yükleyicileri kaldırmak için `spl_autoload_call`'u kaydından çıkarmak PHP 8.5'te kullanımdan kalktı.",
        "Kayıtlı yükleyicileri tek tek kaldırın: `foreach (spl_autoload_functions() as $loader) { spl_autoload_unregister($loader); }`.",
      ],
      en: [
        "PHP 8.5 deprecated unregistering `spl_autoload_call` to remove every autoloader.",
        "Remove the registered autoloaders one by one: `foreach (spl_autoload_functions() as $loader) { spl_autoload_unregister($loader); }`.",
      ],
    },
    before: "spl_autoload_unregister('spl_autoload_call');",
    after: "foreach (spl_autoload_functions() as $loader) {\n    spl_autoload_unregister($loader);\n}",
  },
  "deprecated.unserialize_upper_s": {
    steps: {
      tr: [
        "PHP 8.4'ten beri büyük `S:` etiketli metinleri geri açmak kullanımdan kalktı. Bu etiketi eski PHP sürümleri ve bazı kitaplıklar yazıyordu; içindeki baytlar `\\xx` kaçışlarıyla yazılır.",
        "Saklanan veriyi bir kez okuyup yeniden serileştirin: hedef sürümde `serialize(unserialize($data))` küçük `s:` ile yazar. Bu dönüşümü veriyi yazan yerde de yapın ki yeni kayıtlar eski biçimde oluşmasın.",
      ],
      en: [
        "Since PHP 8.4, unserializing strings with the uppercase `S:` tag is deprecated. Old PHP versions and some libraries wrote this tag; the bytes inside are written as `\\xx` escapes.",
        "Read the stored data once and serialize it again: on the target version `serialize(unserialize($data))` writes the lowercase `s:`. Do this where the data is written too, so new records aren't created in the old form.",
      ],
    },
  },
  "deprecated.void_by_reference": {
    steps: {
      tr: [
        "PHP 8.1'den beri `void` dönüşlü bir fonksiyonu referansla döndüren (`function &f(): void`) olarak tanımlamak kullanımdan kalktı; döndürülecek bir değer yok.",
        "`&`'yi silin.",
      ],
      en: [
        "Since PHP 8.1, declaring a `void` function as returning by reference (`function &f(): void`) is deprecated; there is nothing to return.",
        "Remove the `&`.",
      ],
    },
    before: "function &reset(): void {",
    after: "function reset(): void {",
  },
  "deprecated.zip_fl_recompress": {
    steps: {
      tr: [
        "`ZipArchive::FL_RECOMPRESS` PHP 8.3'te kullanımdan kalktı ve libzip'in sonraki sürümlerinde kaldırılacak.",
        "Kullanıldığı yeri kaldırın; `|` ile birleştirildiği yerlerde yalnızca bu sabiti çıkarın. Ferry `setCompressionName()` ve `setCompressionIndex()` çağrılarında bunu kendisi yapar.",
      ],
      en: [
        "PHP 8.3 deprecated `ZipArchive::FL_RECOMPRESS`, and later libzip versions will remove it.",
        "Remove its use; in a `|` combination, take out only this constant. Ferry does this itself in `setCompressionName()` and `setCompressionIndex()` calls.",
      ],
    },
    before: "$zip->setCompressionName('a.txt', ZipArchive::CM_DEFLATE, ZipArchive::FL_RECOMPRESS);",
    after: "$zip->setCompressionName('a.txt', ZipArchive::CM_DEFLATE);",
  },
  "removed.ai_idn_constant": {
    steps: {
      tr: [
        "`AI_IDN_ALLOW_UNASSIGNED` ve `AI_IDN_USE_STD3_ASCII_RULES` glibc 2.28'den beri hiçbir şey yapmıyor; PHP 7.4'te kullanımdan kalktı, 8.0'da kaldırıldı.",
        "`|` maskelerinden çıkarın; tek başına kullanıldığı yerde 0 yazın. glibc 2.28 ve sonrasında arama aynı kalır. Ferry bunu kendisi yapar; bir değeri onunla karşılaştıran ya da `&` ile sınayan kodu elle değiştirin (sonuç her zaman 0'dı).",
      ],
      en: [
        "`AI_IDN_ALLOW_UNASSIGNED` and `AI_IDN_USE_STD3_ASCII_RULES` have done nothing since glibc 2.28; PHP 7.4 deprecated them and 8.0 removed them.",
        "Drop them from `|` masks; where one stands alone, write 0. The lookup stays the same on glibc 2.28 and later. Ferry does this itself; change code that compares a value with them or tests them with `&` by hand (the result was always 0).",
      ],
    },
    before: "if ($flags & AI_IDN_ALLOW_UNASSIGNED) {",
    after: "if (false) {",
  },
  "removed.ambiguous_trait_alias": {
    steps: {
      tr: [
        "Bu metot kullanılan trait'lerin birden fazlasında var. PHP 8.0'dan beri takma ad hangisini kastettiğini `Trait::metot` ile söylemeli; yoksa sınıf derlenmez.",
        "Takma adı trait adıyla yazın: `T1::f as g;`. Hangi trait'in metodu kullanılıyorsa (`insteadof` ile seçileni) onun adını verin.",
      ],
      en: [
        "This method exists in more than one of the traits used. Since PHP 8.0 the alias has to say which one with `Trait::method`, or the class doesn't compile.",
        "Write the alias with the trait's name: `T1::f as g;`. Name the trait whose method is used (the one chosen with `insteadof`).",
      ],
    },
    before: "use T1, T2 { T1::f insteadof T2; f as g; }",
    after: "use T1, T2 { T1::f insteadof T2; T1::f as g; }",
  },
  "removed.array_object_enum": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `ArrayObject` enum kabul etmez ve `InvalidArgumentException` fırlatır.",
        "Enum'un kendisi yerine gereken değerleri bir diziye koyup verin, örneğin `['case' => $enum->name, 'value' => $enum->value]`.",
      ],
      en: [
        "Since PHP 8.5, `ArrayObject` doesn't accept enums and throws `InvalidArgumentException`.",
        "Put the values you need into an array and pass that instead of the enum itself, such as `['case' => $enum->name, 'value' => $enum->value]`.",
      ],
    },
    before: "$view = new ArrayObject(Status::Active);",
    after: "$view = new ArrayObject(['name' => Status::Active->name, 'value' => Status::Active->value]);",
  },
  "removed.attribute_on_non_class": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `#[Attribute]` soyut sınıflara, arayüzlere, trait'lere ve enum'lara uygulanamaz; dosya derlenmez.",
        "`#[Attribute]`'u silin. Bu tür bir öznitelik zaten hiçbir zaman örneklenemiyordu; öznitelik olarak kullanılacaksa onu somut bir sınıf yapın.",
      ],
      en: [
        "Since PHP 8.5, `#[Attribute]` can't be applied to abstract classes, interfaces, traits or enums; the file doesn't compile.",
        "Remove `#[Attribute]`. Such an attribute could never be instantiated anyway; if it's meant to be used as an attribute, make it a concrete class.",
      ],
    },
    before: "#[Attribute]\nabstract class Route {}",
    after: "abstract class Route {}",
  },
  "removed.break_outside_loop": {
    steps: {
      tr: [
        "Döngü ya da `switch` dışındaki `break` ve `continue` PHP 7.0'dan beri ölümcül hatadır; dosya derlenmez. PHP 5'te fonksiyonun içindeki bir `break`, include edilen dosyayı ya da fonksiyonu bitirmek için kullanılıyordu.",
        "Fonksiyondan çıkmak için `return`, include edilen dosyayı bitirmek için dosyanın en üst düzeyinde `return` yazın. Ferry dönüş tipi `void` olan ya da olmayan fonksiyonlarda düz `break;`'i kendisi `return;` yapar.",
        "Kalan kodun çalışmaması isteniyorsa onu bir koşulun içine alın.",
      ],
      en: [
        "`break` and `continue` outside a loop or `switch` have been a fatal error since PHP 7.0; the file doesn't compile. On PHP 5 a `break` inside a function was used to end it or the included file.",
        "Write `return` to leave a function, and `return` at the top level of an included file to end it. Ferry turns a plain `break;` into `return;` itself in functions without a non-void return type.",
        "If the rest of the code shouldn't run, put it inside a condition.",
      ],
    },
    before: "if (!$ready) {\n    break;\n}",
    after: "if (!$ready) {\n    return;\n}",
  },
  "removed.builtin_method": {
    steps: {
      tr: [
        "Bu yerleşik metot PHP'den kaldırıldı; çağrılması ölümcül hatadır. Bulgunun başlığı metodu, önerilen satır varsa yerine geçeni gösterir.",
        "PHP 4 tarzı kurucu çağrıları (`parent::SoapClient(...)` gibi) `parent::__construct(...)` olmalıdır. Diğer metotlar için PHP kılavuzunda sınıfın sayfasına bakın.",
      ],
      en: [
        "This built-in method was removed from PHP; calling it is a fatal error. The finding's title names the method, and the suggested line shows the replacement when there is one.",
        "PHP 4 style constructor calls (such as `parent::SoapClient(...)`) should be `parent::__construct(...)`. For other methods, look at the class's page in the PHP manual.",
      ],
    },
    before: "parent::SoapClient($wsdl, $options);",
    after: "parent::__construct($wsdl, $options);",
  },
  "removed.bzcompress_range": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `bzcompress()` aralık dışındaki blok boyutu ve iş faktöründe ValueError fırlatır.",
        "Blok boyutunu 1 ile 9, iş faktörünü 0 ile 250 arasında verin; 9 ve 0 en yaygın seçimlerdir.",
      ],
      en: [
        "Since PHP 8.5, `bzcompress()` throws a ValueError for an out-of-range block size or work factor.",
        "Pass a block size from 1 to 9 and a work factor from 0 to 250; 9 and 0 are the usual choices.",
      ],
    },
    before: "bzcompress($data, 10);",
    after: "bzcompress($data, 9);",
  },
  "removed.closure_use_variable": {
    steps: {
      tr: [
        "PHP 7.1'den beri `$this`, süper global değişkenler ve bir parametreyle aynı adlı değişkenler `use()` ile bağlanamaz; dosya derlenmez.",
        "Değişkeni `use()`'dan silin. `$this` kapanışta zaten kullanılabilir; süper global değişkenlere her yerden erişilir; parametreyle çakışan değişkeni başka bir adla bağlayın.",
      ],
      en: [
        "Since PHP 7.1, `$this`, superglobals and variables named like a parameter can't be bound with `use()`; the file doesn't compile.",
        "Remove the variable from `use()`. `$this` is available in the closure anyway, superglobals are reachable everywhere, and a variable that clashes with a parameter should be bound under another name.",
      ],
    },
    before: "$f = function () use ($this) { return $this->id; };",
    after: "$f = function () { return $this->id; };",
  },
  "removed.create_function": {
    steps: {
      tr: [
        "`create_function()` PHP 8.0'da kaldırıldı. Rector sabit kod metinlerini kapanışa çevirir.",
        "Kod metni çalışırken oluşturuluyorsa ya da fonksiyon adı bir metin olarak veriliyorsa (`'create_function'`) mantığı elle bir kapanışa yazın. Metindeki değişkenleri kapanışa parametre ya da `use()` ile verin.",
      ],
      en: [
        "`create_function()` was removed in PHP 8.0. Rector turns literal code strings into closures.",
        "When the code string is built at runtime, or the function name is passed as a string (`'create_function'`), write the logic as a closure by hand. Pass the variables the string used as parameters or with `use()`.",
      ],
    },
    before: "$cmp = create_function('$a, $b', 'return $a[\"' . $key . '\"] <=> $b[\"' . $key . '\"];');",
    after: "$cmp = function ($a, $b) use ($key) {\n    return $a[$key] <=> $b[$key];\n};",
  },
  "removed.csv_separator_length": {
    steps: {
      tr: [
        "PHP 8.4'ten beri CSV fonksiyonları boş ya da birden uzun ayraçta ValueError fırlatır.",
        "Tek karakterlik bir ayraç verin. Birden çok karakterli bir ayraç gerekiyorsa satırı `explode()` ile bölün.",
      ],
      en: [
        "Since PHP 8.4, the CSV functions throw a ValueError for an empty separator or one longer than a character.",
        "Pass a one-character separator. If a multi-character separator is needed, split the line with `explode()`.",
      ],
    },
    before: "$fields = str_getcsv($line, '||');",
    after: "$fields = explode('||', $line);",
  },
  "removed.curl_multi_select_negative": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `curl_multi_select()` negatif zaman aşımında ValueError fırlatır.",
        "0 ya da pozitif bir zaman aşımı verin; varsayılan 1 saniyedir.",
      ],
      en: [
        "Since PHP 8.4, `curl_multi_select()` throws a ValueError for a negative timeout.",
        "Pass 0 or a positive timeout; the default is 1 second.",
      ],
    },
    before: "curl_multi_select($mh, -1);",
    after: "curl_multi_select($mh, 1.0);",
  },
  "removed.curl_safe_upload_off": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `CURLOPT_SAFE_UPLOAD` kapatılamaz; `'@/yol/dosya'` biçimindeki yüklemeler artık çalışmaz.",
        "Dosyayı bir `CURLFile` nesnesiyle verin ve `CURLOPT_SAFE_UPLOAD` satırını silin.",
      ],
      en: [
        "Since PHP 7.0, `CURLOPT_SAFE_UPLOAD` can't be turned off, so `'@/path/file'` uploads no longer work.",
        "Pass the file as a `CURLFile` object and remove the `CURLOPT_SAFE_UPLOAD` line.",
      ],
    },
    before: "curl_setopt($ch, CURLOPT_SAFE_UPLOAD, false);\ncurl_setopt($ch, CURLOPT_POSTFIELDS, ['file' => '@' . $path]);",
    after: "curl_setopt($ch, CURLOPT_POSTFIELDS, ['file' => new CURLFile($path)]);",
  },
  "removed.custom_assert": {
    steps: {
      tr: [
        "`assert()` adında bir fonksiyon tanımlamak PHP 7.3'te kullanımdan kalktı, PHP 8.0'dan beri derleme hatasıdır; `assert` derleyicinin özel ele aldığı bir addır.",
        "Fonksiyonu yeniden adlandırın ve çağrıldığı yerleri değiştirin. Sınıf metotları bundan etkilenmez.",
      ],
      en: [
        "Declaring a function named `assert()` was deprecated in PHP 7.3 and is a compile error since PHP 8.0; `assert` is a name the compiler handles specially.",
        "Rename the function and change the places that call it. Class methods aren't affected.",
      ],
    },
    before: "namespace App;\nfunction assert($cond) {",
    after: "namespace App;\nfunction ensure($cond) {",
  },
  "removed.data_stream_write": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `data:` akışlarına yazılamaz; yazma false döndürür. PHP 7.4'te `file_put_contents('data://text/plain,foo', 'x')` 1 döndürüyordu ama yazılan hiçbir yere gitmiyordu.",
        "Geçici bellek için `php://memory` ya da `php://temp` kullanın. Yalnızca okumak için açılan `data:` akışları çalışmaya devam eder.",
      ],
      en: [
        "Since PHP 8.0, `data:` streams can't be written to; writing returns false. On PHP 7.4, `file_put_contents('data://text/plain,foo', 'x')` returned 1, but what was written went nowhere.",
        "Use `php://memory` or `php://temp` for in-memory data. `data:` streams opened only for reading keep working.",
      ],
    },
    before: "$fp = fopen('data:text/plain,', 'w');",
    after: "$fp = fopen('php://memory', 'w+');",
  },
  "removed.delete_in_identifier": {
    steps: {
      tr: [
        "PHP 7.1'den beri ASCII 0x7F (Delete) karakteri tırnaksız adlarda kullanılamaz; dosya derlenmez.",
        "Karakteri silin. Çoğu düzenleyici bu karakteri göstermez; dosyayı onaltılık görünümde açarak ya da `sed -i 's/\\x7f//g' dosya.php` ile temizleyebilirsiniz.",
      ],
      en: [
        "Since PHP 7.1, the ASCII 0x7F (Delete) character isn't allowed in unquoted names; the file doesn't compile.",
        "Remove the character. Most editors don't show it; you can clean it in a hex view or with `sed -i 's/\\x7f//g' file.php`.",
      ],
    },
  },
  "removed.duplicate_parameter": {
    steps: {
      tr: [
        "PHP 7.0'dan beri iki parametre aynı adı taşıyamaz; dosya derlenmez. PHP 5'te son parametre geçerli oluyordu.",
        "Parametrelerden birini yeniden adlandırın. PHP 5'teki davranışı korumak için gövde aynı adı kullanmaya devam ederse sonraki parametrenin değerini almalıdır; öncekini kullanılmayan bir adla değiştirin.",
      ],
      en: [
        "Since PHP 7.0, two parameters can't share a name; the file doesn't compile. On PHP 5 the last one won.",
        "Rename one of them. To keep the PHP 5 behaviour, the body should keep reading the later parameter under the shared name; give the earlier one an unused name.",
      ],
    },
    before: "function handle($a, $b, $a) {",
    after: "function handle($unused, $b, $a) {",
  },
  "removed.dynamic_scope_call": {
    steps: {
      tr: [
        "`extract()`, `compact()`, `func_get_args()` ve benzerleri PHP 7.1'den beri dinamik olarak (`$f()` ya da `call_user_func()` ile) çağrılamaz. 7.1'de uyarı verir, PHP 8.0'dan beri Error fırlatır.",
        "Fonksiyonu doğrudan çağırın. Çağrılacak fonksiyon bir değişkenden geliyorsa bu fonksiyonlar için ayrı bir dal yazın.",
      ],
      en: [
        "`extract()`, `compact()`, `func_get_args()` and similar functions can't be called dynamically (through `$f()` or `call_user_func()`) since PHP 7.1. It warns on 7.1 and throws an Error since PHP 8.0.",
        "Call the function directly. If the function to call comes from a variable, write a separate branch for these.",
      ],
    },
    before: "$vars = call_user_func('compact', 'id', 'name');",
    after: "$vars = compact('id', 'name');",
  },
  "removed.empty_list": {
    steps: {
      tr: [
        "PHP 7.0'dan beri boş bir `list()` ataması (`list() = $a;` ya da `list(,,) = $a;`) derleme hatasıdır.",
        "Atamayı silin; hiçbir değişkene değer vermiyordu. Sağ taraf bir fonksiyon çağrısıysa ve çalışması gerekiyorsa yalnızca çağrıyı bırakın.",
      ],
      en: [
        "Since PHP 7.0, an empty `list()` assignment (`list() = $a;` or `list(,,) = $a;`) is a compile error.",
        "Remove the assignment; it assigned nothing. If the right side is a function call that has to run, keep only the call.",
      ],
    },
    before: "list(,,) = next_row($result);",
    after: "next_row($result);",
  },
  "removed.extends_final_class": {
    steps: {
      tr: [
        "Bu yerleşik sınıf artık `final`; onu genişleten bir sınıf derlenmez.",
        "Sınıfı genişletmek yerine sarın: nesneyi bir özellikte tutun ve gereken metotları ona yönlendirin.",
      ],
      en: [
        "This built-in class is `final` now, so a class that extends it doesn't compile.",
        "Wrap the class instead of extending it: keep an instance in a property and forward the methods you need to it.",
      ],
    },
    before: "class Money extends GMP {\n}",
    after: "class Money {\n    public function __construct(private GMP $value) {}\n}",
  },
  "removed.extract_flags_zero": {
    steps: {
      tr: [
        "PHP 7.4'ten beri `SplPriorityQueue::setExtractFlags(0)` RuntimeException fırlatır; PHP 5.6'da hata vermiyordu.",
        "Ne çıkarılacağını söyleyin: veri için `SplPriorityQueue::EXTR_DATA` (varsayılan), öncelik için `EXTR_PRIORITY`, ikisi için `EXTR_BOTH`.",
      ],
      en: [
        "Since PHP 7.4, `SplPriorityQueue::setExtractFlags(0)` throws a RuntimeException; on PHP 5.6 it didn't.",
        "Say what to extract: `SplPriorityQueue::EXTR_DATA` for the data (the default), `EXTR_PRIORITY` for the priority, `EXTR_BOTH` for both.",
      ],
    },
    before: "$queue->setExtractFlags(0);",
    after: "$queue->setExtractFlags(SplPriorityQueue::EXTR_DATA);",
  },
  "removed.fetch_all_into": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `fetchAll()` `PDO::FETCH_INTO` modunu kabul etmez; tek bir nesneyi doldurmak zaten satır satır yapılan bir iştir.",
        "Modu `setFetchMode(PDO::FETCH_INTO, $object)` ile ayarlayıp `fetch()`'i bir döngüde çağırın.",
      ],
      en: [
        "Since PHP 8.5, `fetchAll()` doesn't accept the `PDO::FETCH_INTO` mode; filling a single object is a row-by-row job anyway.",
        "Set the mode with `setFetchMode(PDO::FETCH_INTO, $object)` and call `fetch()` in a loop.",
      ],
    },
    before: "$rows = $stmt->fetchAll(PDO::FETCH_INTO);",
    after: "$stmt->setFetchMode(PDO::FETCH_INTO, $row);\nwhile ($stmt->fetch()) {\n    $rows[] = clone $row;\n}",
  },
  "removed.fetch_class_named_args": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `FETCH_CLASS`'ın kurucu argümanları metin anahtarlı olamaz.",
        "Anahtarları kaldırıp argümanları kurucudaki sırayla verin: `array_values($args)`.",
      ],
      en: [
        "Since PHP 8.5, the constructor arguments of `FETCH_CLASS` can't have string keys.",
        "Drop the keys and pass the arguments in the constructor's order: `array_values($args)`.",
      ],
    },
    before: "$stmt->setFetchMode(PDO::FETCH_CLASS, User::class, ['role' => 'admin']);",
    after: "$stmt->setFetchMode(PDO::FETCH_CLASS, User::class, ['admin']);",
  },
  "removed.fetch_props_late_mode": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `PDO::FETCH_PROPS_LATE` yalnızca `PDO::FETCH_CLASS` ile kullanılabilir; başka modlarda zaten etkisi yoktu.",
        "Başka bir modla birlikteyse `FETCH_PROPS_LATE`'i çıkarın; kurucunun özelliklerden önce çalışması isteniyorsa `FETCH_CLASS` ile kullanın.",
      ],
      en: [
        "Since PHP 8.5, `PDO::FETCH_PROPS_LATE` can only be used with `PDO::FETCH_CLASS`; with other modes it had no effect anyway.",
        "With another mode, take `FETCH_PROPS_LATE` out; if the constructor should run before the properties are set, use it with `FETCH_CLASS`.",
      ],
    },
    before: "$rows = $stmt->fetchAll(PDO::FETCH_ASSOC | PDO::FETCH_PROPS_LATE);",
    after: "$rows = $stmt->fetchAll(PDO::FETCH_ASSOC);",
  },
  "removed.fgetss": {
    steps: {
      tr: [
        "`fgetss()` ve `gzgetss()` PHP 8.0'da kaldırıldı. Ferry okumayı `fgets()` ile yapıp `strip_tags()` uygulayan ve dosya sonunda false'u koruyan bir ifadeye çevirir.",
        "Okumayı yalnızca `strip_tags()` ile sarmayın: dosya sonundaki false `''` olur ve `while (... !== false)` döngüsü bitmez. Bir etiket iki satıra bölünmüşse ikinci yarısı artık sonraki satırda kalır; gerekirse dosyanın tamamını okuyup `strip_tags()` uygulayın.",
      ],
      en: [
        "`fgetss()` and `gzgetss()` were removed in PHP 8.0. Ferry turns the read into an expression that reads with `fgets()`, applies `strip_tags()` and keeps the false at end of file.",
        "Don't just wrap the read in `strip_tags()`: the false at end of file becomes `''` and a `while (... !== false)` loop never ends. When a tag is split across lines, its second half now stays in the next line; if that matters, read the whole file and apply `strip_tags()` once.",
      ],
    },
    before: "while (($line = fgetss($fp, 4096, $allowed)) !== false) {",
    after: "while (($line = fgets($fp, 4096)) !== false) {\n    $line = strip_tags($line, $allowed);",
  },
  "removed.file_append_flag": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `file()` geçersiz bayraklarda ValueError fırlatır ve `FILE_APPEND` bunlardan biridir; `file()` için hiçbir zaman bir anlamı yoktu.",
        "Bayrağı çıkarın. Dosyaya eklemek isteniyorsa `file_put_contents($path, $data, FILE_APPEND)` kullanın.",
      ],
      en: [
        "Since PHP 8.3, `file()` throws a ValueError for invalid flags, and `FILE_APPEND` is one of them; it never meant anything for `file()`.",
        "Take the flag out. To append to a file, use `file_put_contents($path, $data, FILE_APPEND)`.",
      ],
    },
    before: "$lines = file($path, FILE_APPEND | FILE_IGNORE_NEW_LINES);",
    after: "$lines = file($path, FILE_IGNORE_NEW_LINES);",
  },
  "removed.filter_flag_host_required": {
    steps: {
      tr: [
        "`FILTER_FLAG_HOST_REQUIRED` ve `FILTER_FLAG_SCHEME_REQUIRED` PHP 8.0'da kaldırıldı; `FILTER_VALIDATE_URL` zaten şema ve host ister.",
        "Bayrağı çıkarın; tek başına kullanıldığı yerde 0 yazın. Ferry bunu kendisi yapar; bir değeri onunla karşılaştıran ya da `&` ile sınayan kodu elle değiştirin.",
      ],
      en: [
        "`FILTER_FLAG_HOST_REQUIRED` and `FILTER_FLAG_SCHEME_REQUIRED` were removed in PHP 8.0; `FILTER_VALIDATE_URL` already requires a scheme and a host.",
        "Take the flag out; where it stands alone, write 0. Ferry does this itself; change code that compares a value with it or tests it with `&` by hand.",
      ],
    },
    before: "filter_var($url, FILTER_VALIDATE_URL, FILTER_FLAG_HOST_REQUIRED);",
    after: "filter_var($url, FILTER_VALIDATE_URL);",
  },
  "removed.gettext_empty_domain": {
    steps: {
      tr: [
        "PHP 8.4'ten beri gettext fonksiyonları boş domain'de ValueError fırlatır.",
        "Domain'in adını verin. Geçerli domain'i okumak için `textdomain(null)` kullanın.",
      ],
      en: [
        "Since PHP 8.4, the gettext functions throw a ValueError for an empty domain.",
        "Pass the domain's name. To read the current domain, use `textdomain(null)`.",
      ],
    },
    before: "$current = textdomain('');",
    after: "$current = textdomain(null);",
  },
  "removed.gmp_setbit_negative": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `gmp_setbit()` negatif bit indisi kabul etmez.",
        "İndisi 0 ya da pozitif verin; bit 0 en düşük bittir.",
      ],
      en: [
        "Since PHP 7.0, `gmp_setbit()` doesn't accept a negative bit index.",
        "Pass an index of 0 or more; bit 0 is the lowest bit.",
      ],
    },
  },
  "removed.hash_hmac_non_crypto": {
    steps: {
      tr: [
        "PHP 7.2'den beri `crc32`, `adler32` ya da `fnv` gibi kriptografik olmayan algoritmalar HMAC için kullanılamaz.",
        "`sha256` gibi kriptografik bir algoritma kullanın. Sonuç farklı olacağı için kaydedilmiş HMAC değerleri yeniden üretilmelidir. Yalnızca bir sağlama toplamı gerekiyorsa anahtarsız `hash('crc32b', $data)` kullanın.",
      ],
      en: [
        "Since PHP 7.2, non-cryptographic algorithms such as `crc32`, `adler32` or `fnv` can't be used for HMAC.",
        "Use a cryptographic algorithm such as `sha256`. The result differs, so stored HMAC values have to be produced again. If only a checksum is needed, use `hash('crc32b', $data)` without a key.",
      ],
    },
    before: "$mac = hash_hmac('crc32', $data, $key);",
    after: "$mac = hash_hmac('sha256', $data, $key);",
  },
  "removed.heredoc_label_in_body": {
    steps: {
      tr: [
        "PHP 7.3'ten beri heredoc ve nowdoc kapanış etiketi girintili olabilir; bu yüzden metnin içinde satır başında (boşluklardan sonra) duran etiket sözcüğü heredoc'u bitirir ve dosya derlenmez.",
        "Metinde geçmeyen bir etiket seçin: `<<<EOT` yerine `<<<SQL_END` gibi.",
      ],
      en: [
        "Since PHP 7.3, the closing label of a heredoc or nowdoc can be indented, so the label word at the start of a line inside the text (after spaces) ends the heredoc and the file doesn't compile.",
        "Choose a label that doesn't appear in the text, such as `<<<SQL_END` instead of `<<<EOT`.",
      ],
    },
    before: "$help = <<<END\n  Press Enter to continue,\n  END to stop.\nEND;",
    after: "$help = <<<HELP_TEXT\n  Press Enter to continue,\n  END to stop.\nHELP_TEXT;",
  },
  "removed.image_quality_range": {
    steps: {
      tr: [
        "PHP 8.4'ten beri GD aralık dışındaki kalite değerlerinde ValueError fırlatır.",
        "`imagejpeg()`, `imagewebp()` ve `imageavif()` için 0 ile 100, `imagepng()` için 0 ile 9 arasında bir değer verin; varsayılan için -1 yazın.",
      ],
      en: [
        "Since PHP 8.4, GD throws a ValueError for quality values out of range.",
        "Pass 0 to 100 for `imagejpeg()`, `imagewebp()` and `imageavif()`, and 0 to 9 for `imagepng()`; write -1 for the default.",
      ],
    },
    before: "imagepng($image, $path, 15);",
    after: "imagepng($image, $path, 9);",
  },
  "removed.imagecropauto_default": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `imagecropauto()` -1 modunu kabul etmez.",
        "-1 yerine `IMG_CROP_DEFAULT` yazın.",
      ],
      en: [
        "Since PHP 8.0, `imagecropauto()` no longer accepts the mode -1.",
        "Write `IMG_CROP_DEFAULT` instead of -1.",
      ],
    },
    before: "imagecropauto($image, -1);",
    after: "imagecropauto($image, IMG_CROP_DEFAULT);",
  },
  "removed.incompatible_override": {
    steps: {
      tr: [
        "Bu metot yerleşik bir sınıfın ya da arayüzün metodunu eziyor ama imzası PHP 8'deki imzaya uymuyor; sınıf yüklenirken ölümcül hata verir. Önerilen satır hedef imzayı gösterir.",
        "Ferry parametre tiplerini genişletebiliyor, eksik varsayılanları ve parametreleri ekleyebiliyorsa bunu kendisi yapar. `static` farkı, referans farkı ya da uymayan bir dönüş tipi varsa imzayı önerilen satıra göre elle düzeltin.",
        "Dönüş tipi yalnızca \"geçici\" (tentative) olarak uyuşmuyorsa metoda `#[\\ReturnTypeWillChange]` eklemek yeterlidir.",
      ],
      en: [
        "This method overrides a built-in class or interface method, but its signature doesn't match PHP 8's; loading the class is a fatal error. The suggested line shows the target signature.",
        "Ferry does this itself when it can widen the parameter types and add missing defaults and parameters. With a `static` difference, a by-reference difference or a return type that doesn't fit, fix the signature by hand from the suggested line.",
        "If only a tentative return type differs, adding `#[\\ReturnTypeWillChange]` to the method is enough.",
      ],
    },
    before: "class Log extends SplFileObject {\n    public function fwrite($data, $length) {",
    after: "class Log extends SplFileObject {\n    public function fwrite($data, $length = 0) {",
  },
  "removed.interface_constant_visibility": {
    steps: {
      tr: [
        "PHP 8.3'ten beri bir arayüz sabitini private ya da protected olarak yeniden tanımlamak derleme hatasıdır.",
        "Sabiti public yapın ya da sınıftaki tanımı silip arayüzünkini kullanın.",
      ],
      en: [
        "Since PHP 8.3, redeclaring an interface constant as private or protected is a compile error.",
        "Make the constant public, or remove the class's declaration and use the interface's.",
      ],
    },
    before: "class Cache implements Store {\n    protected const TTL = 60;\n}",
    after: "class Cache implements Store {\n    public const TTL = 60;\n}",
  },
  "removed.invalid_octal": {
    steps: {
      tr: [
        "0 ile başlayan sayılar sekizliktir. PHP 7.0'dan beri 8 ya da 9 içeren bir sekizlik sayı derleme hatasıdır; PHP 5.6'da sessizce kesiliyordu (`0781` 7 oluyordu).",
        "Onluk sayı kastediliyorsa baştaki 0'ı silin. PHP 5'teki değeri korumak için kesilen hâlini yazın (`07`).",
      ],
      en: [
        "Numbers starting with 0 are octal. Since PHP 7.0, an octal literal with an 8 or a 9 is a compile error; on PHP 5.6 it was silently cut (`0781` became 7).",
        "If a decimal number was meant, remove the leading 0. To keep the PHP 5 value, write the cut form (`07`).",
      ],
    },
    before: "$code = 0781;",
    after: "$code = 781;",
  },
  "removed.invalid_unicode_escape": {
    steps: {
      tr: [
        "PHP 7.0'dan beri çift tırnaklı metinlerde `\\u{` bir Unicode kaçışı başlatır; ardından geçerli bir onaltılık kod noktası gelmezse dosya derlenmez.",
        "Ters bölüyü kaçırın (`\\\\u{`) ya da metni tek tırnağa alın.",
      ],
      en: [
        "Since PHP 7.0, `\\u{` starts a Unicode escape in double-quoted strings, and the file doesn't compile unless a valid hex code point follows.",
        "Escape the backslash (`\\\\u{`) or put the string in single quotes.",
      ],
    },
    before: "$tpl = \"\\u{name}\";",
    after: "$tpl = '\\u{name}';",
  },
  "removed.is_real": {
    steps: {
      tr: [
        "`is_real()` takma adı PHP 8.0'da kaldırıldı; aynı fonksiyon `is_float()` adıyla duruyor. Rector doğrudan çağrıları değiştirir.",
        "Geri çağırma olarak verilen `'is_real'` metnini `'is_float'` yapın.",
      ],
      en: [
        "The `is_real()` alias was removed in PHP 8.0; the same function remains as `is_float()`. Rector changes direct calls.",
        "Change the `'is_real'` string passed as a callback to `'is_float'`.",
      ],
    },
    before: "$floats = array_filter($values, 'is_real');",
    after: "$floats = array_filter($values, 'is_float');",
  },
  "removed.list_result_by_reference": {
    steps: {
      tr: [
        "PHP 7.4'ten beri referanslı olmayan bir `list()` atamasının sonucunu referans bekleyen bir parametreye vermek hatadır.",
        "Atamayı çağrıdan önce yapın ve çağrıya değişkeni verin.",
      ],
      en: [
        "Since PHP 7.4, passing the result of a non-reference `list()` assignment to a by-reference parameter is an error.",
        "Do the assignment before the call and pass the variable.",
      ],
    },
    before: "$last = array_pop(list($a, $b) = $pair);",
    after: "list($a, $b) = $pair;\n$copy = $pair;\n$last = array_pop($copy);",
  },
  "removed.literal_value_error": {
    steps: {
      tr: [
        "Bu değer PHP 7'de uyarı verip false ya da null döndürüyordu; PHP 8.0'dan beri ValueError fırlatır. Bulgunun başlığı hangi çağrı olduğunu, önerilen satır yerine ne yazılacağını gösterir.",
        "Sık durumlar: `explode('', $s)` yerine `str_split($s)`, negatif sayılı `str_repeat()` ve `array_fill()` için sayıyı 0'da sınırlayın, `array_chunk()` ve `str_split()` için en az 1 verin, boş diziyle `array_rand()`'dan önce diziyi kontrol edin, `setcookie()`'ye bir ad verin.",
        "Değer bir değişkenden geliyorsa çağrıdan önce kontrol edin; eski false sonucuna güvenen kod artık ValueError'u yakalamalı.",
      ],
      en: [
        "This value made PHP 7 warn and return false or null; since PHP 8.0 it throws a ValueError. The finding's title names the call, and the suggested line shows what to write instead.",
        "Common cases: `str_split($s)` instead of `explode('', $s)`, keep the count of `str_repeat()` and `array_fill()` at 0 or above, give `array_chunk()` and `str_split()` at least 1, check the array before `array_rand()` on an empty one, give `setcookie()` a name.",
        "When the value comes from a variable, check it before the call; code that relied on the old false result now has to catch the ValueError.",
      ],
    },
    before: "$chars = explode('', $word);",
    after: "$chars = str_split($word);",
  },
  "removed.mb_ereg_e_option": {
    steps: {
      tr: [
        "Değiştirmeyi kod olarak çalıştıran `e` seçeneği PHP 8.0'da kaldırıldı.",
        "`mb_ereg_replace_callback()` kullanın; değiştirme metnindeki kodu, eşleşmeleri dizi olarak alan bir kapanışa yazın.",
      ],
      en: [
        "The `e` option, which ran the replacement as code, was removed in PHP 8.0.",
        "Use `mb_ereg_replace_callback()`; write the code from the replacement as a closure that gets the matches as an array.",
      ],
    },
    before: "mb_ereg_replace('(\\d+)', 'strval(\\\\1 * 2)', $s, 'e');",
    after: "mb_ereg_replace_callback('(\\d+)', fn($m) => strval($m[1] * 2), $s);",
  },
  "removed.mbstring_iso_alias": {
    steps: {
      tr: [
        "mbstring'in `ISO_8859-*` ve `ISO8859_*` takma adları PHP 8.0'da kaldırıldı.",
        "`ISO-8859-*` yazın, örneğin `'ISO-8859-1'`. Aynı adı `iconv()` de kabul eder.",
      ],
      en: [
        "mbstring's `ISO_8859-*` and `ISO8859_*` aliases were removed in PHP 8.0.",
        "Write `ISO-8859-*`, such as `'ISO-8859-1'`. `iconv()` accepts the same name.",
      ],
    },
    before: "mb_convert_encoding($s, 'UTF-8', 'ISO_8859-9');",
    after: "mb_convert_encoding($s, 'UTF-8', 'ISO-8859-9');",
  },
  "removed.mysqli_embedded_server": {
    steps: {
      tr: [
        "Gömülü MySQL sunucusu fonksiyonları PHP 7.4'te kaldırıldı. Varsayılan mysqlnd sürücüsüyle derlenen PHP'de hiç yoklardı; çağrı betiği zaten orada durduruyordu.",
        "Çağrıyı silin; Ferry sonucu kullanılmayan çağrıları kendisi siler. Sonucu bir koşulda kullanılıyorsa koşulu da kaldırın, çünkü fonksiyon yokken o kod hiç çalışmıyordu.",
      ],
      en: [
        "The embedded MySQL server functions were removed in PHP 7.4. PHP built with the default mysqlnd driver never had them, so the call already stopped the script there.",
        "Delete the call; Ferry deletes calls whose result isn't used itself. If the result is used in a condition, remove the condition too, since that code never ran without the function.",
      ],
    },
    before: "if (!mysqli_embedded_server_start(true, [], [])) {\n    die('no server');\n}\n$db = new mysqli($host);",
    after: "$db = new mysqli($host);",
  },
  "removed.mysqli_stat_property": {
    steps: {
      tr: [
        "Belgelenmemiş `mysqli::$stat` özelliği PHP 7.4'te kaldırıldı.",
        "`stat()` metodunu çağırın; aynı sunucu durum metnini döndürür.",
      ],
      en: [
        "PHP 7.4 removed the undocumented `mysqli::$stat` property.",
        "Call the `stat()` method; it returns the same server status string.",
      ],
    },
    before: "echo $db->stat;",
    after: "echo $db->stat();",
  },
  "removed.negative_shift": {
    steps: {
      tr: [
        "PHP 7.0'dan beri negatif bir sayı kadar kaydırmak ArithmeticError fırlatır; PHP 5.6'da sonuç işlemciye bağlıydı.",
        "Kaydırma miktarını pozitif tutun; ters yönde kaydırmak isteniyorsa diğer operatörü kullanın (`$x << -2` yerine `$x >> 2`).",
      ],
      en: [
        "Since PHP 7.0, shifting by a negative number throws an ArithmeticError; on PHP 5.6 the result depended on the CPU.",
        "Keep the shift amount positive; to shift the other way, use the other operator (`$x >> 2` instead of `$x << -2`).",
      ],
    },
    before: "$v = $x << -2;",
    after: "$v = $x >> 2;",
  },
  "removed.nul_byte_in_source": {
    steps: {
      tr: [
        "PHP 8.0'dan beri metinlerin dışındaki bir NUL baytı uyarı değil ParseError'dur.",
        "Baytı silin; çoğu düzenleyici göstermez. Onaltılık görünümde ya da `tr -d '\\000' < eski.php > yeni.php` ile temizleyebilirsiniz.",
      ],
      en: [
        "Since PHP 8.0, a NUL byte outside strings is a ParseError instead of a warning.",
        "Remove the byte; most editors don't show it. You can clean it in a hex view or with `tr -d '\\000' < old.php > new.php`.",
      ],
    },
  },
  "removed.null_constant_default": {
    steps: {
      tr: [
        "Bu sabit null. PHP 8.0'dan önce böyle bir varsayılan değer parametre tipini null kabul eder hâle getiriyordu; artık getirmez ve argüman verilmeyen çağrı TypeError fırlatır.",
        "Tipin önüne `?` ekleyin: `?int $a = N`. Varsayılanı doğrudan `null` yazmak da olur.",
      ],
      en: [
        "This constant is null. Before PHP 8.0 such a default made the parameter type nullable; now it doesn't, and a call that omits the argument throws a TypeError.",
        "Add `?` before the type: `?int $a = N`. Writing the default as plain `null` works too.",
      ],
    },
    before: "function page(int $size = DEFAULT_SIZE) {",
    after: "function page(?int $size = DEFAULT_SIZE) {",
  },
  "removed.odbc_row_zero": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `odbc_fetch_row()` 1'den küçük satır numarasında ValueError fırlatır.",
        "Satır numaraları 1'den başlar; ilk satır için 1 verin ya da sıradaki satır için argümanı hiç vermeyin.",
      ],
      en: [
        "Since PHP 8.4, `odbc_fetch_row()` throws a ValueError for a row number below 1.",
        "Row numbers start at 1; pass 1 for the first row, or leave the argument out for the next row.",
      ],
    },
    before: "odbc_fetch_row($result, 0);",
    after: "odbc_fetch_row($result, 1);",
  },
  "removed.php_uname_mode": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `php_uname()` geçersiz modda ValueError fırlatır; önceden `'a'` gibi davranıyordu.",
        "Geçerli bir mod verin: `'a'`, `'s'`, `'n'`, `'r'`, `'v'` ya da `'m'`.",
      ],
      en: [
        "Since PHP 8.4, `php_uname()` throws a ValueError for an invalid mode; it used to act like `'a'`.",
        "Pass a valid mode: `'a'`, `'s'`, `'n'`, `'r'`, `'v'` or `'m'`.",
      ],
    },
    before: "php_uname('x');",
    after: "php_uname('a');",
  },
  "removed.png2wbmp": {
    steps: {
      tr: [
        "`png2wbmp()` ve `jpeg2wbmp()` PHP 8.0'da kaldırıldı. Ferry çağrıyı aynı dönüşümü PHP ile yapan bir kapanışa çevirir; argümanlar `...` ile açılıyorsa dokunmaz.",
        "Elle yazarken görüntüyü `imagecreatefrompng()` ya da `imagecreatefromjpeg()` ile açın, `imagescale()` ile boyutlandırın ve `imagewbmp()` ile kaydedin. Bu yol yaklaşık bir sonuç verir, çünkü eski fonksiyonlar eşik değeriyle siyah-beyaza çeviriyordu; Ferry'nin yazdığı kapanış PHP 5.6 ile bayt bayt aynı çıktıyı verir.",
      ],
      en: [
        "`png2wbmp()` and `jpeg2wbmp()` were removed in PHP 8.0. Ferry turns the call into a closure that does the same conversion in PHP; it leaves calls whose arguments are unpacked with `...`.",
        "By hand, open the image with `imagecreatefrompng()` or `imagecreatefromjpeg()`, resize it with `imagescale()` and save it with `imagewbmp()`. That gives an approximate result, since the old functions turned the image black and white with the threshold; the closure Ferry writes gives byte-for-byte the same output as PHP 5.6.",
      ],
    },
    before: "png2wbmp($src, $dest, $h, $w, 7);",
    after: "$im = imagecreatefrompng($src);\nimagewbmp(imagescale($im, $w, $h), $dest);",
  },
  "removed.range_invalid": {
    steps: {
      tr: [
        "PHP 8.3'ten beri `range()` 0 adımda, boş metin sınırda ve dizi sınırda hata fırlatır.",
        "Adımı en az 1 yapın, sınırları sayı ya da tek harf olarak verin. Sınır bir değişkenden geliyorsa çağrıdan önce kontrol edin.",
      ],
      en: [
        "Since PHP 8.3, `range()` throws for a step of 0, an empty string boundary or an array boundary.",
        "Make the step at least 1, and pass the boundaries as numbers or single letters. When a boundary comes from a variable, check it before the call.",
      ],
    },
    before: "range(1, 10, 0);",
    after: "range(1, 10, 1);",
  },
  "removed.readonly_clone_reference": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `__clone()` içinde readonly bir özelliğe referans almak dolaylı değişiklik sayılır ve Error fırlatır.",
        "Özelliğe doğrudan atayın: `$this->items = clone $this->items;`. Referansla çalışan bir yardımcı fonksiyon kullanılıyorsa önce değeri bir değişkene alın, değiştirip özelliğe atayın.",
      ],
      en: [
        "Since PHP 8.4, taking a reference to a readonly property inside `__clone()` counts as indirect modification and throws an Error.",
        "Assign the property directly: `$this->items = clone $this->items;`. If a helper that works by reference is used, copy the value into a variable first, change it and assign it back.",
      ],
    },
    before: "public function __clone() {\n    $ref = &$this->items;\n    $ref = clone $ref;\n}",
    after: "public function __clone() {\n    $this->items = clone $this->items;\n}",
  },
  "removed.reflection_export": {
    steps: {
      tr: [
        "`Reflection*::export()` PHP 8.0'da kaldırıldı. Nesnenin metnini satır sonuyla yazıp null döndürüyordu; son argüman true ise metni döndürüyordu, bunu `(string) new ReflectionX(...)` yapar. Ferry bilinen biçimleri kendisi çevirir.",
        "Kalan çağrılarda (dönüş değeri hem yazılıp hem kullanılıyorsa gibi) aynı kalıbı elle yazın: yazdırmak için `echo new ReflectionClass($c), \"\\n\";`, metin için `(string) new ReflectionClass($c)`.",
      ],
      en: [
        "`Reflection*::export()` was removed in PHP 8.0. It printed the object's text with a newline and returned null; with true as the last argument it returned the text, which `(string) new ReflectionX(...)` does. Ferry converts the forms it knows itself.",
        "For the rest (when the result is both printed and used, say), write the same pattern by hand: `echo new ReflectionClass($c), \"\\n\";` to print, `(string) new ReflectionClass($c)` for the text.",
      ],
    },
    before: "$text = ReflectionClass::export($class, true);",
    after: "$text = (string) new ReflectionClass($class);",
  },
  "removed.regex_invalid_escape": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `\\i` gibi bilinmeyen kaçışlar düz karakter değil derleme hatasıdır (PHP 7.4'te `/\\i/` hâlâ `i`'yi buluyordu); `X` belirteci de yok sayılır, bu yüzden desen hiç eşleşmez.",
        "Karakteri ters bölüsüz yazın (`\\i` yerine `i`). Ters bölünün kendisi kastediliyorsa `\\\\\\\\` yazın. `X` belirtecini silin.",
      ],
      en: [
        "Since PHP 8.0, unknown escapes such as `\\i` are a compile error instead of literal characters (on PHP 7.4 `/\\i/` still found `i`), and the `X` modifier is ignored, so the pattern never matches.",
        "Write the character without the backslash (`i` instead of `\\i`). If a literal backslash was meant, write `\\\\\\\\`. Remove the `X` modifier.",
      ],
    },
    before: "preg_match('/^\\i\\d+$/', $s);",
    after: "preg_match('/^i\\d+$/', $s);",
  },
  "removed.regex_keep_in_lookaround": {
    steps: {
      tr: [
        "PHP 8.5'ten beri bir lookaround içinde `\\K` kullanmak derleme hatasıdır.",
        "`\\K`'yi lookaround'un dışına taşıyın ya da eşleşmenin istenen kısmını bir grupla yakalayıp onu kullanın.",
      ],
      en: [
        "Since PHP 8.5, `\\K` inside a lookaround is a compile error.",
        "Move `\\K` outside the lookaround, or capture the part of the match you want in a group and use that.",
      ],
    },
  },
  "removed.repeated_static": {
    steps: {
      tr: [
        "PHP 8.3'ten beri bir fonksiyonda aynı static değişkeni iki kez tanımlamak derleme hatasıdır.",
        "İkinci tanımı silin. PHP 8.2'ye kadar son başlatıcı geçerliydi; Ferry tanımı o değerle bir kez yazar. Başlatıcılardan biri sabit değilse tanımları elle birleştirin.",
      ],
      en: [
        "Since PHP 8.3, declaring the same static variable twice in a function is a compile error.",
        "Remove the second declaration. Up to PHP 8.2 the last initializer won; Ferry writes the declaration once with that value. If one of the initializers isn't constant, merge them by hand.",
      ],
    },
    before: "static $cache;\nstatic $hits = 0, $cache = [];",
    after: "static $hits = 0, $cache = [];",
  },
  "removed.round_invalid_mode": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `round()` geçersiz modda ValueError fırlatır; önceden yarıyı yukarı yuvarlıyordu.",
        "`PHP_ROUND_HALF_UP`, `PHP_ROUND_HALF_DOWN`, `PHP_ROUND_HALF_EVEN` ya da `PHP_ROUND_HALF_ODD` verin; eski sonuç için `PHP_ROUND_HALF_UP`.",
      ],
      en: [
        "Since PHP 8.4, `round()` throws a ValueError for an invalid mode; it used to round half up.",
        "Pass `PHP_ROUND_HALF_UP`, `PHP_ROUND_HALF_DOWN`, `PHP_ROUND_HALF_EVEN` or `PHP_ROUND_HALF_ODD`; `PHP_ROUND_HALF_UP` for the old result.",
      ],
    },
    before: "round($value, 2, 99);",
    after: "round($value, 2, PHP_ROUND_HALF_UP);",
  },
  "removed.setlocale_string_category": {
    steps: {
      tr: [
        "PHP 7.0'dan beri `setlocale()` kategoriyi yalnızca bir `LC_*` sabiti olarak kabul eder; metin vermek çalışmaz.",
        "Tırnakları kaldırıp sabiti yazın: `setlocale(LC_ALL, 'C')`.",
      ],
      en: [
        "Since PHP 7.0, `setlocale()` only accepts the category as an `LC_*` constant; a string doesn't work.",
        "Drop the quotes and write the constant: `setlocale(LC_ALL, 'C')`.",
      ],
    },
    before: "setlocale('LC_ALL', 'tr_TR.UTF-8');",
    after: "setlocale(LC_ALL, 'tr_TR.UTF-8');",
  },
  "removed.sql_regcase": {
    steps: {
      tr: [
        "`sql_regcase()` PHP 7.0'da kaldırıldı ve doğrudan bir karşılığı yok. Ferry çağrıyı aynı çıktıyı veren bir geri çağırmaya çevirir.",
        "Desen büyük-küçük harfe duyarsız aramak için kuruluyorsa `preg_match()`'e `i` belirteciyle düz deseni vermek daha basittir.",
      ],
      en: [
        "`sql_regcase()` was removed in PHP 7.0 and has no direct replacement. Ferry turns the call into a callback that gives the same output.",
        "If the pattern is built for a case-insensitive search, passing the plain pattern to `preg_match()` with the `i` modifier is simpler.",
      ],
    },
    before: "if (eregi(sql_regcase($word), $text)) {",
    after: "if (preg_match('/' . preg_quote($word, '/') . '/i', $text)) {",
  },
  "removed.ssl_context_option": {
    steps: {
      tr: [
        "`rsa_key_size`, `CN_match` ve `SNI_server_name` SSL bağlam seçenekleri PHP 7.0'da kaldırıldı ve sessizce yok sayılıyor.",
        "Sertifika adı için `peer_name` kullanın; SNI de bu adla gönderilir. `rsa_key_size`'ı silin.",
      ],
      en: [
        "The `rsa_key_size`, `CN_match` and `SNI_server_name` SSL context options were removed in PHP 7.0 and are silently ignored.",
        "Use `peer_name` for the certificate name; SNI is sent with it too. Remove `rsa_key_size`.",
      ],
    },
    before: "stream_context_create(['ssl' => ['CN_match' => 'api.example.com']]);",
    after: "stream_context_create(['ssl' => ['peer_name' => 'api.example.com']]);",
  },
  "removed.strip_tags_filter": {
    steps: {
      tr: [
        "`string.strip_tags` akış süzgeci PHP 7.3'te kullanımdan kalktı, PHP 8.0'da kaldırıldı.",
        "Okuduğunuz metne `strip_tags()` uygulayın. Etiketler okuma parçalarına bölünebileceği için mümkünse metnin tamamını okuyup bir kez uygulayın.",
      ],
      en: [
        "The `string.strip_tags` stream filter was deprecated in PHP 7.3 and removed in PHP 8.0.",
        "Apply `strip_tags()` to the text you read. Tags can be split across reads, so read the whole text and apply it once where you can.",
      ],
    },
    before: "stream_filter_append($fp, 'string.strip_tags');\n$text = stream_get_contents($fp);",
    after: "$text = strip_tags(stream_get_contents($fp));",
  },
  "removed.trait_property_default": {
    steps: {
      tr: [
        "Sınıf, kullandığı trait'in bir özelliğini farklı bir varsayılanla yeniden tanımlıyor. PHP 7.2'den beri varsayılanlar tür çevirmeden karşılaştırılır; `1` ile `'1'` bile uyumsuzdur ve sınıf derlenmez.",
        "Sınıftaki tanıma trait'teki varsayılanın aynısını verin, ya da sınıftaki tanımı silip değeri kurucuda atayın.",
      ],
      en: [
        "The class redeclares a property of a trait it uses with a different default. Since PHP 7.2 the defaults are compared without type juggling, so even `1` and `'1'` don't match and the class doesn't compile.",
        "Give the class's declaration the same default as the trait's, or remove the class's declaration and assign the value in the constructor.",
      ],
    },
    before: "trait Paging { public $size = 20; }\nclass Report { use Paging; public $size = '20'; }",
    after: "trait Paging { public $size = 20; }\nclass Report { use Paging; public $size = 20; }",
  },
  "removed.typed_constant_override": {
    steps: {
      tr: [
        "Bu yerleşik sınıfın sabitleri artık tipli. Alt sınıfta birini aynı tip olmadan yeniden tanımlamak derleme hatasıdır.",
        "Sabite yerleşik tipi verin (`const string ATOM = ...`). Sabit yalnızca bir değeri tutmak için yeniden tanımlandıysa başka bir adla tanımlamak daha güvenlidir.",
      ],
      en: [
        "This built-in class's constants are typed now. Redeclaring one in a subclass without the same type is a compile error.",
        "Give the constant the built-in type (`const string ATOM = ...`). If it was redeclared just to hold a value, declaring it under another name is safer.",
      ],
    },
    before: "class Stamp extends DateTime {\n    const ATOM = 'Y-m-d';\n}",
    after: "class Stamp extends DateTime {\n    const string ATOM = 'Y-m-d';\n}",
  },
  "removed.uncloneable_object": {
    steps: {
      tr: [
        "PHP 8.4'ten beri bu nesne kopyalanamaz; `clone` Error fırlatır. Bulgunun başlığı hangi sınıf olduğunu gösterir.",
        "Kopyalamak yerine yenisini oluşturun: `DOMXPath` için `new DOMXPath($doc)`. Düğüm listeleri için listeyi yeniden sorgulayın ya da `iterator_to_array()` ile bir diziye çevirin.",
      ],
      en: [
        "Since PHP 8.4 this object can't be cloned; `clone` throws an Error. The finding's title names the class.",
        "Create a new one instead of cloning: `new DOMXPath($doc)` for `DOMXPath`. For node lists, query again or turn the list into an array with `iterator_to_array()`.",
      ],
    },
    before: "$copy = clone $xpath;",
    after: "$copy = new DOMXPath($xpath->document);",
  },
  "removed.unset_this": {
    steps: {
      tr: [
        "`unset($this)` PHP 7.1'den beri ölümcül hatadır; dosya derlenmez. PHP 5'te yalnızca yerel değişkeni siliyor, nesneye dokunmuyordu.",
        "Çağrıyı silin; metot sonrasında `$this`'i okumuyorsa davranış aynı kalır ve Ferry bunu kendisi yapar. Okuyorsa o satırlar PHP 5'te tanımsız bir değişkene bakıyordu; onları da gözden geçirin.",
      ],
      en: [
        "`unset($this)` has been a fatal error since PHP 7.1; the file doesn't compile. On PHP 5 it only removed the local variable and left the object alone.",
        "Delete the call; when the method doesn't read `$this` afterwards the behaviour stays the same, and Ferry does this itself. If it does, those lines looked at an undefined variable on PHP 5; review them too.",
      ],
    },
    before: "unset($this);\nreturn $result;",
    after: "return $result;",
  },
  "removed.vsprintf_string": {
    steps: {
      tr: [
        "PHP 8.0'dan beri `vsprintf()` ve `vprintf()` dizi dışında bir şey verilince TypeError fırlatır; PHP 7.4'te false döndürüyorlardı.",
        "Değeri bir diziye koyun: `vsprintf('%s', [$value])`. Zaten tek değer varsa `sprintf()` kullanın.",
      ],
      en: [
        "Since PHP 8.0, `vsprintf()` and `vprintf()` throw a TypeError when given anything but an array; on PHP 7.4 they returned false.",
        "Put the value in an array: `vsprintf('%s', [$value])`. With a single value, use `sprintf()`.",
      ],
    },
    before: "$line = vsprintf('%s', $name);",
    after: "$line = vsprintf('%s', [$name]);",
  },
  "removed.xpath_non_node_set": {
    steps: {
      tr: [
        "PHP 8.5'ten beri `SimpleXMLElement::xpath()`, düğüm kümesi döndürmeyen ifadelerde (`count(//a)` gibi) hata fırlatır.",
        "Bu ifadeleri `DOMXPath::evaluate()` ile çalıştırın: `(new DOMXPath(dom_import_simplexml($xml)->ownerDocument))->evaluate('count(//a)')`.",
      ],
      en: [
        "Since PHP 8.5, `SimpleXMLElement::xpath()` throws for expressions that don't return a node set (such as `count(//a)`).",
        "Run such expressions with `DOMXPath::evaluate()`: `(new DOMXPath(dom_import_simplexml($xml)->ownerDocument))->evaluate('count(//a)')`.",
      ],
    },
    before: "$n = (int) $xml->xpath('count(//item)');",
    after: "$n = (int) (new DOMXPath(dom_import_simplexml($xml)->ownerDocument))->evaluate('count(//item)');",
  },
  "removed.ereg": {
    steps: {
      tr: [
        "POSIX regex fonksiyonları (`ereg()`, `eregi()`, `ereg_replace()`, `eregi_replace()`, `split()`, `spliti()`) PHP 7.0'da kaldırıldı. Rector doğrudan çağrıları sabit desenlerle `preg_*` karşılıklarına çevirir.",
        "Elle çevirirken deseni sınırlayıcıyla yazın (`'/.../'`), desendeki sınırlayıcıyı kaçırın; `eregi*` ve `spliti()` için `i` belirtecini ekleyin. `[[:alpha:]]` gibi POSIX sınıfları PCRE'de de çalışır.",
        "Desen bir değişkenden geliyorsa `'/' . str_replace('/', '\\/', $pattern) . '/'` ile sarın. Ad geri çağırma metni olarak verilmişse (`array_filter($a, 'ereg')` gibi) bir kapanış yazın.",
      ],
      en: [
        "The POSIX regex functions (`ereg()`, `eregi()`, `ereg_replace()`, `eregi_replace()`, `split()`, `spliti()`) were removed in PHP 7.0. Rector turns direct calls with literal patterns into their `preg_*` counterparts.",
        "By hand, write the pattern with delimiters (`'/.../'`) and escape the delimiter inside it; add the `i` modifier for `eregi*` and `spliti()`. POSIX classes such as `[[:alpha:]]` work in PCRE too.",
        "When the pattern comes from a variable, wrap it with `'/' . str_replace('/', '\\/', $pattern) . '/'`. When the name is passed as a callback string (such as `array_filter($a, 'ereg')`), write a closure.",
      ],
    },
    before: "if (eregi('^[a-z]+$', $name)) {",
    after: "if (preg_match('/^[a-z]+$/i', $name)) {",
  },
  "removed.call_user_method": {
    steps: {
      tr: [
        "Bu fonksiyon kaldırıldı ya da kullanımdan kalktı; önerilen satır yerine geçeni gösterir. Rector doğrudan çağrıları kendisi değiştirir.",
        "Ad geri çağırma metni olarak verildiyse (`array_map('eski_ad', $x)` gibi) ya da çağrı `eval()` içindeyse yerine geçeni elle yazın: argümanlar aynıysa yeni fonksiyonun adını, değişiyorsa bir kapanış.",
      ],
      en: [
        "This function was removed or deprecated; the suggested line shows what replaces it. Rector changes direct calls itself.",
        "When the name is passed as a callback string (such as `array_map('old_name', $x)`) or the call is inside `eval()`, write the replacement by hand: the new function's name when the arguments are the same, a closure when they change.",
      ],
    },
    before: "$data = array_map('read_exif_data', $files);",
    after: "$data = array_map('exif_read_data', $files);",
  },
  "deprecated.curl_close": {
    steps: {
      tr: [
        "Bu kapatma fonksiyonu PHP 8.0'dan beri hiçbir şey yapmıyor; kaynak artık bir nesne ve son referansı gidince kendiliğinden kapanıyor. Yeni sürümlerde kullanımdan kalktı.",
        "Çağrıyı silin; Rector ve Ferry tek başına duran çağrıları kendileri siler. Nesneyi hemen bırakmak istiyorsanız değişkene null atayın ya da `unset()` kullanın.",
      ],
      en: [
        "This close function has done nothing since PHP 8.0; the resource is an object now and closes on its own when the last reference goes. Newer versions deprecated it.",
        "Delete the call; Rector and Ferry delete standalone calls themselves. To let go of the object right away, set the variable to null or `unset()` it.",
      ],
    },
    before: "curl_close($ch);\nreturn $body;",
    after: "return $body;",
  },
  "deprecated.mysqli_no_data": {
    steps: {
      tr: [
        "Bu sabit PHP'nin hiç kullanmadığı bir değeri tutuyordu ve kullanımdan kalktı; önerilen satır değerini gösterir.",
        "Sabitin yerine değerini yazın; sonuç aynıdır. Ferry bunu kendisi yapar. Bayrak bir `|` maskesindeyse ve hiçbir şey yapmıyorsa maskeden çıkarın.",
      ],
      en: [
        "This constant held a value PHP never used and was deprecated; the suggested line shows the value.",
        "Write the value instead of the constant; the result is the same. Ferry does this itself. If the flag is in a `|` mask and does nothing, take it out of the mask.",
      ],
    },
    before: "if ($status === MYSQLI_NO_DATA) {",
    after: "if ($status === 100) {",
  },
  "deprecated.pdo_driver_constant": {
    steps: {
      tr: [
        "PHP 8.4 sürücüye özel PDO sabitlerini sürücü sınıflarına taşıdı ve `PDO::MYSQL_ATTR_*` gibi eski adları PHP 8.5'te kullanımdan kaldırdı.",
        "Önerilen satırdaki yeni adı yazın: `PDO::MYSQL_ATTR_SSL_CA` yerine `Pdo\\Mysql::ATTR_SSL_CA`. Değer aynıdır; Rector ve Ferry bilinen sabitleri kendileri değiştirir.",
      ],
      en: [
        "PHP 8.4 moved the driver-specific PDO constants to the driver classes, and PHP 8.5 deprecated old names such as `PDO::MYSQL_ATTR_*`.",
        "Write the new name from the suggested line: `Pdo\\Mysql::ATTR_SSL_CA` instead of `PDO::MYSQL_ATTR_SSL_CA`. The value is the same; Rector and Ferry change the constants they know themselves.",
      ],
    },
    before: "$options = [PDO::MYSQL_ATTR_SSL_CA => $ca];",
    after: "$options = [Pdo\\Mysql::ATTR_SSL_CA => $ca];",
  },
  "removed.get_magic_quotes_gpc": {
    steps: {
      tr: [
        "`get_magic_quotes_gpc()` ve `get_magic_quotes_runtime()` PHP 8.0'da kaldırıldı. PHP 5.4'ten beri her zaman false döndürüyorlardı.",
        "Çağrının yerine false yazın; `if (get_magic_quotes_gpc()) { ... }` gibi bir bloğu tümüyle silin, çünkü hiç çalışmıyordu. Ferry ikisini de kendisi yapar.",
      ],
      en: [
        "`get_magic_quotes_gpc()` and `get_magic_quotes_runtime()` were removed in PHP 8.0. They had always returned false since PHP 5.4.",
        "Write false instead of the call, and remove a block like `if (get_magic_quotes_gpc()) { ... }` altogether, since it never ran. Ferry does both itself.",
      ],
    },
    before: "if (get_magic_quotes_gpc()) {\n    $name = stripslashes($name);\n}\nsave($name);",
    after: "save($name);",
  },
  "removed.each": {
    steps: {
      tr: [
        "`each()` PHP 8.0'da kaldırıldı. Rector `while (list($k, $v) = each($a))` gibi yaygın döngüleri `foreach`'e çevirir.",
        "Kalan kullanımlarda dizinin iç işaretçisini `key()`, `current()` ve `next()` ile ilerletin: `$k = key($a); $v = current($a); next($a);`. Dizinin sonunda `key()` null döner; `each()` false döndürüyordu.",
      ],
      en: [
        "`each()` was removed in PHP 8.0. Rector turns common loops such as `while (list($k, $v) = each($a))` into `foreach`.",
        "For the rest, move the array's internal pointer with `key()`, `current()` and `next()`: `$k = key($a); $v = current($a); next($a);`. At the end of the array `key()` returns null, where `each()` returned false.",
      ],
    },
    before: "while (list($key, $value) = each($items)) {",
    after: "foreach ($items as $key => $value) {",
  },
  "removed.ezmlm_hash": {
    steps: {
      tr: [
        "`ezmlm_hash()` PHP 8.0'da kaldırıldı. Ferry çağrıyı aynı sonucu veren bir kapanışla değiştirir.",
        "Ad geri çağırma metni olarak verildiyse aynı kapanışı elle yazın: adresin her karakteri için `$h = (($h + ($h << 5)) ^ ord(strtolower($c))) & 0xFFFFFFFF;`, 5381'den başlayarak; sonuç `$h % 53`.",
      ],
      en: [
        "`ezmlm_hash()` was removed in PHP 8.0. Ferry replaces the call with a closure that gives the same result.",
        "When the name is passed as a callback string, write the same closure by hand: for each character of the address, `$h = (($h + ($h << 5)) ^ ord(strtolower($c))) & 0xFFFFFFFF;`, starting from 5381; the result is `$h % 53`.",
      ],
    },
  },
  "deprecated.mysqli_kill": {
    steps: {
      tr: [
        "`mysqli_kill()` ve `mysqli::kill()` PHP 8.4'te kullanımdan kalktı.",
        "Aynı işi bir sorguyla yapın: `mysqli_query($link, 'KILL CONNECTION ' . (int) $id)`. Ferry bunu kendisi yazar.",
      ],
      en: [
        "PHP 8.4 deprecated `mysqli_kill()` and `mysqli::kill()`.",
        "Do the same with a query: `mysqli_query($link, 'KILL CONNECTION ' . (int) $id)`. Ferry writes this itself.",
      ],
    },
    before: "mysqli_kill($link, $id);",
    after: "mysqli_query($link, 'KILL CONNECTION ' . (int) $id);",
  },
  "deprecated.mysqli_ping": {
    steps: {
      tr: [
        "`mysqli_ping()` ve `mysqli::ping()` PHP 8.4'te kullanımdan kalktı. PHP 8.2'den beri kopan bağlantıyı yeniden kurmuyorlardı.",
        "Ferry iki seçenek sunar: aynı gidiş-dönüşü `DO 1` sorgusuyla yapmak ya da kontrolü kaldırmak. Bağlantının kopabildiği uzun süreli işlerde sorgu hatasını yakalayıp bağlantıyı yeniden kurun.",
      ],
      en: [
        "PHP 8.4 deprecated `mysqli_ping()` and `mysqli::ping()`. Since PHP 8.2 they no longer reconnected a dropped connection.",
        "Ferry offers two options: make the same round trip with a `DO 1` query, or drop the check. In long-running jobs where the connection can drop, catch the query error and reconnect.",
      ],
    },
    before: "if (!$db->ping()) {\n    $db = connect();\n}",
    after: "try {\n    $db->query('DO 1');\n} catch (mysqli_sql_exception $e) {\n    $db = connect();\n}",
  },
  "removed.convert_cyr_string": {
    steps: {
      tr: [
        "`convert_cyr_string()` PHP 8.0'da kaldırıldı. Ferry iki seçenek sunar: `mb_convert_encoding()` ya da PHP 5.6'nın dönüşüm tablosuyla aynı sonucu veren `strtr()`.",
        "Tek harflik kodlar şunlara karşılık gelir: `k` KOI8-R, `w` Windows-1251, `i` ISO-8859-5, `a` ve `d` CP866, `m` x-mac-cyrillic. `mb_convert_encoding()` eşlenemeyen karakterlerde `?` yazar.",
      ],
      en: [
        "`convert_cyr_string()` was removed in PHP 8.0. Ferry offers two options: `mb_convert_encoding()`, or `strtr()` with PHP 5.6's conversion table, which gives the same result.",
        "The one-letter codes map to: `k` KOI8-R, `w` Windows-1251, `i` ISO-8859-5, `a` and `d` CP866, `m` x-mac-cyrillic. `mb_convert_encoding()` writes `?` for characters it can't map.",
      ],
    },
    before: "$text = convert_cyr_string($text, 'k', 'w');",
    after: "$text = mb_convert_encoding($text, 'Windows-1251', 'KOI8-R');",
  },
  "behavior.mysqli_default_exceptions": {
    steps: {
      tr: [
        "PHP 8.1'den beri mysqli'nin varsayılan hata modu `MYSQLI_REPORT_ERROR | MYSQLI_REPORT_STRICT`'tir: başarısız sorgu false döndürmek yerine `mysqli_sql_exception` fırlatır, bu kontrol hiç çalışmaz.",
        "Ferry iki seçenek sunar: çağrıdan önce `mysqli_report(MYSQLI_REPORT_OFF)` ile eski davranışa dönmek ya da çağrıyı `try`/`catch` içine almak. İlki istek boyunca bütün mysqli çağrılarını etkiler.",
      ],
      en: [
        "Since PHP 8.1, mysqli's default error mode is `MYSQLI_REPORT_ERROR | MYSQLI_REPORT_STRICT`: a failed query throws `mysqli_sql_exception` instead of returning false, so this check never runs.",
        "Ferry offers two options: go back to the old behaviour with `mysqli_report(MYSQLI_REPORT_OFF)` before the call, or wrap the call in `try`/`catch`. The first affects every mysqli call in the request.",
      ],
    },
    before: "if (!mysqli_query($db, $sql)) {\n    log_error(mysqli_error($db));\n}",
    after: "try {\n    mysqli_query($db, $sql);\n} catch (mysqli_sql_exception $e) {\n    log_error($e->getMessage());\n}",
  },
  "removed.builtin_function": {
    steps: {
      tr: [
        "Bu fonksiyon hedef sürümde yok; çağrılması `Call to undefined function` Error'u verir. Bulgunun başlığı fonksiyonu, önerilen satır varsa yerine geçeni gösterir.",
        "Fonksiyon kaldırılan bir eklentiye aitse (mcrypt, imap, xmlrpc gibi) o eklentinin rehberine ya da önerilen pakete bakın. Kendi başına kaldırılmışsa PHP kılavuzundaki fonksiyon sayfası yerine geçeni söyler.",
        "Kod fonksiyonu yalnızca varsa kullanıyorsa (`if (function_exists('ad'))`) o dal hedef sürümde zaten çalışmaz; dalı silmek de olur.",
      ],
      en: [
        "This function doesn't exist on the target version; calling it is a `Call to undefined function` Error. The finding's title names the function, and the suggested line shows the replacement when there is one.",
        "If the function belongs to a removed extension (such as mcrypt, imap or xmlrpc), see that extension's guide or the suggested package. If it was removed on its own, its page in the PHP manual names the replacement.",
        "If the code only uses the function when it exists (`if (function_exists('name'))`), that branch never runs on the target version anyway; removing it is fine too.",
      ],
    },
  },
  "removed.builtin_class": {
    steps: {
      tr: [
        "Bu sınıf hedef sürümde yok; kullanılması `Class not found` Error'u verir. Bulgunun başlığı sınıfı, önerilen satır varsa yerine geçeni gösterir.",
        "Sınıf kaldırılan bir eklentiye aitse o eklentinin rehberine ya da önerilen pakete bakın; diğer durumlarda PHP kılavuzundaki sınıf sayfası yerine geçeni söyler.",
      ],
      en: [
        "This class doesn't exist on the target version; using it is a `Class not found` Error. The finding's title names the class, and the suggested line shows the replacement when there is one.",
        "If the class belongs to a removed extension, see that extension's guide or the suggested package; otherwise its page in the PHP manual names the replacement.",
      ],
    },
  },
  "removed.builtin_class_constant": {
    steps: {
      tr: [
        "Bu sınıf sabiti hedef sürümde yok; kullanılması Error verir. Bulgunun başlığı sabiti, önerilen satır varsa yerine geçeni gösterir.",
        "Sabitin yerine geçen bir sabit varsa onu yazın; yoksa sabite bağlı kodu kaldırın ya da sabitin eski değerini açıkça yazın.",
      ],
      en: [
        "This class constant doesn't exist on the target version; using it is an Error. The finding's title names the constant, and the suggested line shows the replacement when there is one.",
        "Write the constant that replaces it if there is one; otherwise remove the code that depends on it, or write its old value out.",
      ],
    },
  },
  "deprecated.builtin_constant": {
    steps: {
      tr: [
        "Bu sabit hedef sürümde kullanımdan kalktı: hâlâ çalışır ama uyarı verir ve sonraki bir sürümde kaldırılabilir. Bulgunun önerilen satırı varsa yerine geçeni gösterir.",
        "Yerine geçen sabiti ya da sabitin değerini yazın. Sabit bir `|` maskesindeyse ve artık hiçbir şey yapmıyorsa maskeden çıkarın.",
      ],
      en: [
        "This constant is deprecated on the target version: it still works but warns, and a later version may remove it. The finding's suggested line shows the replacement when there is one.",
        "Write the replacing constant or the constant's value. If it's in a `|` mask and no longer does anything, take it out of the mask.",
      ],
    },
  },
  "behavior.inherited_static_reference": {
    steps: {
      tr: [
        "Bu statik özellik üst sınıftan geliyor. PHP 7.3'ten beri ona referans atamak özelliği alt sınıf için ayırmaz; üst sınıfın değeri de değişir. PHP 7.2'de yalnızca alt sınıfınki değişiyordu.",
        "Alt sınıfın kendi değeri olmalıysa özelliği alt sınıfta yeniden tanımlayın: `public static $x;`. Ortak değer zaten istenen buysa değişiklik gerekmez.",
      ],
      en: [
        "This static property comes from the parent class. Since PHP 7.3, assigning a reference to it doesn't separate it for the subclass, so the parent's value changes too. On PHP 7.2 only the subclass's changed.",
        "If the subclass should have its own value, redeclare the property in the subclass: `public static $x;`. If a shared value is what's wanted, nothing needs to change.",
      ],
    },
    before: "class Child extends Base {}\nChild::$config = &$local;",
    after: "class Child extends Base {\n    public static $config;\n}\nChild::$config = &$local;",
  },
  "behavior.trait_static_property": {
    steps: {
      tr: [
        "Bu trait üst sınıfta da tanımlı bir statik özellik içeriyor. PHP 8.3'ten beri trait'i kullanan sınıf özelliği kendisi için yeniden tanımlar ve üst sınıfla aynı değeri paylaşmaz.",
        "Değer ortak kalmalıysa özelliği trait'ten çıkarın ve üst sınıfınkini kullanın. Her sınıfın kendi değeri isteniyorsa değişiklik gerekmez.",
      ],
      en: [
        "This trait contains a static property that's also declared in the parent class. Since PHP 8.3 a class using the trait redeclares the property for itself and no longer shares the parent's value.",
        "If the value should stay shared, remove the property from the trait and use the parent's. If each class should have its own value, nothing needs to change.",
      ],
    },
  },
  "deprecated.chr_out_of_range": {
    steps: {
      tr: [
        "PHP 8.5'te `chr()`'a 0–255 dışında bir sayı vermek kullanımdan kalktı. Önceden sayının son baytı kullanılıyordu: `chr(256)` `\"\\0\"`, `chr(-1)` `\"\\xFF\"`, `chr(321)` `'A'` veriyordu.",
        "Eski sonuç için sayıyı 255 ile maskeleyin: `chr($n & 255)`. Bu bütün sürümlerde aynı baytı verir.",
      ],
      en: [
        "PHP 8.5 deprecated passing a number outside 0–255 to `chr()`. It used to take the number's last byte: `chr(256)` gave `\"\\0\"`, `chr(-1)` gave `\"\\xFF\"` and `chr(321)` gave `'A'`.",
        "For the old result, mask the number with 255: `chr($n & 255)`. That gives the same byte on every version.",
      ],
    },
    before: "$byte = chr($code);",
    after: "$byte = chr($code & 255);",
  },
  "deprecated.imap_nil_constant": {
    steps: {
      tr: [
        "imap eklentisinin `NIL` sabiti PHP 8.1'de kullanımdan kalktı; hiçbir fonksiyon onu beklemiyordu.",
        "Sabiti kullanan yeri kaldırın. imap eklentisinin kendisi PHP 8.4'te çekirdekten çıkarıldı; o kodu taşırken imap rehberine bakın.",
      ],
      en: [
        "PHP 8.1 deprecated the imap extension's `NIL` constant; no function expected it.",
        "Remove the code that uses it. The imap extension itself left the core in PHP 8.4; see the imap guide when you move that code.",
      ],
    },
  },
  "deprecated.pdo_driver_method": {
    steps: {
      tr: [
        "`PDO::sqlite*` ve `PDO::pgsql*` metotları PHP 8.5'te kullanımdan kalktı; PHP 8.4'te gelen `Pdo\\Sqlite` ve `Pdo\\Pgsql` sınıflarında aynı adlı metotlar var (`sqliteCreateFunction()` → `createFunction()`).",
        "Bağlantıyı sürücü sınıfıyla açın: `PDO::connect($dsn)` doğru alt sınıfı döndürür, ya da `new Pdo\\Sqlite($dsn)`. Sonra metodu yeni adıyla çağırın.",
      ],
      en: [
        "PHP 8.5 deprecated the `PDO::sqlite*` and `PDO::pgsql*` methods; the `Pdo\\Sqlite` and `Pdo\\Pgsql` classes added in PHP 8.4 have the same methods (`sqliteCreateFunction()` → `createFunction()`).",
        "Open the connection through the driver class: `PDO::connect($dsn)` returns the right subclass, or use `new Pdo\\Sqlite($dsn)`. Then call the method by its new name.",
      ],
    },
    before: "$pdo = new PDO('sqlite::memory:');\n$pdo->sqliteCreateFunction('md5', 'md5');",
    after: "$pdo = PDO::connect('sqlite::memory:');\n$pdo->createFunction('md5', 'md5');",
  },
  "deprecated.pdo_uri_dsn": {
    steps: {
      tr: [
        "PHP 8.5'te DSN'i bir dosyadan `uri:` ile okumak kullanımdan kalktı.",
        "Dosyayı kendiniz okuyup DSN'i verin: `new PDO(trim(file_get_contents($path)), $user, $pass)`.",
      ],
      en: [
        "PHP 8.5 deprecated reading the DSN from a file with `uri:`.",
        "Read the file yourself and pass the DSN: `new PDO(trim(file_get_contents($path)), $user, $pass)`.",
      ],
    },
    before: "$pdo = new PDO('uri:file:///etc/app/dsn');",
    after: "$pdo = new PDO(trim(file_get_contents('/etc/app/dsn')));",
  },
  "removed.pdo_oci": {
    steps: {
      tr: [
        "PDO_OCI sürücüsü PHP 8.4'te çekirdekten çıkarıldı; yalnızca PECL'den kurulursa çalışır.",
        "Sürücüyü sunucuya `pecl install pdo_oci` ile kurun (Oracle Instant Client gerekir) ve `extension=pdo_oci` satırını ekleyin. Kod değişmez.",
      ],
      en: [
        "PHP 8.4 unbundled the PDO_OCI driver; it only works when installed from PECL.",
        "Install it on the server with `pecl install pdo_oci` (Oracle Instant Client is needed) and add the `extension=pdo_oci` line. The code doesn't change.",
      ],
    },
  },
  "removed.script_tag": {
    steps: {
      tr: [
        "`<script language=\"php\">` ve ASP tarzı `<% %>` etiketleri PHP 7.0'da kaldırıldı; içindeki kod artık çalışmaz, metin olarak çıktıya gider.",
        "Etiketleri `<?php` ve `?>` ile değiştirin.",
      ],
      en: [
        "The `<script language=\"php\">` and ASP-style `<% %>` tags were removed in PHP 7.0; the code inside no longer runs and is printed as text.",
        "Replace the tags with `<?php` and `?>`.",
      ],
    },
    before: "<script language=\"php\">echo $title;</script>",
    after: "<?php echo $title; ?>",
  },
  "removed.splfileobject_extra_argument": {
    steps: {
      tr: [
        "PHP 8.2'den beri bu `SplFileObject` metotları argüman verildiğinde ArgumentCountError fırlatır; argümanlar zaten yok sayılıyordu.",
        "Argümanları silin.",
      ],
      en: [
        "Since PHP 8.2, these `SplFileObject` methods throw ArgumentCountError when given arguments; the arguments were ignored anyway.",
        "Remove the arguments.",
      ],
    },
    before: "if ($file->hasChildren(true)) {",
    after: "if ($file->hasChildren()) {",
  },
  "framework.laravel_helper": {
    steps: {
      tr: [
        "Laravel 6 `str_` ve `array_` yardımcılarını kaldırdı. PHP 8 en az Laravel 6.20 istediği için bu çağrı çalışmaz.",
        "`Illuminate\\Support\\Str` ve `Arr` sınıflarını kullanın (`str_slug($s)` → `Str::slug($s)`, `array_get($a, 'k')` → `Arr::get($a, 'k')`), ya da eski yardımcıları `composer require laravel/helpers` ile geri ekleyin.",
      ],
      en: [
        "Laravel 6 removed the `str_` and `array_` helpers. PHP 8 needs at least Laravel 6.20, so this call breaks.",
        "Use the `Illuminate\\Support\\Str` and `Arr` classes (`str_slug($s)` → `Str::slug($s)`, `array_get($a, 'k')` → `Arr::get($a, 'k')`), or bring the old helpers back with `composer require laravel/helpers`.",
      ],
    },
    before: "$slug = str_slug($title);",
    after: "$slug = \\Illuminate\\Support\\Str::slug($title);",
  },
  "framework.laravel_input": {
    steps: {
      tr: [
        "`Input` facade'ı Laravel 6'da kaldırıldı. PHP 8 en az Laravel 6.20 istediği için bu çağrı çalışmaz.",
        "`Request` facade'ını ya da `request()` yardımcısını kullanın: `Input::get('q')` → `Request::input('q')`.",
      ],
      en: [
        "Laravel 6 removed the `Input` facade. PHP 8 needs at least Laravel 6.20, so this call breaks.",
        "Use the `Request` facade or the `request()` helper: `Input::get('q')` → `Request::input('q')`.",
      ],
    },
    before: "$q = Input::get('q');",
    after: "$q = Request::input('q');",
  },
  "removed.ini_hash_comment": {
    steps: {
      tr: [
        "PHP 7.0'dan beri ini dosyalarında `#` yorum başlatmaz; satır ayar olarak okunur ve dosya hata verebilir.",
        "Satırın başındaki `#`'yi `;` yapın.",
      ],
      en: [
        "Since PHP 7.0, `#` doesn't start a comment in ini files; the line is read as a setting and the file can fail to parse.",
        "Change the `#` at the start of the line to `;`.",
      ],
    },
    before: "# memory_limit = 256M",
    after: "; memory_limit = 256M",
  },
  "removed.ini_setting": {
    steps: {
      tr: [
        "Bu ayar PHP'den kaldırıldı. Açık bırakıldığında PHP `Directive is no longer available` hatasıyla başlamaz; site hiç açılmaz.",
        "Satırı ini dosyasından ya da `.htaccess`'ten silin. Ayarın sağladığı şey (magic quotes, register_globals gibi) hâlâ gerekiyorsa onu kodda yapın.",
      ],
      en: [
        "PHP removed this setting. Left on, PHP refuses to start with `Directive is no longer available`, so the site doesn't load at all.",
        "Delete the line from the ini file or `.htaccess`. If what the setting did (magic quotes, register_globals and so on) is still needed, do it in code.",
      ],
    },
    before: "magic_quotes_gpc = On\nmemory_limit = 256M",
    after: "memory_limit = 256M",
  },
  "behavior.opcache_jit_default": {
    steps: {
      tr: [
        "PHP 8.4'te `opcache.jit`'in varsayılanı `disable` oldu. `opcache.jit` ayarlanmadan yalnızca `opcache.jit_buffer_size` verilirse JIT artık sessizce kapalı kalır.",
        "JIT açık kalsın isteniyorsa `opcache.jit = tracing` satırını ekleyin.",
      ],
      en: [
        "In PHP 8.4, `opcache.jit` defaults to `disable`. Setting only `opcache.jit_buffer_size`, without `opcache.jit`, now silently leaves the JIT off.",
        "If the JIT should stay on, add the line `opcache.jit = tracing`.",
      ],
    },
    before: "opcache.jit_buffer_size = 64M",
    after: "opcache.jit_buffer_size = 64M\nopcache.jit = tracing",
  },
  "behavior.opcache_zend_extension": {
    steps: {
      tr: [
        "PHP 8.5'te OPcache her zaman PHP'nin içinde gelir; onu `zend_extension` ile yüklemek başlangıçta uyarı verir.",
        "`zend_extension=opcache` satırını silin; `opcache.*` ayarları olduğu gibi kalabilir.",
      ],
      en: [
        "In PHP 8.5, OPcache is always built in, and loading it with `zend_extension` raises a warning at startup.",
        "Delete the `zend_extension=opcache` line; the `opcache.*` settings can stay as they are.",
      ],
    },
    before: "zend_extension=opcache\nopcache.enable=1",
    after: "opcache.enable=1",
  },
  "server.composer_platform": {
    steps: {
      tr: [
        "`composer.json`'daki `config.platform.php` Composer'a projenin hangi PHP'de çalışacağını söyler; eski sürümde kalırsa paketler hedef sürümü desteklemeyen sürümlerde kilitli kalır.",
        "Değeri hedef sürüm yapın ve `composer update` çalıştırın. Ferry değeri yazabilir; güncellemeyi siz çalıştırın.",
      ],
      en: [
        "`config.platform.php` in `composer.json` tells Composer which PHP the project runs on; left at the old version, packages stay locked at releases that don't support the target.",
        "Set it to the target version and run `composer update`. Ferry can write the value; run the update yourself.",
      ],
    },
    before: "\"config\": { \"platform\": { \"php\": \"7.4\" } }",
    after: "\"config\": { \"platform\": { \"php\": \"8.5\" } }",
  },
  "server.php_version_exact": {
    steps: {
      tr: [
        "Bu dosya PHP sürümünü yama sürümüyle birlikte sabitliyor (`8.1.27` gibi). Ferry hedefi `8.5.0` olarak yazar.",
        "`8.5.0`'ı sunucuda ya da imajda gerçekten kullandığınız yama sürümüyle değiştirin; güvenlik düzeltmeleri yama sürümlerinde gelir.",
      ],
      en: [
        "This file pins the PHP version down to the patch release (such as `8.1.27`). Ferry writes the target as `8.5.0`.",
        "Change `8.5.0` to the patch release you actually use on the server or in the image; security fixes come in patch releases.",
      ],
    },
  },
  "server.composer_platform_check": {
    steps: {
      tr: [
        "`config.platform-check: false` Composer'ın çalışırken PHP sürümünü ve eklentileri denetlemesini kapatır; proje yanlış sürümde açılırsa hata yerine garip davranışlar görürsünüz.",
        "Denetimi açın (`\"platform-check\": true` ya da satırı silin) ve `composer dump-autoload` çalıştırın.",
      ],
      en: [
        "`config.platform-check: false` stops Composer from checking the PHP version and extensions at runtime; if the project starts on the wrong version, you get odd behaviour instead of an error.",
        "Turn the check back on (`\"platform-check\": true`, or delete the line) and run `composer dump-autoload`.",
      ],
    },
  },
  "server.composer_platform_ext": {
    steps: {
      tr: [
        "`config.platform` PHP'yle birlikte gelen bir eklentiyi belirli bir sürüme sabitliyor; Composer bu sürümü gerçek eklentinin yerine kullanır ve hedef sürümde yanlış paketler seçilebilir.",
        "Satırı silin ve `composer update` çalıştırın; PHP'yle gelen eklentilerin sürümü PHP'ninkiyle aynıdır.",
      ],
      en: [
        "`config.platform` pins an extension that ships with PHP to a specific version; Composer uses that version instead of the real extension, so the wrong packages can be picked on the target.",
        "Delete the line and run `composer update`; extensions that ship with PHP have PHP's version.",
      ],
    },
  },
  "behavior.html_default_flags": {
    steps: {
      tr: [
        "PHP 8.1'den beri `htmlspecialchars()`, `htmlentities()` ve kod çözen karşılıklarının varsayılan bayrakları `ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401`'dir: tek tırnak da kodlanır, geçersiz UTF-8 boş sonuç yerine `�` olur.",
        "Ferry iki seçenek sunar: eski çıktıyı korumak için bu dosyadaki bayraksız çağrılara `ENT_COMPAT | ENT_HTML401` eklemek, ya da yeni bayrakları açıkça yazmak. Tek tırnaklı HTML özniteliklerinde eski bayraklar XSS açığı bırakır; mümkünse yenileri seçin.",
      ],
      en: [
        "Since PHP 8.1, the default flags of `htmlspecialchars()`, `htmlentities()` and their decoding counterparts are `ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401`: single quotes are encoded too, and invalid UTF-8 becomes `�` instead of an empty result.",
        "Ferry offers two options: add `ENT_COMPAT | ENT_HTML401` to the calls without flags in this file to keep the old output, or write the new flags out. The old flags leave an XSS gap in single-quoted HTML attributes; pick the new ones where you can.",
      ],
    },
    before: "echo htmlspecialchars($name);",
    after: "echo htmlspecialchars($name, ENT_QUOTES | ENT_SUBSTITUTE | ENT_HTML401);",
  },
  "deprecated.date_rfc7231": {
    steps: {
      tr: [
        "`DATE_RFC7231` ve `DateTimeInterface::RFC7231` biçimi saat dilimini yok sayıp her zaman `GMT` yazar; PHP 8.5'te kullanımdan kalktı.",
        "Ferry iki seçenek sunar: aynı çıktıyı veren `'D, d M Y H:i:s \\G\\M\\T'` biçimini yazmak, ya da tarihi UTC'ye çevirip bu biçimle yazmak. HTTP başlıkları gibi gerçekten GMT bekleyen yerlerde ikincisi doğrudur.",
      ],
      en: [
        "The `DATE_RFC7231` and `DateTimeInterface::RFC7231` format ignores the time zone and always writes `GMT`; PHP 8.5 deprecated it.",
        "Ferry offers two options: write the `'D, d M Y H:i:s \\G\\M\\T'` format, which gives the same output, or convert the date to UTC and format it that way. Where GMT is really expected, such as HTTP headers, the second is right.",
      ],
    },
    before: "header('Expires: ' . date(DATE_RFC7231, $time));",
    after: "header('Expires: ' . gmdate('D, d M Y H:i:s \G\M\T', $time));",
  },
  "deprecated.dynamic_property": {
    steps: {
      tr: [
        "PHP 8.2'den beri sınıfta tanımlı olmayan bir özelliğe atama yapmak kullanımdan kalktı; PHP 9'da hata olacak.",
        "Ferry iki seçenek sunar: sınıfa `#[\\AllowDynamicProperties]` eklemek (davranış aynı kalır) ya da kullanılan özellikleri sınıfta tanımlamak. Bir sınıfın bütün özellikleri aynı seçimi izler.",
        "Özellik adları çalışırken oluşuyorsa (`$this->$name = ...`) `#[\\AllowDynamicProperties]` ya da bir dizi özelliği (`$this->data[$name]`) kullanın.",
      ],
      en: [
        "Since PHP 8.2, assigning to a property that isn't declared on the class is deprecated; PHP 9 will make it an error.",
        "Ferry offers two options: add `#[\\AllowDynamicProperties]` to the class (the behaviour stays the same), or declare the properties used. All properties of a class follow the same choice.",
        "When property names are made at runtime (`$this->$name = ...`), use `#[\\AllowDynamicProperties]` or an array property (`$this->data[$name]`).",
      ],
    },
    before: "class Item {\n    public function __construct() {\n        $this->label = '';\n    }\n}",
    after: "class Item {\n    public $label;\n\n    public function __construct() {\n        $this->label = '';\n    }\n}",
  },
  "deprecated.serializable": {
    steps: {
      tr: [
        "PHP 8.1'den beri `Serializable` arayüzünü `__serialize()` ve `__unserialize()` olmadan uygulamak kullanımdan kalktı.",
        "İki metodu ekleyin. Ferry eski `serialize()` ve `unserialize()` metotlarını çağıran bir çift önerir; böylece mevcut veriyle aynı sonuç çıkar. PHP 7.4 ve sonrasında yeni metotlar önceliklidir.",
        "PHP 7.3 ve öncesini desteklemeniz gerekmiyorsa `Serializable`'ı ve eski metotları tümüyle kaldırabilirsiniz; ama eski biçimde saklanmış veri yine eski metotlarla okunur.",
      ],
      en: [
        "Since PHP 8.1, implementing `Serializable` without `__serialize()` and `__unserialize()` is deprecated.",
        "Add the two methods. Ferry proposes a pair that calls the old `serialize()` and `unserialize()` methods, so the result matches existing data. On PHP 7.4 and later the new methods take precedence.",
        "If you don't need PHP 7.3 and earlier, you can drop `Serializable` and the old methods; but data stored in the old format is still read through the old methods.",
      ],
    },
    before: "class Token implements Serializable {\n    public function serialize() { return $this->value; }\n    public function unserialize($data) { $this->value = $data; }\n}",
    after: "class Token implements Serializable {\n    public function serialize() { return $this->value; }\n    public function unserialize($data) { $this->value = $data; }\n    public function __serialize(): array { return ['value' => $this->value]; }\n    public function __unserialize(array $data): void { $this->value = $data['value']; }\n}",
  },
  "deprecated.soap_functions_all": {
    steps: {
      tr: [
        "PHP 8.4'ten beri `SoapServer::addFunction()`'a tamsayı (`SOAP_FUNCTIONS_ALL`) vermek kullanımdan kalktı.",
        "Sunulacak fonksiyonların adlarını bir dizi olarak verin. Ferry iki seçenek sunar: PHP'nin önerdiği gibi o an tanımlı bütün fonksiyonlar (sonradan tanımlananlar çağrılamaz), ya da yalnızca projenin fonksiyonları; ikincisi PHP'nin kendi fonksiyonlarını dışarı açmaz. Yalnızca gerçekten sunulacakları yazmak en güvenlisidir.",
      ],
      en: [
        "Since PHP 8.4, passing an integer (`SOAP_FUNCTIONS_ALL`) to `SoapServer::addFunction()` is deprecated.",
        "Pass the names of the functions to expose as an array. Ferry offers two options: every function defined at that point, as PHP suggests (ones defined later can't be called), or only the project's functions, which doesn't expose PHP's own. Listing only the ones you really expose is the safest.",
      ],
    },
    before: "$server->addFunction(SOAP_FUNCTIONS_ALL);",
    after: "$server->addFunction(['getOrder', 'listOrders']);",
  },
  "removed.input_request": {
    steps: {
      tr: [
        "`INPUT_REQUEST` ve `INPUT_SESSION` hiç uygulanmamıştı: `filter_input()` bunlarla her zaman null döndürüp uyarı veriyordu, `filter_has_var()` false döndürüyordu. PHP 8.0'da kaldırıldılar.",
        "Ferry iki seçenek sunar: çağrının eski sonucunu yazmak, ya da `$_REQUEST` veya `$_SESSION`'ı okumak. İkincisi kodun hiç görmediği gerçek veriyi getirir; o veriyle ne olacağını kontrol edin.",
      ],
      en: [
        "`INPUT_REQUEST` and `INPUT_SESSION` were never implemented: `filter_input()` always returned null with them and warned, and `filter_has_var()` returned false. PHP 8.0 removed them.",
        "Ferry offers two options: write the call's old result, or read `$_REQUEST` or `$_SESSION`. The second brings in real data the code has never seen; check what happens with it.",
      ],
    },
    before: "$id = filter_input(INPUT_REQUEST, 'id', FILTER_VALIDATE_INT);",
    after: "$id = filter_var($_REQUEST['id'] ?? null, FILTER_VALIDATE_INT);",
  },
  "removed.version_compare_operator": {
    steps: {
      tr: [
        "PHP 8.1'den beri `version_compare()` belgelenmemiş operatör kısaltmalarında (`'gte'` gibi) ValueError fırlatır; önceden karşılaştırmadan null döndürüyordu.",
        "Ferry iki seçenek sunar: kastedilen operatörü (`'>='`, `'<='`, `'=='`, `'!='`) yazmak ya da eski sonucu, yani null'ı yazmak. Geçerli operatörler `<`, `lt`, `<=`, `le`, `>`, `gt`, `>=`, `ge`, `==`, `eq`, `!=`, `<>` ve `ne`'dir.",
      ],
      en: [
        "Since PHP 8.1, `version_compare()` throws a ValueError for undocumented operator abbreviations (such as `'gte'`); it used to return null without comparing.",
        "Ferry offers two options: write the operator meant (`'>='`, `'<='`, `'=='`, `'!='`), or write the old result, null. The valid operators are `<`, `lt`, `<=`, `le`, `>`, `gt`, `>=`, `ge`, `==`, `eq`, `!=`, `<>` and `ne`.",
      ],
    },
    before: "if (version_compare(PHP_VERSION, '7.0', 'gte')) {",
    after: "if (version_compare(PHP_VERSION, '7.0', '>=')) {",
  },
};

/** Rules that share another rule's guide. */
const SHARED_GUIDES: [string, string][] = [
  ["deprecated.assert_constants", "deprecated.assert_options"],
  ["deprecated.mhash_constant", "deprecated.mhash"],
  ["deprecated.mysqli_refresh_constant", "deprecated.mysqli_refresh"],
  ["deprecated.openssl_x509_free", "deprecated.openssl_pkey_free"],
  ["removed.filter_flag_scheme_required", "removed.filter_flag_host_required"],
  ["removed.gzgetss", "removed.fgetss"],
  ["removed.jpeg2wbmp", "removed.png2wbmp"],
  ["removed.ldap_control_paged_result", "removed.ldap_control_paged_result_response"],
  ["removed.eregi", "removed.ereg"],
  ["removed.ereg_replace", "removed.ereg"],
  ["removed.eregi_replace", "removed.ereg"],
  ["removed.split", "removed.ereg"],
  ["removed.spliti", "removed.ereg"],
  ["removed.call_user_method_array", "removed.call_user_method"],
  ["removed.read_exif_data", "removed.call_user_method"],
  ["removed.gmp_random", "removed.call_user_method"],
  ["removed.image2wbmp", "removed.call_user_method"],
  ["removed.restore_include_path", "removed.call_user_method"],
  ["removed.hebrevc", "removed.call_user_method"],
  ["removed.mbereg", "removed.call_user_method"],
  ["removed.mbsplit", "removed.call_user_method"],
  ["removed.mbregex_encoding", "removed.call_user_method"],
  ["removed.set_socket_blocking", "removed.call_user_method"],
  ["removed.datefmt_set_timezone_id", "removed.call_user_method"],
  ["deprecated.socket_set_timeout", "removed.call_user_method"],
  ["deprecated.mysqli_execute", "removed.call_user_method"],
  ["deprecated.pg_alias", "removed.call_user_method"],
  ["deprecated.utf8_encode", "removed.call_user_method"],
  ["deprecated.utf8_decode", "removed.call_user_method"],
  ["deprecated.lcg_value", "removed.call_user_method"],
  ["deprecated.spl_object_storage", "removed.call_user_method"],
  ["deprecated.curl_share_close", "deprecated.curl_close"],
  ["deprecated.finfo_close", "deprecated.curl_close"],
  ["deprecated.imagedestroy", "deprecated.curl_close"],
  ["deprecated.xml_parser_free", "deprecated.curl_close"],
  ["deprecated.shmop_close", "deprecated.curl_close"],
  ["removed.oci_internal_debug", "deprecated.curl_close"],
  ["deprecated.mysqli_data_truncated", "deprecated.mysqli_no_data"],
  ["deprecated.mysqli_server_ps_out_params", "deprecated.mysqli_no_data"],
  ["deprecated.dom_php_err", "deprecated.mysqli_no_data"],
  ["deprecated.mysqli_update_max_length", "deprecated.mysqli_no_data"],
  ["deprecated.file_binary", "deprecated.mysqli_no_data"],
  ["deprecated.file_text", "deprecated.mysqli_no_data"],
  ["deprecated.pgsql_transaction_constant", "deprecated.mysqli_no_data"],
  ["removed.get_magic_quotes_runtime", "removed.get_magic_quotes_gpc"],
  ["deprecated.builtin_class_constant", "deprecated.builtin_constant"],
  ["deprecated.filter_sanitize_stripped", "deprecated.filter_sanitize_string"],
  ["removed.input_session", "removed.input_request"],
  ["removed.numberformatter_type_currency", "deprecated.numberformatter_type_currency"],
];
for (const [rule, same] of SHARED_GUIDES) GUIDES[rule] = GUIDES[same];
