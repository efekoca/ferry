import type { Lang } from "./index";

export interface RuleNote {
  deprecated?: string;
  why: Record<Lang, string>;
}

const MAGIC_QUOTES: Record<Lang, string> = {
  tr: "Magic quotes, gelen veriye otomatik ters eğik çizgi ekleyen eski bir özellikti. SQL enjeksiyonuna karşı gerçek koruma sağlamıyor, veriyi de bozuyordu. Özellik PHP 5.4'te kaldırıldı, ona bağlı işlevler de sonraki sürümlerde silindi.",
  en: "Magic quotes added backslashes to incoming data automatically. It gave no real protection against SQL injection and corrupted data. The feature was removed in PHP 5.4, and the functions around it followed in later versions.",
};

const EREG: Record<Lang, string> = {
  tr: "POSIX düzenli ifade işlevleri yavaştı, UTF-8 desteği zayıftı ve PCRE varken ayrıca bakımı gerekiyordu. Bu yüzden PHP 7.0'da kaldırıldılar. Aynı işi preg_* işlevleri yapar; desenin başına ve sonuna ayraç eklemek gerekir.",
  en: "The POSIX regular expression functions were slow, had poor UTF-8 support and duplicated PCRE. PHP 7.0 removed them. The preg_* functions do the same job; the pattern needs delimiters around it.",
};

const REAL: Record<Lang, string> = {
  tr: "real, float tipinin eski bir adıydı ve dilde iki ayrı ad tutmak karışıklık yaratıyordu. PHP 8.0 yalnızca float adını bırakır.",
  en: "real was an old name for float, and keeping two names for one type caused confusion. PHP 8.0 keeps only float.",
};

const USER_METHOD: Record<Lang, string> = {
  tr: "Bu işlevler nesne yöntemini adıyla çağırmak için vardı ve PHP 4'ten kalmaydı. call_user_func() ile dizi biçimindeki çağrı aynı işi yapar.",
  en: "These functions called an object method by name and dated from PHP 4. call_user_func() with an array callable does the same job.",
};

const STRFTIME: Record<Lang, string> = {
  tr: "strftime(), sonucu işletim sisteminin yerel ayarlarına bağladığı için aynı kod farklı sunucularda farklı çıktı veriyordu. Türkçe gibi dillerde tarih yazmak için IntlDateFormatter, sabit biçimler için date() kullanılır.",
  en: "strftime() depended on the operating system's locale, so the same code gave different output on different servers. IntlDateFormatter handles localized dates, and date() handles fixed formats.",
};

const UTF8: Record<Lang, string> = {
  tr: "Adı yanıltıcıydı: işlev yalnızca ISO-8859-1 ile UTF-8 arasında çevirir. Türkçe gibi başka kodlamalarda metni sessizce bozabilir. mb_convert_encoding() kaynak kodlamayı açıkça ister.",
  en: "The name was misleading: it only converts between ISO-8859-1 and UTF-8, and it silently damages text in any other encoding. mb_convert_encoding() asks for the source encoding explicitly.",
};

const SUN: Record<Lang, string> = {
  tr: "Eski güneş işlevleri ini ayarlarına ve belirsiz parametrelere bağlıydı. date_sun_info() enlem ve boylamı doğrudan alır, sonucu zaman damgası olarak verir.",
  en: "The old sun functions depended on ini settings and vague parameters. date_sun_info() takes latitude and longitude directly and returns timestamps.",
};

export const RULE_NOTES: Record<string, RuleNote> = {
  "removed.mysql": {
    deprecated: "5.5",
    why: {
      tr: "Eski mysql eklentisi hazırlanmış sorguları desteklemiyordu; bu yüzden SQL enjeksiyonuna açık kod yazmayı kolaylaştırıyordu. Yeni MySQL özelliklerini de izlemiyordu. PHP 7.0'da tümüyle kaldırıldı. Yerine mysqli ya da PDO kullanılır.",
      en: "The old mysql extension had no prepared statements, which made SQL injection easy to introduce, and it had stopped following new MySQL features. PHP 7.0 removed it entirely. Use mysqli or PDO instead.",
    },
  },
  "removed.mcrypt": {
    deprecated: "7.1",
    why: {
      tr: "mcrypt'in dayandığı libmcrypt kütüphanesi yıllardır bakımsızdı ve bilinen hataları düzeltilmiyordu. PHP 7.2'de çekirdekten çıkarıldı. Şifreleme için openssl ya da sodium kullanılır. Eski veriyi çözmek gerekiyorsa algoritma ve doldurma biçimine dikkat edilmelidir.",
      en: "mcrypt relied on libmcrypt, which had been unmaintained for years with known bugs left unfixed. PHP 7.2 removed it from the core. Use openssl or sodium instead. If old data must be decrypted, watch the algorithm and padding closely.",
    },
  },
  "removed.ereg": { deprecated: "5.3", why: EREG },
  "removed.eregi": { deprecated: "5.3", why: EREG },
  "removed.ereg_replace": { deprecated: "5.3", why: EREG },
  "removed.eregi_replace": { deprecated: "5.3", why: EREG },
  "removed.split": { deprecated: "5.3", why: EREG },
  "removed.spliti": { deprecated: "5.3", why: EREG },
  "removed.sql_regcase": { deprecated: "5.3", why: EREG },
  "removed.set_magic_quotes_runtime": { deprecated: "5.3", why: MAGIC_QUOTES },
  "removed.magic_quotes_runtime": { deprecated: "5.3", why: MAGIC_QUOTES },
  "removed.get_magic_quotes_gpc": { deprecated: "7.4", why: MAGIC_QUOTES },
  "removed.get_magic_quotes_runtime": { deprecated: "7.4", why: MAGIC_QUOTES },
  "removed.each": {
    deprecated: "7.2",
    why: {
      tr: "each() dizinin iç işaretçisini ilerletir; bu yüzden döngüden sonra dizi beklenmedik bir yerde kalabilir. foreach bu durumu yaşatmaz ve daha hızlıdır.",
      en: "each() moves the array's internal pointer, which can leave the array in an unexpected state after the loop. foreach avoids that and is faster.",
    },
  },
  "removed.create_function": {
    deprecated: "7.2",
    why: {
      tr: "create_function() kodu metin olarak alıp içeride eval ile çalıştırıyordu. Bu hem güvenlik riskiydi hem de bellekten hiç temizlenmiyordu. PHP 5.3'ten beri gelen anonim işlevler aynı işi güvenle yapar.",
      en: "create_function() took code as a string and ran it through eval. That was a security risk, and the result was never freed from memory. Anonymous functions, available since PHP 5.3, do the same job safely.",
    },
  },
  "removed.__autoload": {
    deprecated: "7.2",
    why: {
      tr: "__autoload() tek bir yükleyiciye izin veriyordu; Composer ve kütüphaneler aynı projede birlikte çalışamıyordu. spl_autoload_register() birden çok yükleyiciyi sıraya koyar. PHP 8.0 bu işlevi hiç çağırmaz, sınıflar sessizce bulunamaz.",
      en: "__autoload() allowed only one loader, so Composer and libraries couldn't work side by side. spl_autoload_register() queues several loaders. PHP 8.0 never calls this function, so classes simply aren't found.",
    },
  },
  "removed.php4_constructor": {
    deprecated: "7.0",
    why: {
      tr: "PHP 4'te sınıfla aynı adı taşıyan yöntem kurucuydu. PHP 8.0'da bu yöntem sıradan bir yöntem olur ve new sırasında çağrılmaz. Hata vermeden nesne yarım kurulur, en sinsi sorunlardan biridir.",
      en: "In PHP 4 a method named after its class was the constructor. In PHP 8.0 it becomes an ordinary method and new no longer calls it. The object ends up half built with no error, which makes this a quiet failure.",
    },
  },
  "removed.curly_offset": {
    deprecated: "7.4",
    why: {
      tr: "Dizi ve metin öğelerine süslü parantezle erişim, köşeli parantezin eski bir eşdeğeriydi. PHP 8.0'da sözdizimi hatasıdır; dosya hiç derlenmez, yani o dosyayı kullanan her sayfa durur.",
      en: "Reading array and string offsets with braces was an old equivalent of square brackets. In PHP 8.0 it is a syntax error, so the file doesn't compile and every page using it stops.",
    },
  },
  "removed.nested_ternary": {
    deprecated: "7.4",
    why: {
      tr: "PHP'de iç içe üçlü işleç soldan sağa çözülüyordu; bu, diğer dillerin tersiydi ve sessiz hatalara yol açıyordu. PHP 8.0 parantezsiz yazımı derleme hatası sayar. Parantez hangi anlamın kastedildiğini belirler.",
      en: "PHP resolved nested ternaries from left to right, the opposite of other languages, which caused silent bugs. PHP 8.0 treats the unparenthesized form as a compile error. Parentheses state the intended meaning.",
    },
  },
  "removed.bareword_key": {
    deprecated: "7.2",
    why: {
      tr: "Tırnaksız anahtar aslında tanımsız bir sabittir. PHP eskiden bunu metne çevirip devam ederdi. PHP 8.0'da Error fırlatır ve istek durur.",
      en: "An unquoted key is really an undefined constant. PHP used to turn it into a string and carry on. PHP 8.0 throws an Error and the request stops.",
    },
  },
  "removed.money_format": {
    deprecated: "7.4",
    why: {
      tr: "money_format() işletim sisteminin yerel ayarına bağlıydı ve Windows'ta hiç yoktu. NumberFormatter para birimini her ortamda aynı biçimde gösterir.",
      en: "money_format() depended on the operating system's locale and never existed on Windows. NumberFormatter shows currency the same way everywhere.",
    },
  },
  "removed.real_cast": { deprecated: "7.4", why: REAL },
  "removed.is_real": { deprecated: "7.4", why: REAL },
  "removed.call_user_method": { deprecated: "5.3", why: USER_METHOD },
  "removed.call_user_method_array": { deprecated: "5.3", why: USER_METHOD },
  "deprecated.implicit_nullable": {
    why: {
      tr: "Bir parametreye tip verip varsayılanını null yapmak, tipi gizlice nullable hâle getiriyordu. Bu kural dilin başka yerlerindeki tip davranışıyla çelişiyordu. Açık yazım bu belirsizliği kaldırır ve PHP 7.1'den beri çalışır.",
      en: "Giving a parameter a type and a null default quietly made the type nullable, which clashed with how types work elsewhere in the language. The explicit form removes the ambiguity and has worked since PHP 7.1.",
    },
  },
  "deprecated.strftime": { why: STRFTIME },
  "deprecated.gmstrftime": { why: STRFTIME },
  "deprecated.e_strict": {
    why: {
      tr: "PHP 8.0'dan beri hiçbir kod E_STRICT seviyesinde hata üretmiyor, yani sabit artık bir işe yaramıyor. E_ALL zaten tüm hataları kapsar.",
      en: "No code has raised an E_STRICT error since PHP 8.0, so the constant no longer does anything. E_ALL already covers every error.",
    },
  },
  "deprecated.tentative_return_type": {
    why: {
      tr: "PHP 8.1, yerleşik sınıfların yöntemlerine dönüş tipi ekledi. Bunları genişleten kod aynı tipi yazmazsa uyarı alır; ileride bu ölümcül hataya dönüşecek. Eski sürümlerle uyum gerekiyorsa #[\\ReturnTypeWillChange] uyarıyı geçici olarak susturur.",
      en: "PHP 8.1 added return types to the methods of built-in classes. Code that extends them without the same type gets a warning, and this will become a fatal error later. If you need to support old versions, #[\\ReturnTypeWillChange] silences it for now.",
    },
  },
  "deprecated.dynamic_property": {
    why: {
      tr: "Sınıfta tanımlı olmayan bir özelliğe yazmak genelde yazım hatasıydı ve fark edilmiyordu. Özelliği sınıfta tanımlamak en doğrusudur. Bilerek yapılıyorsa #[\\AllowDynamicProperties] kullanılır.",
      en: "Writing to a property the class doesn't declare was usually a typo that went unnoticed. Declaring the property is the best fix. If it is intentional, use #[\\AllowDynamicProperties].",
    },
  },
  "deprecated.utf8_encode": { why: UTF8 },
  "deprecated.utf8_decode": { why: UTF8 },
  "deprecated.date_sunrise": { why: SUN },
  "deprecated.date_sunset": { why: SUN },
};
