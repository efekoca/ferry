# Ferry — Proje İnceleme Raporu

**Tarih:** 1 Kasım 2026  
**İncelenen commit:** 38cd8341f22a3cdee4beda042d362e2a158e52f5  
**Amaç:** PHP sürüm yükseltme iş akışının doğruluğunu, dosya güvenliğini, mimarisini, arayüzünü ve edge case'lerini değerlendirmek.  
**Değişiklik sınırı:** Projenin mevcut dosyaları değiştirilmedi. Yalnızca bu rapor oluşturuldu. Deneyler ve ek doğrulama kodları geçici bir kopyada çalıştırıldı.

## 1. Genel değerlendirme

Projenin temeli iyi: Rust çekirdeğinin CLI ve masaüstü tarafından paylaşılması, Rector'ın dry-run çıktısının uygulama tarafından yazılması, önizleme kimliği, içerik kontrolü, yedekleme, iki dil ve erişilebilirlik testleri doğru tercihler. Arayüzün görsel dili de tutarlı.

Ancak mevcut hâliyle **“güvenli yükseltme” vaadinin kritik istisnaları var**. Özellikle başarısız yazmanın geri alınması, sembolik bağlantılar, hard link'ler ve geri yükleme onayı veri kaybına yol açabiliyor. Bazı tarama ve runtime hataları da başarılı/temiz sonuç olarak sunulabiliyor.

Önceliğim yeni özelliklerden önce aşağıdaki P1 maddelerini kapatmak olurdu. Mevcut testlerin geçmesi bu maddeleri kapsamıyor; rapordaki ek deneylerin önemli kısmı mevcut testlerde bulunmayan hata yollarını çalıştırıyor.

### Öncelik ve kanıt dili

- **P1:** Veri kaybı, onay dışı dosya değişimi veya yükseltme/CI kararını ciddi biçimde yanıltan hata. Yayın öncesi ele alınmalı.
- **P2:** İşlevsel hata, önemli yanlış pozitif/negatif, güvenilirlik veya erişilebilirlik problemi.
- **P3:** Daha düşük etkili tutarsızlık veya bakım sorunu.
- **Çalıştırılarak doğrulandı:** Geçici örnek, gerçek Rector, PHP, CLI, IPC veya arayüz testiyle gözlendi.
- **Kod akışıyla doğrulandı:** Hatalı koşul kaynakta mevcut; ilgili arıza ortamı ayrıca enjekte edilmedi.
- Tasarım önerileri ve henüz bulunmayan özellikler ayrı bölümlerde; bunlar mevcut hata gibi sayılmıyor.

### Önce ele alınması gerekenler

| ID | Öncelik | Sorun |
|---|---|---|
| F01 | P1 | Kısmi yazma hatasında bozulan dosya geri alınmıyor, yedeği silinebiliyor |
| F02 | P1 | Apply/restore proje dışına çıkan sembolik bağlantıları izleyebiliyor |
| F03 | P1 | Seçilmemiş hard link dosyaları da değişiyor |
| F04 | P1 | Geçerli sabit kullanımı değiştirilip “Otomatik” olarak sunuluyor |
| F05 | P1 | Geri yükleme onayı, onaylanan dosya içeriğine bağlı değil |
| F06 | P1 | Runtime fatal hata, ilgisiz deprecation içine yutulup CI başarılı çıkabiliyor |
| F07 | P1 | PHPUnit başarısızlığı ve timeout sonucu kullanıcıya aktarılmıyor |
| F08 | P1 | Taranamayan dosyalar CLI'da başarılı, 100/100 sonuç verebiliyor |

## 2. İnceleme ve doğrulama kapsamı

Kaynak incelemesi Rust çekirdeğini, Tauri komutlarını, React store ve ekranlarını, stilleri, çeviri yapısını, CLI çıktılarını, GitHub Action/CI tanımlarını, testleri ve proje dokümantasyonunu kapsadı. Büyük fonksiyon/signature tablolarının her girdisi ayrı ayrı elle doğrulanmadı; PHP reflection testleri ayrıca çalıştırıldı.

Deney ortamı macOS, Node 22.23.3 ve PHP CLI 8.5.12 idi. Gerçek Rector, uygulamada kurulu araçların geçici kopyası üzerinden çalıştırıldı. Orijinal projenin bağımlılıkları güncellenmedi.

| Kontrol | Sonuç |
|---|---|
| Mevcut React/Vitest testleri | 59 geçti |
| TypeScript typecheck | Geçti |
| Vite production build | Geçti |
| Rust core unit testleri | 167 geçti |
| Rust CLI testleri | 5 geçti |
| Gerçek Rector ile demo testleri | 6 geçti |
| Gerçek Rector ile integration testleri | 5 geçti |
| Mevcut Tauri IPC testleri | 17 geçti |
| PHP reflection ile deprecated fonksiyon/metot/sabit kontrolleri | 2 geçti |
| İnceleme için geçici kopyaya eklenen arayüz doğrulamaları | 4 senaryo doğrulandı |
| İnceleme için geçici kopyaya eklenen IPC doğrulamaları | 3 senaryo doğrulandı |
| Dosya sistemi, hata enjeksiyonu ve analiz örnekleri | Aşağıdaki bulguların ilgili yerlerinde sonuçlar verildi |

Rector'ın yerel işlem soketi başlangıçta çalışma ortamının sandbox sınırına takıldı; sokete izin verilen tekrarlar geçti. Bu ilk ortam hatası proje hatası olarak değerlendirilmedi.

Görsel inceleme, geçici kopyanın mock arayüzünde 1280×720 ve uygulamanın minimum boyutu olan 980×640 için yapıldı. Native macOS pencere efektleri, Windows/Linux paketleri, gerçek sunucu deployment'ı ve geniş dış proje corpus'u bu oturumda çalıştırılmadı. Reflection dışındaki üç corpus/benchmark testi ayrıca yürütülmedi.

**Önemli test nüansı:** FERRY_TEST_TOOLS verilmediğinde bazı Rector testleri erken return ile “passed” görünüyor. Bu nedenle Rector testlerinin araçlar gerçekten verilerek tekrarlanması özellikle yapıldı.

## 3. Kritik güvenlik ve doğruluk bulguları

### F01 — P1 — Başarısız yazılan dosya rollback listesine girmiyor; kurtarılabilir yedek de siliniyor

**Kanıt:** Çalıştırılarak doğrulandı; dosya boyutu sınırı ile kısmi yazma hatası enjekte edildi.

**Konum:** [rector.rs:607](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:607), [rector.rs:666](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:666), [Tauri lib.rs:730](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:730).

Hard link dalında dosya truncate edilip yerinde yazılıyor. write_all veya sync_all başarısız olursa dosya zaten değişmiş olabilir. Buna rağmen written listesine ekleme yalnızca write_atomic başarıyla döndükten sonra yapılıyor.

apply_changes hata durumunda yalnızca written dosyalarını geri yüklüyor. Liste boşsa restore_files başarılı dönüyor ve snapshot siliniyor.

**Gözlenen sonuç:**

    fault_result = FileTooLarge
    written = []
    dosyanın içeriği = "xxxx"       // başlangıçta "original"
    kalan yedek sayısı = 0

Bu deneyde gerçek write_changes ve snapshot API'leri, Tauri hata dalındaki aynı rollback/silme sırasıyla çalıştırıldı. Sürece özel RLIMIT_FSIZE kullanıldı; gerçek disk doldurulmadı.

**Düzeltme yönü:** “Tamamlanan” ile “dokunulmuş olabilecek” dosyaları ayır. Her yazma denemesinden önce rollback journal'a kaydet. Sonuç belirsizse snapshot'ı tut. Hard link'lere yerinde yazmayı kaldırmak/refüze etmek F03'ü de çözer. Persist sonrası permission hatası gibi dosyanın değişip fonksiyonun hata döndüğü yollar da aynı kapsamda olmalı.

**Regresyon:** İlk dosyada ve sonraki dosyada partial write, sync, rename ve permission hataları; hata dönen dosya dahil tüm başlangıç içerikleri geri gelmeli ve kurtarma belirsizse yedek kalmalı.

### F02 — P1 — Proje sınırı yazma/geri yükleme anında doğrulanmıyor

**Kanıt:** Hem apply hem restore için çalıştırılarak doğrulandı.

**Konum:** [rector.rs:609](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:609), [rector.rs:655](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:655), [snapshot.rs:70](/Users/efe/Desktop/eklenti/crates/ferry-core/src/snapshot.rs:70), [snapshot.rs:227](/Users/efe/Desktop/eklenti/crates/ferry-core/src/snapshot.rs:227).

Apply son dosyanın symlink olmasını reddediyor, fakat üst klasörlerin symlink olmasını kontrol etmiyor. Snapshot tarafındaki safe_relative ise yalnızca leksik olarak ../ ve absolute path kontrolü yapıyor; bağlantının gerçek hedefini güvenceye almıyor.

**İki yeniden üretim:**

1. Normal src/a.php için önizleme al. src klasörünü, aynı başlangıç içeriğini taşıyan proje dışı klasöre symlink yap. check_fresh geçiyor ve apply dışarıdaki a.php'yi değiştiriyor.
2. Normal a.php için snapshot al. a.php'yi proje dışındaki victim.php'ye symlink ile değiştir. restore, victim.php'nin içeriğinin üstüne yedeği yazıyor.

İki durum da gözlendi. Statik taramanın follow_links(false) kullanması sonradan değiştirilen bağlantıları engellemiyor.

**Düzeltme yönü:** Yedek alma, yazma ve restore için ortak güvenli dosya erişim katmanı oluştur. Proje root'u ve yol bileşenleri işlem anında doğrulansın; symlink/junction/reparse point politikası platforma göre açık olsun. Yalnızca önce canonicalize edip sonra normal open yapmak yarış aralığını tamamen kapatmaz; dosya/dizin handle'larına bağlı no-follow işlemleri değerlendirilmeli.

**Regresyon:** Dosya symlink'i, üst klasör symlink'i, Windows junction/reparse point ve önizlemeden sonra bağlantı değiştirme. Dışarıdaki hiçbir dosya değişmemeli.

### F03 — P1 — Tek dosya seçilse bile hard link'in diğer adları da değişiyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [rector.rs:617](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:617).

nlink > 1 olduğunda yerinde truncate/write tercih ediliyor. a.php ve b.php aynı inode'a hard link ise yalnızca a.php'yi seçmek b.php'yi de değiştiriyor. Bağlantının diğer adı proje dışında da olabilir. Snapshot ve written listesi yalnızca seçilen yolu içeriyor.

**Gözlenen:** Yalnızca a.php apply edildi; b.php de “changed” oldu.

**Düzeltme yönü:** Varsayılan olarak hard link dosyalarını reddet ve açık bir açıklama göster. Link'i kırarak yalnızca seçilen adı değiştirmek ayrı, bilinçli bir politika olabilir. Tüm alias'lara etkisi bilinmeden yerinde yazma yapılmamalı.

**Regresyon:** Proje içi ve proje dışı hard link; seçilmeyen yolun içeriği sabit kalmalı veya işlem yazmadan reddedilmeli.

### F04 — P1 — Başka dosyada tanımlı sabit, string'e çevrilerek davranış değiştiriyor; dosya “Otomatik” işaretleniyor

**Kanıt:** Gerçek Rector preview/apply ve PHP çalıştırmasıyla doğrulandı.

**Konum:** [analyzer.rs:207](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:207), [analyzer.rs:2757](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:2757), [rector.rs:505](/Users/efe/Desktop/eklenti/crates/ferry-core/src/rector.rs:505).

Örnek:

    // config.php
    <?php define('active', 'actual-key');

    // a.php
    <?php
    require 'config.php';
    $a = ['actual-key' => 1, 'active' => 2];
    echo $a[active];

defined_constants yalnızca mevcut dosyayı biliyor. Geçerli active sabiti bilinmeyince a.php, $a['active'] biçimine çevriliyor.

**Gerçek sonuç:** Başlangıç çıktısı 1, apply sonrası çıktı 2. Ayrıca preview'da rule.fix = Likely olmasına rağmen file.fix = Auto çıktı. Staging fallback'i FileChange.fix değerini koşulsuz Auto atıyor.

Burada iki düzeltme gerekli: sabit çözümleme yanlış ve dosya seviyesindeki risk sınıflaması kuralı bastırıyor.

**Düzeltme yönü:** Tanımsız olduğu kanıtlanamayan identifier'ı otomatik string'e çevirme. Proje sabitlerini/namespace'lerini çöz veya bu dönüşümü manuel incelemeye bırak. Her FileChange için fix, kuralların en yüksek riskinden türetilsin; staged-only dosyalar da aynı yolu kullansın.

**Regresyon:** include/require ile tanımlanan sabit, autoload.files, namespaced const, çalışma zamanında define ve staged-only Likely kuralı. Davranış korunmalı; Likely dosya “yalnızca otomatik” seçimine girmemeli.

### F05 — P1 — Restore onayı dosya adına bağlı; onaydan sonraki yeni düzenlemeler korunmuyor

**Kanıt:** Tauri IPC üzerinden çalıştırılarak doğrulandı.

**Konum:** [Tauri lib.rs:757](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:757), [Other.tsx:449](/Users/efe/Desktop/eklenti/src/screens/Other.tsx:449).

snapshot_conflicts yalnızca dosya adlarını döndürüyor. restore_snapshot, acknowledged içinde bulunan dosyalar için yeni içeriği karşılaştırmıyor.

**Senaryo:** a.php apply sonrası düzenleniyor; kullanıcı restore penceresinde a.php çakışmasını görüyor. Ardından editörde a.php tekrar değiştiriliyor. Restore çağrısı acknowledged = ["a.php"] ile hâlâ kabul ediliyor ve ikinci düzenleme de siliniyor.

Kodun “onaydan sonra değişti” kontrolü yalnızca yeni dosya adları için işe yarıyor; zaten çakışan dosyanın tekrar değişmesini yakalamıyor. Restore öncesi mevcut içeriğin ayrıca yedeği de alınmıyor.

**Düzeltme yönü:** Onayı {file, reviewedHash} veya sunucu tarafından üretilmiş içerik bağlı conflict token'ına bağla. Restore öncesi yeniden doğrula. Kullanıcının üzerine yazılmasına izin verdiği mevcut içerik için ayrıca kurtarma snapshot'ı oluştur.

**Regresyon:** Conflict incelemesinden sonra aynı dosyaya ikinci edit; yeni onay alınmadan restore reddedilmeli.

### F06 — P1 — Runtime fatal hata ilgisiz statik bulgunun içine yutulabiliyor

**Kanıt:** Gerçek CLI çağrısıyla doğrulandı; exit code 0.

**Konum:** [runtime.rs:131](/Users/efe/Desktop/eklenti/crates/ferry-core/src/runtime.rs:131), [CLI main.rs:462](/Users/efe/Desktop/eklenti/crates/ferry-cli/src/main.rs:462).

Eşleştirme aynı satırda severity veya mesaj kimliği aramıyor. Bir satır farkında ise yalnızca severity yeterli. İlk eşleşen statik bulguya runtime_hits ekleniyor; runtime olayının ayrı kaydı kayboluyor.

Örnek:

    // a.php — statik olarak sadece deprecated
    <?php utf8_encode($s);

    // İçeri aktarılan log
    PHP Fatal error: Uncaught RuntimeException: independent failure in /srv/app/a.php on line 1

**Gözlenen CLI sonucu:** confirmed = 1, added = 0, breaking = 0, deprecated = 1, score = 99; varsayılan fail-on breaking ile **exit 0**.

Üstelik runtime badge'i utf8_encode sorununun çalışma anında doğrulandığı izlenimini veriyor.

**Düzeltme yönü:** Runtime olaylarını kayıpsız ayrı bir modelde tut. Statik bulgu ile ilişki ancak mesaj/rule sembolü ve konum güçlü biçimde uyuşuyorsa “confirmed” olsun. Eşleştirilmiş olsa bile fatal olay CI değerlendirmesinde bağımsız olarak breaking etkisini korusun.

**Regresyon:** Aynı satırdaki ilgisiz fatal, aynı satırdaki çoklu sorun, komşu satırdaki ilgisiz warning ve birden fazla farklı runtime mesajı. Fatal hiçbir zaman deprecation'a indirgenmemeli.

### F07 — P1 — PHPUnit başarısızlığı, timeout ve çalıştırma çıktısı kayboluyor

**Kanıt:** Başarısız test süreciyle çalıştırılarak doğrulandı; UI'ya dönüş yolu ayrıca incelendi.

**Konum:** [localcheck.rs:37](/Users/efe/Desktop/eklenti/crates/ferry-core/src/localcheck.rs:37), [Tauri lib.rs:228](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:228), [store.tsx:509](/Users/efe/Desktop/eklenti/src/lib/store.tsx:509).

run_tests exit status'u saklamıyor. try_wait sonucu yalnızca bitiş bilgisi olarak kullanılıyor. Timeout'ta süreç öldürülüp yine Ok(output) döndürülüyor. Tauri ise ham çıktıyı atıp yalnızca runtime::parse sonucunu saklıyor.

Bir assertion failure, “No tests executed”, bootstrapping hatası veya tanınmayan PHPUnit çıktısı PHP warning/fatal formatında değilse kullanıcı testlerin başarısız olduğunu göremiyor.

**Gözlenen:** Exit 1 veren ve “There was 1 failure: assertion failed” yazan süreç için sonuç Ok(...). Bu çıktı runtime parser'da bulguya dönüşmüyor.

**Düzeltme yönü:** CheckResult modeli gerekli: kullanılan PHP, komut, exitCode, timedOut, cancelled, duration, stdout/stderr veya saklanan log ve parsed events. Test başarısı ayrı görünmeli. Timeout ve çalıştırılamama başarılı kontrol olarak dönmemeli.

**Regresyon:** Assertion failure, bootstrap exception, unsupported option, test bulunmaması, timeout, sinyal ile bitiş ve başarılı suite. Her sonuç kullanıcı tarafından ayırt edilebilmeli.

### F08 — P1 — Eksik tarama CLI'da temiz ve başarılı sonuç olarak sunulabiliyor

**Kanıt:** 2 MB üstü PHP dosyasıyla gerçek CLI üzerinden doğrulandı.

**Konum:** [analyzer.rs:75](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:75), [analyzer.rs:97](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:97), [CLI main.rs:221](/Users/efe/Desktop/eklenti/crates/ferry-cli/src/main.rs:221), [CLI main.rs:475](/Users/efe/Desktop/eklenti/crates/ferry-cli/src/main.rs:475), [report.rs:82](/Users/efe/Desktop/eklenti/crates/ferry-core/src/report.rs:82).

analyze_files okunamayan/büyük dosyaları errors içine koyuyor. CLI run bu alanı kullanmıyor; JSON'da da errors yok. filesScanned ise gerçekten analiz edilen dosyalar yerine files.len().

**Örnek:** İçinde each($a) bulunan 2 MB üstü tek a.php.

**Gözlenen JSON:** filesScanned = 1, findings = [], score = 100; **exit 0**. Dosyanın analiz edilmediği JSON'dan anlaşılmıyor.

Markdown export da scan.errors bilgisini taşımıyor. GUI'de uyarı var, fakat özet kartında “hazır” mesajı çıkabiliyor; uyarı sayfanın en altında.

**Düzeltme yönü:** complete/partial/failed analiz durumu, discovered/analyzed/skipped sayaçları ve yapılandırılmış errors tüm çıktılara taşınmalı. CLI'da partial scan varsayılan olarak operasyon hatası ya da açık bir fail-on-incomplete politikası üretmeli.

**Regresyon:** Büyük dosya, permission denied, kaybolan dosya/klasör, sıfır dosya ve kısmi index. “Tam tarama başarılı” ile “bulabildiğim yerde sorun yok” ayrılmalı.

## 4. Diğer işlevsel ve teknik bulgular

### F09 — P2 — GitHub Action cache'i eski tarayıcıyı çalıştırabiliyor

**Kanıt:** Kod akışıyla doğrulandı.

**Konum:** [action.yml:25](/Users/efe/Desktop/eklenti/action.yml:25).

Cache anahtarı Cargo.lock, runner.os ve github.action_ref içeriyor; Rust kaynakları veya çözülmüş action commit SHA'sı yok. @main gibi aynı ref üzerinde yalnızca analiz kodu/kural tablosu değişirse eski executable cache'den geliyor. command -v ferry-cli bulunduğunda build tamamen atlanıyor. PATH'te önceden bulunan başka ferry-cli da kabul ediliyor.

**Düzeltme yönü:** Binary'yi çözülmüş kaynak commit'i/ilgili kaynak hash'i ile eşleştir; action'a ait açık bir kurulum konumundan çalıştır. Cache miss/hit, kaynak doğruluğunun yerine geçmemeli.

**Regresyon:** Cargo.lock değişmeden yeni kural eklenmiş iki action commit'i ve PATH'te eski binary.

### F10 — P2 — Restore kısmi başarısızlıkta projeyi karışık durumda bırakıyor

**Kanıt:** Eksik yedek dosyasıyla çalıştırılarak doğrulandı.

**Konum:** [snapshot.rs:227](/Users/efe/Desktop/eklenti/crates/ferry-core/src/snapshot.rs:227).

Restore dosyaları sırayla kopyalıyor, hatalara rağmen diğerlerini deniyor ve sonunda yalnızca ilk hatayı döndürüyor. Ön doğrulama, atomik dosya değişimi ve işlemin geri alınması yok.

**Gözlenen:** İki dosyalı snapshot'ta ikinci yedek silindi. Restore hata döndü; a.php eski hâline geçti, b.php yeni hâlinde kaldı. UI hangi dosyaların geri geldiğini alamıyor. Hata döndüğü için başarılı restore sonrasındaki preview invalidation/rescan de yapılmıyor.

**Düzeltme yönü:** Önce tüm yedeklerin varlığını/bütünlüğünü ve hedefleri doğrula; sonra geçici dosyaları hazırla. Başarısızlıkta mevcut hâli kurtaracak journal kullan. Kısmi sonuç gerekiyorsa değişen dosyalar açıkça döndürülmeli.

**Regresyon:** Eksik/bozuk ikinci yedek, write permission hatası, restore sırasında kesinti ve disk hatası.

### F11 — P2 — Restore çakışma koruması metadata yoksa sessizce devre dışı kalıyor

**Kanıt:** Kod akışıyla doğrulandı.

**Konum:** [Tauri lib.rs:709](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:709), [Tauri lib.rs:785](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:785), [snapshot.rs:273](/Users/efe/Desktop/eklenti/crates/ferry-core/src/snapshot.rs:273).

changes.json ve written.json yazma hataları yok sayılıyor. read_attachment bütün okuma hatalarını “yok” olarak döndürüyor. written.json yoksa conflicts boş; JSON bozuksa unwrap_or_default ile yine boş.

Böylece apply başarılı görünüp restore'un koruması eksik kalabilir. Legacy metadata'sız snapshot ile bozulmuş/yazılamamış güncel snapshot da ayrılmıyor.

**Düzeltme yönü:** Snapshot manifest'inde şema sürümü, bütünlük ve uygulanmış içerik hash'leri birlikte zorunlu olmalı. Metadata tamamlanmadan işlem “tamamlandı” sayılmamalı. Eksik/bozuk metadata “çakışma yok” anlamına gelmemeli.

**Regresyon:** written.json yazma/okuma hatası, bozuk JSON ve eski snapshot şeması.

### F12 — P2 — PHPUnit stdout/stderr okuma sırası pipe deadlock yaratıyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [localcheck.rs:49](/Users/efe/Desktop/eklenti/crates/ferry-core/src/localcheck.rs:49).

Tek reader thread önce stdout'u EOF'a kadar okuyor, sonra stderr'e geçiyor. Çocuk süreç stdout'u açık tutup yeterince stderr üretirse stderr pipe'ı dolar, çocuk bekler; reader ise stdout kapanmasını bekler.

**Deney:** Süreç stderr'e 1 MB yazıp ardından stdout'a COMPLETED yazacaktı. 600 ms timeout ile 625 ms sonunda öldürüldü; COMPLETED yoktu, yalnızca 65536 byte alınabildi. Normalde bu iş anında tamamlanabilirdi. Üretim timeout'u 20 dakika.

**Düzeltme yönü:** İki pipe eşzamanlı tüketilmeli; mevcut run_cancellable yaklaşımından yararlanılabilir. Timeout nedeni F07'deki yapılandırılmış sonuca taşınmalı.

**Regresyon:** Büyük stderr, büyük stdout ve iki kanala dönüşümlü yoğun çıktı.

### F13 — P2 — Hedef sürümde çalışmayan koşullu fonksiyon tanımı, geçerli polyfill sanılıyor

**Kanıt:** Çalıştırılarak doğrulandı; analiz sonucu boş.

**Konum:** [index.rs:135](/Users/efe/Desktop/eklenti/crates/ferry-core/src/index.rs:135), [analyzer.rs:728](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:728).

Örnek:

    <?php
    if (PHP_VERSION_ID < 80000) {
        function each($x) {}
    }
    each($x);

PHP 8.4 hedefinde tanımın dalı çalışmaz ve çağrı undefined function olur. Index bütün tanımları hedef koşulundan bağımsız topladığı için çağrı güvenli sayılıyor.

**Düzeltme yönü:** Tanımların kapsamı ve hedefte var olma durumu index'e taşınmalı. “Projede metin olarak tanımlı” ile “bu hedefte mevcut olduğu kanıtlı” aynı şey olmamalı.

**Regresyon:** Eski sürüm dalındaki polyfill, ters function_exists kontrolü, koşullu include ve birden fazla alternatif tanım.

### F14 — P2 — Function import/alias ve çoklu namespace çözümlemesi bulguları kaçırıyor

**Kanıt:** Üç örnek de çalıştırılarak doğrulandı; hiç bulgu üretilmedi.

**Konum:** [analyzer.rs:178](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:178), [analyzer.rs:717](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:717).

    <?php namespace App; use function each; each($a);
    <?php namespace App; use function each as old_each; old_each($a);
    <?php namespace A { function each($x) {} } namespace B { each($x); }

İlk iki örnekte kaldırılmış global fonksiyon import ediliyor. Üçüncüde A namespace'indeki fonksiyon B'deki global fallback çağrısını korumuyor. local_functions ise dosya genelinde kısa ad seti; import'ları da yerel tanım gibi kabul ediyor.

**Düzeltme yönü:** Kapsama bağlı function symbol resolver; import alias'ı → gerçek FQN → global fallback sırası. Kısa ad setiyle erken return kaldırılmalı.

**Regresyon:** Yukarıdaki üçü, grouped use function, büyük/küçük harf ve namespace blokları arasında aynı kısa ad.

### F15 — P2 — Dinamik özellik kontrolü case sensitivity ve private görünürlüğünü kaybediyor

**Kanıt:** Ferry boş sonuç verdi; gerçek PHP iki örnekte de deprecation üretti.

**Konum:** [index.rs:556](/Users/efe/Desktop/eklenti/crates/ferry-core/src/index.rs:556), [index.rs:323](/Users/efe/Desktop/eklenti/crates/ferry-core/src/index.rs:323), [analyzer.rs:2725](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:2725).

    <?php
    class A {
        public $name;
        function f() { $this->Name = 1; }
    }

    class C { private $name; }
    class D extends C {
        function f() { $this->name = 1; }
    }

Property adları lowercase saklanıyor ve parent'taki bütün property'ler görünürlük olmadan birleştiriliyor. İlk örnekte Name başka bir özellik; ikinci örnekte child, parent'ın private property'sine yazamıyor.

**Düzeltme yönü:** Property kimliğinde orijinal case, declaring class, visibility ve static bilgisi tutulmalı. Metot/class adlarının case-insensitive politikası property'lere uygulanmamalı.

**Regresyon:** Case farkı, private parent property, protected/public miras, trait property ve aynı adın farklı sınıflardaki tanımları.

### F16 — P2 — Bir built-in sınıfın dosyada bulunması, başka nesnenin metoduna yanlış alarm üretiyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [analyzer.rs:162](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:162), [analyzer.rs:887](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:887).

    <?php
    $r = new ReflectionFunction('strlen');
    $mine->isDisabled();

Ferry ikinci satırdaki bağımsız $mine nesnesine ReflectionFunction::isDisabled deprecation'ı veriyor. Çoğu member rule'da receiver type yerine dosyada ilgili class adının geçmesi yeterli; bazı sınıf kontrolleri starts_with kullanıyor.

**Düzeltme yönü:** Receiver type kanıtı gerekli. Bilinmeyen nesne türünde kesin built-in metot bulgusu üretme; isteniyorsa ayrı düşük güvenli ipucu modeli kullan. Class eşleşmeleri FQN üzerinden tam olmalı.

**Regresyon:** Aynı isimli kullanıcı metodu, farklı namespace'te aynı kısa class adı, class adıyla başlayan bağımsız kullanıcı sınıfı ve alias'lar.

### F17 — P2 — Composer conflict.php göz ardı ediliyor

**Kanıt:** Hem root hem kilitli paket örneğinde çalıştırılarak doğrulandı.

**Konum:** [deps.rs:58](/Users/efe/Desktop/eklenti/crates/ferry-core/src/deps.rs:58), [packagist.rs:55](/Users/efe/Desktop/eklenti/crates/ferry-core/src/packagist.rs:55).

    {
      "require": { "php": "^8.0" },
      "conflict": { "php": ">=8.4" }
    }

Hedef 8.4 için rootStatus ve paket status değeri compatible çıktı. Kontrol yalnızca require.php okuyor. Packagist önerileri de aynı bilgiyle seçiliyor.

Composer conflict alanı kurulamayacak sürümleri ayrıca dışlar. [Composer şeması](https://getcomposer.org/doc/04-schema.md#conflict)

**Düzeltme yönü:** PHP uygunluğu require ve conflict'in birlikte değerlendirilmesiyle hesaplanmalı; root, locked package ve önerilen release için aynı politika kullanılmalı.

**Regresyon:** require izin veriyor/conflict engelliyor, conflict-only paket ve belirli patch'i dışlayan conflict.

### F18 — P2 — Constraint parser geçersiz girdiyi kabul ediyor; patch aralıklarını örnekleyerek yanlış karar verebiliyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [version.rs:128](/Users/efe/Desktop/eklenti/crates/ferry-core/src/version.rs:128), [version.rs:141](/Users/efe/Desktop/eklenti/crates/ferry-core/src/version.rs:141), [version.rs:219](/Users/efe/Desktop/eklenti/crates/ferry-core/src/version.rs:219).

Gözlenenler:

| Constraint | Ferry sonucu, hedef 8.4 | Problem |
|---|---|---|
| > | Some(true) | Operatörün operand'ı yok; boş AND grubu kabul ediliyor |
| 8.4bananas | Some(true) | Sayıdan sonraki geçersiz son ek doğrulanmıyor |
| >=8.4.41 <8.4.100 | Some(false) | 41–99 aralığı mevcut, fakat yalnızca 0–40 ve 999 deneniyor |

Ek olarak bare/exact sürümler ile wildcard sürümlerinin semantiği Composer'a karşı daha kapsamlı karşılaştırılmalı. Composer exact, wildcard, range ve stability koşullarını ayrı tanımlar. [Composer sürüm kısıtları](https://getcomposer.org/doc/articles/versions.md)

**Düzeltme yönü:** Parser tam girdiyi tüketmeli, eksik token'ı reddetmeli; sürüm hattı kesişimi sonlu patch örneklemesiyle hesaplanmamalı. Composer semver'a karşı fixture/differential testleri eklenmeli. Desteklenmeyen ifadeler compatible yerine unknown olmalı.

**Regresyon:** Yukarıdaki üçü, prerelease/stability suffix'leri, whitespace, birleşik operatörler, wildcard negation ve patch aralık sınırları.

### F19 — P2 — Unknown bağımlılıklar toplu mesajda compatible gibi sunuluyor

**Kanıt:** Markdown çıktısında çalıştırılarak; GUI'de kod akışıyla doğrulandı.

**Konum:** [report.rs:141](/Users/efe/Desktop/eklenti/crates/ferry-core/src/report.rs:141), [Other.tsx:152](/Users/efe/Desktop/eklenti/src/screens/Other.tsx:152).

bad listesi yalnızca incompatible paketler. Bu liste boşken bütün paketler unknown olsa bile “All 1 packages look compatible” / “Kilitli paketlerin hepsi ... uyumlu görünüyor” çıkıyor.

**Düzeltme yönü:** compatible/incompatible/unknown sayıları ayrı hesaplanmalı. Bilinmeyen paket varsa “engel saptanmadı, N paket doğrulanamadı” denmeli. Root unknown ve okunamayan/bozuk lock dosyası da görünür olmalı.

**Regresyon:** Tamamı unknown, compatible+unknown karışımı, unknown framework desteği ve bozuk composer.lock.

### F20 — P2 — Kodda doğrudan görülmeyen zorunlu extension'lar sunucu kontrolünden düşüyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [extensions.rs:31](/Users/efe/Desktop/eklenti/crates/ferry-core/src/extensions.rs:31), [probe.ts:40](/Users/efe/Desktop/eklenti/src/lib/probe.ts:40).

composer.json içinde require.ext-intl = "*" olan ve yalnızca echo içeren örnekte dependencies.extensions boş döndü. declared listesi çıktı üretmiyor; yalnızca kaynak taramasında bulunan extension'a etiket ekliyor.

Özellikle vendor içinden kullanılan, dinamik çağrılan veya Composer tarafından açıkça zorunlu tutulan extension eksikliği probe karşılaştırmasına girmiyor.

**Düzeltme yönü:** Envanter kaynak kullanımını, root gereksinimini ve kilitli paketlerin platform gereksinimlerini birleştirsin. Production/dev ayrımı korunsun. Extension adı alias'ları da normalize edilsin.

**Regresyon:** Yalnızca composer.json'da zorunlu extension, transitive package requirement ve kodda dinamik kullanılan extension.

### F21 — P2 — Runtime yol eşleştirmesi başka projedeki olayı yerel projeye bağlayabiliyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [runtime.rs:111](/Users/efe/Desktop/eklenti/crates/ferry-core/src/runtime.rs:111).

Yolun başındaki bileşenler sırayla atılıp yerelde mevcut ilk suffix seçiliyor. Logdaki /other-project/a.php, açık projede a.php varsa kabul ediliyor.

**Gözlenen:** outside = 0, added = 1, file = a.php. Birden fazla dosyanın aynı basename'e sahip olması da yanlış eşleşme yaratabilir. Normalizasyon ../ bileşenleri için canonical containment kontrolü de yapmıyor.

**Düzeltme yönü:** Açık remote-root → local-root eşlemesi; benzersiz suffix için asgari güven ölçütü; belirsiz/outside olayları kullanıcıya ayırma. Gerçek hedef proje sınırı içinde doğrulanmalı.

**Regresyon:** Ortak basename, iki farklı uzak proje, vendor mesajı, Windows yolu, ../ bileşeni ve silinmiş kaynak dosya.

### F22 — P2 — Yalnızca runtime'dan eklenen bulgu “Yok say” ile yok sayılamıyor

**Kanıt:** Tauri IPC üzerinden çalıştırılarak doğrulandı.

**Konum:** [Tauri lib.rs:167](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:167), [Tauri lib.rs:306](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:306).

Runtime bulguları respond sırasında klonlanmış scan'e ekleniyor. ignore_finding ve unignore_finding ise bulguyu state.scans içindeki ham statik scan'de arıyor.

**Gözlenen:** UI'nın sunduğu runtime.fatal ID'si ile ignore_finding çağrısı “Bulgu bulunamadı; projeyi yeniden tarayın” hatası verdi. Yeniden taramak yapısal problemi çözmüyor.

**Düzeltme yönü:** Etkileşilen bulgu kümesi ile ekranda gösterilen küme aynı sürümlü modelden gelmeli. Runtime olayları için uygun ve kararlı ignore kimliği tasarlanmalı.

**Regresyon:** Runtime-only fatal/warning/deprecation için ignore/unignore; statik+runtime ilişkili bulgu da çalışmaya devam etmeli.

### F23 — P2 — ferry-ignore yalnızca yorumlarda aranmadığından string içeriği bulguyu bastırıyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [ignore.rs:81](/Users/efe/Desktop/eklenti/crates/ferry-core/src/ignore.rs:81).

    <?php $text = 'ferry-ignore'; each($a);

Bu dosyada ignore yorumu yok. Buna rağmen kept = 0, ignored = 1 oldu. inline_reason satır metninde substring arıyor; yorum token'ı doğrulamıyor.

**Düzeltme yönü:** Direktifi lexer'ın gerçek yorum token'larında ve tanımlı sözdizimiyle parse et. String, heredoc ve HTML metni direktif olmamalı.

**Regresyon:** String içinde direktif, önceki satırdaki string, benzer identifier, block comment ve gerçek satır sonu yorumu.

### F24 — P2 — Inline ignore için “Show again / Göster” işlemi etkisiz

**Kanıt:** Tauri IPC üzerinden çalıştırılarak doğrulandı.

**Konum:** [Findings.tsx:390](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:390), [Tauri lib.rs:334](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:334), [ignore.rs:111](/Users/efe/Desktop/eklenti/crates/ferry-core/src/ignore.rs:111).

UI persisted ve inline ignore'lara aynı unignore eylemini sunuyor. Backend yalnızca ignore.json girdisini siliyor. Sonraki respond, yorum nedeniyle bulguyu tekrar ignore ediyor.

**Gözlenen:** unignore_finding başarılı döndü; bulgu hâlâ ignored listesinde.

**Düzeltme yönü:** Ignore kaynağını modele ekle. Inline ignore için “Kaynak yorumunu düzenle” yönlendirmesi veya açıkça önizlenen yorum değişimi sun. Etkisiz buton gösterme.

**Regresyon:** JSON ignore, inline ignore ve ikisinin aynı bulguya uygulanması.

### F25 — P2 — Uzun satırdaki aktif ignore aynı anda stale kabul ediliyor

**Kanıt:** Çalıştırılarak doğrulandı.

**Konum:** [analyzer.rs:515](/Users/efe/Desktop/eklenti/crates/ferry-core/src/analyzer.rs:515), [ignore.rs:120](/Users/efe/Desktop/eklenti/crates/ferry-core/src/ignore.rs:120).

Finding snippet'i 200 karakterde kesilip sonuna … ekleniyor. Ignore bu kısaltılmış gösterim metninden üretiliyor. stale kontrolü ise tam kaynak satırının snippet'i içerip içermediğine bakıyor; kaynakta … bulunmadığı için eşleşmiyor.

**Gözlenen:** Tek, hâlâ geçerli ignore için ignored = 1 ve stale = 1. “Eski ignore'ları temizle” bu aktif kararı silebiliyor.

**Düzeltme yönü:** Eşleştirme fingerprint'i, kullanıcıya gösterilen kısaltılmış snippet'ten ayrılmalı. Tam normalize edilmiş kod veya token tabanlı kimlik kullanılmalı.

**Regresyon:** 200 karakter üstü satır, minified dosya, multibyte metin ve satır numarası değişimi.

### F26 — P2 — Ignore uygulaması aynı dosyanın tüm satırlarını her bulgu için tekrar klonluyor

**Kanıt:** Kod ve ölçek deneyiyle doğrulandı.

**Konum:** [ignore.rs:95](/Users/efe/Desktop/eklenti/crates/ferry-core/src/ignore.rs:95).

sources cache var, ancak lines_of her çağrıda Option<Vec<String>> değerini clone ediyor. M bulgulu, N satırlı dosyada yaklaşık O(M×N) kopyalama oluyor; yoğun legacy/minified kaynakta tarama sonundaki görünmez gecikme büyüyor.

**Tek koşuluk debug-build ölçümü; üretim benchmark'ı değildir:**

| Aynı dosyadaki satır/bulgu sayısı | Yalnızca ignore::apply |
|---:|---:|
| 1000 | 29 ms |
| 2000 | 116 ms |
| 4000 | 460 ms |

İki kat girdi yaklaşık dört kat süre üretti.

**Düzeltme yönü:** Kaynak cache'ini bir kez doldur; bulgu başına yalnızca gereken satırlara referansla eriş. Stale araması da tek geçiş veya normalize fingerprint setiyle yapılmalı.

**Regresyon:** Az dosyada çok bulgu; hem ignore listesi boşken hem doluyken performans eğrisi izlenmeli.

### F27 — P2 — Temiz bulgu ekranı, runtime ile eksik sorunları bulma yolunu kapatıyor

**Kanıt:** Ek React testiyle doğrulandı.

**Konum:** [Findings.tsx:185](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:185), [Findings.tsx:236](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:236).

findings.length === 0 olduğunda bütün toolbar kaldırılıyor. Log içe aktar, PHP ile lint et ve testleri çalıştır düğmeleri de bu toolbar içinde.

Statik taramanın hiçbir şey bulamadığı proje, runtime kanıtının en yararlı olduğu senaryolardan biri. Kullanıcı tam bu durumda kontrolleri başlatamıyor. Son bulgu ignore edildiğinde veya temizlenmek istenen boş runtime kaydı olduğunda da aynı problem yaşanıyor.

**Düzeltme yönü:** Kontrol eylemleri ve runtime/coverage durumları boş liste görünümünden bağımsız kalmalı.

**Regresyon:** İlk tarama temiz, tamamı ignored, runtime kaydı sıfır olay içeriyor ve son aktif bulgunun kaldırılması.

### F28 — P2 — Probe JSON'unun zayıf doğrulanması render hatasına izin veriyor

**Kanıt:** Ek test parse/compare sonucunun yanlış tipleri kabul ettiğini doğruladı; React'e aktarımı kaynakta doğrudan mevcut.

**Konum:** [probe.ts:25](/Users/efe/Desktop/eklenti/src/lib/probe.ts:25), [Other.tsx:395](/Users/efe/Desktop/eklenti/src/screens/Other.tsx:395), [Other.tsx:423](/Users/efe/Desktop/eklenti/src/screens/Other.tsx:423), [App.tsx:68](/Users/efe/Desktop/eklenti/src/App.tsx:68).

parseProbe yalnızca ferryProbe, php ve extensions alanlarının bazı tiplerini kontrol ediyor. server veya ini.memory_limit nesne olabilir. Bu değerler React child olarak render ediliyor; geçersiz child uygulamanın ekranını düşürebilir. Uygulamada ErrorBoundary da yok.

Örnek:

    {
      "ferryProbe": 1,
      "php": "8.4.27",
      "extensions": [],
      "server": { "unexpected": true },
      "ini": { "memory_limit": { "unexpected": true } }
    }

**Düzeltme yönü:** Sınırda tam runtime schema validation ve açık normalizasyon. php sürümü de doğrulansın. Hatalı içerik kontrollü “geçersiz probe” görünümüne dönmeli. ErrorBoundary son savunma katmanı olarak eklenebilir.

**Regresyon:** Nesne/array/null server, geçersiz ini, malformed php, büyük payload ve bozuk/eksik alanlar.

### F29 — P2 — Ayarlardan seçilen PHP, hedef sürüm lint/test keşfinde kullanılmıyor

**Kanıt:** Kod akışıyla doğrulandı.

**Konum:** [toolchain.rs:187](/Users/efe/Desktop/eklenti/crates/ferry-core/src/toolchain.rs:187), [Tauri lib.rs:222](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:222), [Tauri lib.rs:234](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:234).

find_php override'ı Rector/toolchain kontrolünde kullanıyor. local_php_versions ve run_local_check ise yalnızca hardcoded candidate yolları/PATH üzerinden keşif yapıyor; AppState.php_override verilmemiş.

Kullanıcı Ayarlar'da standart olmayan /custom/php84/bin/php seçse bile hedef sürüm lint/test düğmeleri bulunmayabilir veya başka binary çalışabilir.

**Düzeltme yönü:** PHP keşfi tek hizmetten yapılmalı; seçili binary doğrulanıp candidate listesinin önüne eklenmeli. Ekranda gerçek binary yolu ve sürümü gösterilmeli.

**Regresyon:** PATH'te olmayan override, aynı minor'da iki PHP binary'si ve geçersiz/kaybolmuş override yolu.

### F30 — P2 — Kısmi apply sonrası kullanıcının dışladığı dosyalar yeniden seçiliyor

**Kanıt:** Kod akışıyla doğrulandı.

**Konum:** [Changes.tsx:17](/Users/efe/Desktop/eklenti/src/screens/Changes.tsx:17), [Changes.tsx:31](/Users/efe/Desktop/eklenti/src/screens/Changes.tsx:31), [store.tsx:382](/Users/efe/Desktop/eklenti/src/lib/store.tsx:382).

Dışlama listesi preview nesnesinin referansına bağlı. Apply sonrasında store kalan değişiklikler için yeni preview nesnesi oluşturuyor. excluded.preview === preview false olduğu için dışlamalar yok sayılıyor.

**Senaryo:** A ve B dosyalarından B'yi dışla, yalnızca A'yı uygula. Kalan B, kullanıcı fikrini değiştirmediği hâlde tekrar seçilmiş oluyor.

**Düzeltme yönü:** Seçim kararı preview.id ve dosya kimliğine bağlı tutulmalı. Apply edilenler listeden çıkarılırken kalan dosyaların önceki seçimi korunmalı.

**Regresyon:** Dışlanmış dosyalarla kısmi apply, apply sonrası rescan ve tekrar preview hesaplama.

### F31 — P2 — Yeni syntax hatası, dosya önceden de hatalıysa gizleniyor

**Kanıt:** Gerçek PHP ile çalıştırılarak doğrulandı.

**Konum:** [toolchain.rs:274](/Users/efe/Desktop/eklenti/crates/ferry-core/src/toolchain.rs:274), [Tauri lib.rs:720](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:720).

new_syntax_errors, after lint hatalıysa before'ı kontrol ediyor; before herhangi bir hata veriyorsa after'daki hata tamamen atılıyor. Hataların aynı olup olmadığına bakılmıyor.

**Örnek:** Before PHP 8'de kaldırılmış (real) cast içeriyor; after tamamen başka bir parse hatası içeriyor. Fonksiyon boş lintErrors döndürdü.

Ayrıca lint yazmadan sonra yapılıyor, host PHP >= target şartıyla seçiliyor ve lint çalıştırılamaması da lint_source içinde None ile “hata yok” sonucuna karışıyor.

**Düzeltme yönü:** Lint sonucu success/error/not-run olarak ayrılmalı. Hedef minor doğrulaması mümkünse yazmadan önce yapılmalı. Eski ve yeni hatalar kimlik/konum/mesaj bakımından karşılaştırılmalı; “önceden bir hata vardı” yeni hatayı bastırmamalı.

**Regresyon:** Önce eski syntax hatası/sonra başka hata, iki aynı hata, lint binary'sinin çalıştırılamaması ve host'un target'tan yeni olması.

### F32 — P2 — main'e yalnızca kaynak değişikliği push edilmesi CI'ı tetiklemiyor

**Kanıt:** Workflow tanımıyla doğrulandı.

**Konum:** [.github/workflows/ci.yml:4](/Users/efe/Desktop/eklenti/.github/workflows/ci.yml:4).

push.paths yalnızca manifest, lock ve workflow dosyalarını kapsıyor. Sadece analyzer.rs, bir React dosyası veya test değişen main push'u bu workflow'u çalıştırmıyor. README'deki “every ... push to main” iddiası karşılanmıyor.

PR olayı tanımlı olduğu için PR testleri bundan bağımsız çalışabilir; sorun özellikle doğrudan main push'ları ve main'deki sonucun tekrar doğrulanması.

**Düzeltme yönü:** Kaynak/test/build/config yollarını kapsa veya üst seviye push path filtresini kaldır; mevcut changes job'ıyla dokümantasyon optimizasyonu yap.

**Regresyon:** Yalnızca .rs, .tsx, .css ve test değişen push; yalnızca dokümantasyon değişikliği.

### F33 — P2 — Whitespace içermeyen kısa PHP açılış etiketi taranmıyor

**Kanıt:** Ferry ve gerçek PHP ile doğrulandı.

**Konum:** [lexer.rs:134](/Users/efe/Desktop/eklenti/crates/ferry-core/src/lexer.rs:134).

    <?each($a);?>

Lexer <? sonrasında whitespace aradığı için bütün kodu HTML gibi atlıyor. short_open_tag=1 ile gerçek PHP bu kaynağı kod olarak çalıştırıyor ve undefined each() fatal hatası veriyor. Ferry sonucu boş.

**Düzeltme yönü:** Legacy kısa etiketler için açık tarama politikası belirle. XML ile ayırt etme ihtiyacı varsa belirsizliği raporla; desteklenen eski kodu sessizce kapsam dışına çıkarma.

**Regresyon:** <?echo, <?if, <? boşluklu biçim, XML bildirimi ve <?=.

### F34 — P2 — Sayfalama ve filtre birleşince bulgu listesinde hiçbir klavye odağı kalmıyor

**Kanıt:** Ek React testiyle doğrulandı.

**Konum:** [Findings.tsx:71](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:71), [Findings.tsx:89](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:89), [Findings.tsx:277](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:277).

**Senaryo:** 420 bulgulu listede End ile sona git; aramaya tüm .php dosyalarını hâlâ eşleştiren “php” yaz.

visible yeni array olduğundan gösterim limiti 300'e düşüyor. selectedId ise 420. bulguyu seçili tutuyor. Seçili öğe render edilmediği için çizilen 300 option'ın hepsi tabIndex=-1 oluyor.

**Gözlenen:** Render edilen option sayısı 300, tabIndex=0 olan option sayısı 0.

**Düzeltme yönü:** Seçili öğe korunuyorsa onu kapsayacak gösterim aralığını koru; aksi durumda görünür bir öğeyi seç ve tek tab stop invariant'ını sürdür.

**Regresyon:** End → filtre, “Daha fazla” → filtre, ignore sonrası seçili öğenin kaybolması ve rescan sonrası liste küçülmesi.

## 5. Daha küçük sorunlar ve ek edge case'ler

Bu tablo da kaynak incelemesinde tespit edilen işlerdir; P1/P2 güvenlik maddelerinin önüne geçirilmemeli.

| ID | Öncelik / kanıt | Konum | Sorun ve öneri |
|---|---|---|---|
| K01 | P3, ek React testi | [store.tsx:46](/Users/efe/Desktop/eklenti/src/lib/store.tsx:46) | RecentProject doğrulaması yalnızca wordpress/laravel/symfony/drupal/generic kabul ediyor. Magento, PrestaShop, Joomla, Moodle, MediaWiki ve phpBB kayıtları yeniden açılışta siliniyor. Gerçek Framework enum'undan türet. |
| K02 | P3, kod akışı | [Findings.tsx:50](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:50), [Findings.tsx:229](/Users/efe/Desktop/eklenti/src/screens/Findings.tsx:229) | “Seen at runtime” filtresi NO_FILTER dışında. Tek başına aktifken “Filtreleri temizle” görünmüyor; diğer filtreler temizlense de runtime filtresi kalıyor. Tek bir filtre modeline alınmalı. |
| K03 | P2, kod akışı | [Modal.tsx:83](/Users/efe/Desktop/eklenti/src/components/Modal.tsx:83) | Ignore dialog ilk açıldığında confirm disabled. Başlangıç odağı bu disabled butona verilmeye çalışıldığı için odak alttaki ekranda kalabiliyor. İlk kullanılabilir kontrol/textarea veya dialog başlığı odaklanmalı; arka içerik inert olmalı. Mevcut axe senaryosu ignore dialog'u kontrol etmiyor. |
| K04 | P3, kod akışı | [Tauri lib.rs:525](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:525) | Git dirty uyarısı untracked-files=no kullanıyor; yeni ve commit edilmemiş dosyalar uyarıya girmiyor. Porcelain metni -z olmadan parse edildiği için boşluk, Unicode escape ve “ -> ” içeren dosya adları da sorunlu. Status -z ve untracked kapsamı kullanılmalı. |
| K05 | P3, kod akışı | [history.rs:70](/Users/efe/Desktop/eklenti/crates/ferry-core/src/history.rs:70) | Saklama limiti hedef başına 30 değil toplam 150. Bir hedefte 150+ tarama diğer hedeflerin son kayıtlarını silebilir. Hedef başına prune edilmeli. |
| K06 | P3, kod akışı | [store.tsx:207](/Users/efe/Desktop/eklenti/src/lib/store.tsx:207), [Tauri lib.rs:184](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:184) | Projeyi kapatma/değiştirme frontend sonuçlarını temizliyor; backend runtime cache'i temizlenmiyor. Aynı proje yeniden açılıp taranınca eski log geri birleşebilir. README log'un proje kapanınca silindiğini söylüyor. Proje session yaşam döngüsü gerekli. |
| K07 | P2, kod akışı | [server.rs:29](/Users/efe/Desktop/eklenti/crates/ferry-core/src/server.rs:29), [compare.rs:36](/Users/efe/Desktop/eklenti/crates/ferry-core/src/compare.rs:36) | Server taramasındaki depth/entry/finding sınırları ve okuma hataları kapsam uyarısı üretmiyor; compare worker panic'i de eksik hedef satırı olarak yutuluyor. Truncation/error bilgisi F08'deki ortak coverage modeline bağlanmalı. |
| K08 | P3, çalıştırıldı | [diff.rs:110](/Users/efe/Desktop/eklenti/crates/ferry-core/src/diff.rs:110), [diff.rs:217](/Users/efe/Desktop/eklenti/crates/ferry-core/src/diff.rs:217) | “No newline at end of file” işaretleri atlanıyor. one\\n → two, final newline yok diff'i two\\n üretti. Final newline politikası diff'ten okunmalı veya bilinçli normalizasyon olarak belgelenmeli. |
| K09 | P3, kod akışı | [CLI main.rs:266](/Users/efe/Desktop/eklenti/crates/ferry-cli/src/main.rs:266) | SARIF artifact URI'leri sadece slash normalizasyonu yapıyor; boşluk, #, % ve Unicode dosya adları URI olarak encode edilmeli. Aksi hâlde konum bağlantıları yanlış yorumlanabilir. |
| K10 | P2, kod akışı | [SettingsScreen save, Other.tsx:766](/Users/efe/Desktop/eklenti/src/screens/Other.tsx:766), [ignore.rs:37](/Users/efe/Desktop/eklenti/crates/ferry-core/src/ignore.rs:37) | Ayar/ignore yazmaları ortak sürüm veya işlem kilidi olmadan read-modify-write yapıyor. Hızlı iki ayar değişikliği eski settings kopyasıyla bir önceki seçimi silebilir; eşzamanlı ignore işlemleri ortak .tmp dosyasında yarışabilir. Alan bazlı güncelleme veya seri yazma kullanılmalı. |
| K11 | P3, kod akışı | [Tauri lib.rs:836](/Users/efe/Desktop/eklenti/src-tauri/src/lib.rs:836), [snapshot.rs:185](/Users/efe/Desktop/eklenti/crates/ferry-core/src/snapshot.rs:185) | Tek snapshot silme apply/restore ile aynı writes kilidini kullanmıyor. Ayrıca list() trash klasörlerini temizleyerek okuma sırasında silme yapıyor. Yedek yaşam döngüsü işlemleri ortak koordinasyona alınmalı. |

## 6. Mimari ve güven modeli

### 6.1. “Güvenli dosya işlemi” çekirdeğin birinci sınıf kavramı olmalı

Şu anda güvenlik kuralları rector, snapshot ve Tauri handler'ları arasında dağınık. Genel ferry_core::rector::apply API'si kendi başına snapshot/çok dosyalı rollback sağlamıyor; güvence üst katmandaki handler'a bağlı.

Önerilen ortak akış:

1. Plan: onaylanan preview ID, hedef, dosya kimlikleri, başlangıç hash'leri ve amaçlanan byte içeriği.
2. Preflight: root/bağlantı politikası, tüm dosyaların erişimi, hedef PHP ile mümkünse syntax kontrolü.
3. Prepare: dayanıklı snapshot + içerik hash'leri + işlem journal'ı; bunların tamamlanması zorunlu.
4. Commit: dosya başına güvenli yazma; her adımın durumu kaydedilir.
5. Recover: hata/crash sonrası tamamlanmamış işlemi tanı; yedeği koru ve kurtarma sun.

Batch başlangıcındaki iki check_fresh çağrısı, harici editörün kontrol ile yazma arasında değişiklik yapmasını tümüyle engellemiyor. Dosya başına son kontrol ve kimlik doğrulaması gerekli. Kalıcı journal olmadan süreç çökmesi/elektrik kesilmesi çok dosyalı apply'ı yarıda bırakabilir.

### 6.2. “Bulgu” ile “kanıt” ayrılmalı

Statik tahmin, çalışma anında gözlenen olay, framework destek matrisi ve kullanıcı ignore kararı farklı bilgiler. Bunların tek Finding içine düzleştirilmesi F06, F21 ve F22 gibi sorunlara yol açıyor.

Her sonuç için en az şunlar yararlı:

- Kaynak: static/runtime/framework/dependency/server.
- Güven seviyesi ve neden.
- Analiz hedefi, gerçek runtime PHP sürümü, dosya içerik hash'i ve scan/session ID.
- Runtime olayının ham mesajı; ilişkilendirilen statik bulgular ayrı ilişkiler olarak.
- Ignore kaynağı: JSON/inline; kapsam ve kararlı kimlik.

Eski bir log, hedef sürüm değişince kendiliğinden yeni hedefte gözlenmiş kanıt sayılmamalı. Target karşılaştırması bugün runtime log'unu hesaba katmazken ana özet katabiliyor; karşılaştırma kapsamı görünür olmalı.

### 6.3. Ortak subprocess runner gerekli

Rector, PHPUnit, PHP keşfi/lint, Composer, curl ve git farklı yürütme yollarında. Bazı çağrılarda timeout yok. İptalde yalnızca doğrudan child öldürülüyor; Rector/PHPUnit worker'ları ve miras aldıkları pipe'lar yaşayabilir.

Tek runner; ayrı stdout/stderr tüketimi, timeout/cancel, process group veya Windows Job Object, process reap, çıktı boyutu sınırı ve yapılandırılmış sonuç sağlamalı. Kullanıcının uygulamayı kapatması da aktif test/lint işlerini kapsamalı.

### 6.4. Dosya bir kez parse edilip ortak analiz modeli kullanılmalı

PHP'nin yaklaşık grameri lexer/analyzer/index/guards/extensions arasında tekrar yorumlanıyor. F13–F16 gibi farklar bunun sonucu. Her dosyayı birden fazla kez okuma/tokenize etme de maliyeti artırıyor.

İlk adım bütün sistemi değiştirmek olmak zorunda değil: tek source snapshot, tek token stream, namespace/import/scope modeli ve açık “bilinmiyor” durumu oluşturulabilir. Sonra gerçek AST/CST parser kullanımı değerlendirilir.

Property, function, constant ve method kimlikleri aynı normalizasyon kurallarına sahip değil. Tek bir lowercase/kısa ad yaklaşımı yeterli değil.

### 6.5. Rector kuralları için güvenli varsayılan belirsiz olmalı

describe_rule, LIKELY_RULES listesinde olmayan her şeyi Auto sayıyor. Compat modu da modernizasyon kurallarını isim blacklist'iyle dışlıyor. Buna karşılık araç kurulumu rector/rector için ^2.0 aralığında güncelleme yapıyor.

Yeni/yeniden adlandırılmış bir Rector kuralı, Ferry tarafında ayrıca gözden geçirilmeden Auto sınıfına veya Compat moduna girebilir. F04'te mevcut bir sınıflama hatası zaten görüldü.

Rector sürümü/profil'i kayda alınmalı; doğrulanmış kurallar açıkça Auto kabul edilmeli. Bilinmeyen kural Review olmalı. Sürüm güncellemesi, yeni kurallar için sözleşme testlerini tetiklemeli.

### 6.6. Sonuç yaşam döngüsü ve hata modeli merkezileştirilmeli

scans/runtime/previews path ile anahtarlanan bağımsız cache'ler; frontend generation/previewSeq, backend latest_scan ve cancellation token'ları ayrı mekanizmalar.

Bu, stale response savunmaları açısından iyi bir başlangıç. Ancak openProject için sıralama koruması yok; yavaş eski inspect cevabı sonradan gelip yeni proje seçimini ezebilir. Uzun süren local check'in farklı scan sonucuyla birleşmesi ve ayar değişikliği sırasında eski callback'lerin tamamlanması da aynı sınıf.

Ortak OperationContext: projectSessionId + scanId + target + inputHash + operationId. Backend ve frontend aynı bağlamı doğrulamalı. Hatalar kullanıcı mesajına çok erken çevrilmeden typed error code/detail şeklinde taşınmalı.

## 7. Arayüz ve ürün tasarımı

### İyi çalışan yönler

- Sakin renk paleti, sistem fontları ve tutarlı spacing; geliştirici aracı kimliğine uygun.
- Severity ile düzeltme türünün ayrı rozetlerde sunulması doğru.
- Dosya → bulgu → neden → öneri akışı anlaşılır.
- Diff önizlemesi, dosya seçimi ve apply onayı iyi bir temel.
- Klavye kullanımı, reduced-motion, iki dil ve axe testleri düşünülmüş.

### D01 — Hazırlık skoru, çalışabilirlik garantisi gibi görünmemeli

**Konum:** [report.rs:23](/Users/efe/Desktop/eklenti/crates/ferry-core/src/report.rs:23), [labels.ts:42](/Users/efe/Desktop/eklenti/src/lib/labels.ts:42), [Overview.tsx:160](/Users/efe/Desktop/eklenti/src/screens/Overview.tsx:160).

Skor yalnızca bulgu sayısı × severity × düzeltme eforundan geliyor. Bir Auto breaking bulgu yaklaşık 98/100; tek Manual breaking yaklaşık 90/100 veriyor. Bunlar hâlâ uygulamayı durdurabilir. Paket engelleri, tarama eksikleri ve testlerin çalışmamış olması skora dahil değil. Sıfır statik bulgu doğrudan “Hedef sürüme hazır”.

**Öneri:** Üstte bağımsız durum göster:

- Geçişi engelleyen sorunlar: var/yok.
- Analiz kapsamı: tam/kısmi.
- Bağımlılıklar: doğrulandı/belirsiz/engelli.
- Hedefte test/lint: geçti/başarısız/çalıştırılmadı.

Sayısal skor kalacaksa “tahmini düzeltme yükü” olarak açıklanmalı. Yeşil halka, fatal sorun veya doğrulanmamış kapsamı gölgelememeli.

### D02 — Özet ekranında karar bilgisi ilk görünümde daha yukarı gelmeli

1280×720 görünümünde platform kartı, büyük hedef kartı ve sürekli güvenlik açıklaması ilk alanın önemli kısmını kaplıyor. Skor kartının ardından gelen framework engeli ekranın alt sınırına dayanıyor; kritik sayılar, karşılaştırma ve sonraki adımlar aşağıda.

**Öneri:** Tarama öncesi ve sonrası üst alanı farklılaştır. Tarama sonrasında hedefi kompakt bir başlık kontrolüne indir; kritik engel ve kapsam bilgisini skorun yanında göster. Güvenlik açıklamasını kısa bir uyarı + ayrıntı açılımı hâline getir.

### D03 — Minimum pencere boyutunda sabit panel oranları fazla sıkışıyor

980×640 için layout alt alta geçiyor. Bulgularda toolbar'ın altında ancak yaklaşık iki satır; Düzeltmeler'de dosya listesinde yaklaşık tek dosya görünüyor. Kullanıcı listeler ve diff arasında çok sayıda bağımsız scroll yapmak zorunda.

**Konum:** [app.css:2678](/Users/efe/Desktop/eklenti/src/styles/app.css:2678).

**Öneri:** Yeniden boyutlandırılabilir splitter; dar görünümde liste/ayrıntı sekmesi veya tam ekran ayrıntı; toolbar için daha kompakt yerleşim. Seçili dosya ve apply eylemi sabit kalmalı.

### D04 — Düzeltme seçiminde “görüldü” ile “uygulanacak” farklı olmalı

Bütün değişiklikler, Review olanlar dahil, başlangıçta seçili. Kullanıcı dosyaları açmadan “Tümünü uygula”ya gidebiliyor. “Kurala göre seç” aslında o kuralın bulunduğu dosyaların bütün kurallarını seçiyor.

**Öneri:** Review dosyalarını ayrıca onaylamak; incelenen/incelenmeyen durumunu göstermek; onay ekranında Auto/Review dosya sayılarını belirtmek. Kurala göre seçimin **dosya bazlı** olduğu açık yazılmalı. Hunk/rule bazlı apply bulunmuyorsa yalnızca tek kural uygulanacağı izlenimi oluşmamalı.

### D05 — Uzun kontrollerin kalıcı sonuç yüzeyi eksik

Lint/test sonucu ve lint'in uygulanamadığı durum bir toast'a veya runtime sayısına sığmıyor. Kullanıcı “hangi PHP, hangi komut, kaç test, hangi exit status?” sorularına tek ekrandan cevap bulmalı.

**Öneri:** Check history veya son kontrol kartı; command/version/status, özet ve açılabilir log. Testler başarısızken “0 runtime olay” olumlu sonuç gibi görünmemeli.

### D06 — “Canlı test” navigasyonu mevcut runtime özelliklerinin keşfini zorlaştırıyor

Sidebar “Canlı test — Yakında” derken mevcut log import/lint/PHPUnit işlemleri Bulgular toolbar'ında. Kullanıcı runtime özelliklerinin tamamının henüz yapılmadığını düşünebilir.

**Öneri:** Mevcut kontroller için “Doğrulama” alanı oluştur. Docker karşılaştırması onun içinde açıkça gelecek özellik olsun. AI placeholder'ı ana iş akışının önüne geçmesin.

### D07 — Erişilebilirlik güvencesi daha dar ifade edilmeli

axe testlerinde color-contrast ve region kuralları kapalı. Bu testlerin geçmesi WCAG 2.1 AA'nın tamamının doğrulandığı anlamına gelmiyor. F34 ve K03 gibi etkileşim hataları axe ile tek başına yakalanmıyor.

**Konum:** [a11y.test.tsx:8](/Users/efe/Desktop/eklenti/src/test/a11y.test.tsx:8).

**Öneri:** Gerçek tarayıcıda iki tema için kontrast, 200% zoom, klavye-only ve ekran okuyucu smoke testi. Modal, error state, boş liste ve 300+ öğeli liste senaryoları kapsamda olmalı.

### D08 — Kesin ürün iddiaları mevcut kapsamla hizalanmalı

Welcome/README'deki “kaldırılan ya da değişen her şeyi bulur”, “davranışı değiştirmeyen otomatik dönüşüm” ve “hiçbir dosya değişmez” gibi ifadeler çok kesin. README kapsam sınırlamalarını ve Rector'ın proje autoload kodu çalıştırdığını daha aşağıda açıklıyor; bu iyi, fakat karar anında aynı bağlam görünür değil.

**Öneri:** Statik tarama kapsamını ve belirsizlikleri özetin parçası yap. Rector dry-run'ının kod yükleyebildiğini preview başlatma yüzeyinde kısa biçimde anlat. Bu, ek bir zorunlu onay diyaloğu olmak zorunda değil; doğru açıklama yeterli olabilir.

PHP güvenlik sürümü eşikleri ayrıca resmi PHP duyurusuyla kontrol edildi; 8.2.35/8.3.36/8.4.27/8.5.12 değerleri bu inceleme tarihinde duyuruyla uyumluydu. Bu değerler hata olarak işaretlenmedi. Yine de sabit tablolara kaynak ve doğrulanma tarihi eklenmeli. [PHP resmi duyuruları](https://www.php.net/index.php)

## 8. Eksikler ve geliştirilebilecek yetenekler

Bu bölüm mevcut bug listesinden ayrıdır. Bazıları ROADMAP'te zaten doğru biçimde yer alıyor.

| Alan | Eksik / neden önemli | Önerilen yaklaşım |
|---|---|---|
| Analiz kapsamı | Eksik tarama, bilinmeyen dependency ve parser belirsizliği ortak bir modelde değil | Coverage + confidence + typed errors; F08 ve F19'un kalıcı temeli |
| Proje yapılandırması | Hedef, exclude, test komutu ve ortam eşlemesi masaüstü/CI arasında tek kaynak değil | Sürümlü .ferry config; GUI/CLI aynı ayarı okusun |
| Extension/platform gereksinimleri | require, require-dev, conflict ve lock platform bilgisi bütün değil | Composer platform modelini birleştir; F17/F20 |
| PHP kuralları | Değer/akış gerektiren değişimler ve bazı dil/API formları eksik | Resmi migration catalog → rule → fixture → runtime/lint oracle matrisi |
| Runtime kanıtı | Kullanılan PHP sürümü, remote-root ve source revision bağlı değil | Import metadata ve içerik bağlamı; belirsiz eşleşmeleri ayrı göster |
| Gerçek doğrulama | Kaynak ve hedef PHP'de aynı test/komutu karşılaştıran tamamlanmış akış yok | Önce yerel kontrolleri doğru modelle; sonra isteğe bağlı Docker runner |
| Çoklu Composer root | Monorepo/alt uygulama bağımlılık kapsamı tek root varsayımına takılıyor | Kökleri keşfet ve kullanıcıya seçtir; dosya → paket root ilişkisi |
| Değişiklik granülerliği | Dosya seçimi var, hunk/kural bazlı apply yok | Önce mevcut seçimin açıklamasını düzelt; sonra bağımlı hunk'ları gözeten plan |
| Kurtarma | Crash sonrası apply journal/recovery yok | F01/F05/F10'u ortak transaction modeliyle çöz |
| Büyük projeler | Aynı kaynak tekrar okunuyor/tokenize ediliyor; karşılaştırmada index tekrar kuruluyor | İçerik hash'li source snapshot/cache; önce F26 gibi ölçülmüş darboğazlar |
| Sürüm matrisleri | Framework destek tablolarında kaynak/provenance yok | Her satıra resmi kaynak, doğrulanma tarihi ve support-vs-known-breakage ayrımı |
| Test zorunluluğu | Araç yoksa kritik testler yeşil görünerek atlanabiliyor | Zorunlu integration job'ı eksik araçta fail etsin; yerel optional testler açık skipped raporlasın |

### Kapsam genişletirken özellikle test edilmesi gereken PHP örnekleri

- Imported/aliased global fonksiyonlar ve çoklu namespace.
- Hedefe göre aktif/pasif polyfill tanımları ve conditional include'lar.
- Farklı dosyalardaki const/define ve namespaced sabitler.
- Case-sensitive property'ler, private miras ve trait adaptation.
- Aynı kısa adlı kullanıcı/built-in sınıflar ve belirsiz receiver tipleri.
- Named arguments, unpacking, first-class callable, attribute, heredoc/nowdoc ve kısa açılış etiketi kombinasyonları.
- Blade/Twig gibi PHP dışı template sözdiziminin taranıp taranmadığının açık belirtilmesi.
- Dinamik çağrılar, null/array key değerleri, resource→object geçişleri ve locale davranışı için runtime kanıtı.
- PHP sürümü pinlerinin farklı CI job'ları, container stage'leri ve multiline YAML içinde bulunması.

Katalog kapsamını artırmadan önce yanlış otomatik düzeltme üreten F04 ve bilinmeyen sonucu güvenli sayan yollar kapatılmalı.

## 9. Önerilen uygulama sırası

### Aşama 1 — Veri güvenliği ve güvenilir sonuç

1. F01–F05: Dosya yazma/restore politikası, link davranışı, içerik bağlı onay ve yedek bütünlüğü.
2. F06–F08: Runtime olayını kaybetmeyen model, doğru CheckResult ve partial-scan hata davranışı.
3. F10/F11/F31: Restore preflight, zorunlu metadata ve anlamlı lint sonucu.
4. Bu değişiklikler için arıza enjeksiyonlu regresyon testleri.

**Bitiş ölçütü:** Hiçbir başarısız işlem sessiz veri kaybı üretmiyor; eksik doğrulama veya fatal olay CI/UI'da olumlu sonuca dönüşmüyor.

### Aşama 2 — Analiz doğruluğu

1. F13–F18: Scope/symbol resolver, property kimlikleri ve Composer semver.
2. F19–F25: Unknown dependency, extension envanteri, runtime eşleştirme ve ignore kimlikleri.
3. F26/F33: Ölçülmüş performans ve lexer edge case'leri.

**Bitiş ölçütü:** Rapordaki örnekler kalıcı fixture'lara dönüşmüş; doğru alarm kadar yanlış alarmı engelleyen testler de var.

### Aşama 3 — İş akışı, tasarım ve dağıtım

1. F09/F32: Kullanılan CLI binary'sinin ve CI tetiklerinin doğruluğu.
2. F27–F30/F34: Boş durumlar, input validation, PHP seçimi, seçim/focus sürekliliği.
3. D01–D07: Hazırlık mesajı, doğrulama alanı, dar pencere ve erişilebilirlik.
4. Küçük tutarsızlıklar ve proje konfigürasyonu.

Yeni AI/Docker özelliklerini bu temelden sonra ele almak daha doğru olur; aksi hâlde mevcut güven modeli daha fazla yürütme/yazma yoluyla karmaşıklaşır.

## 10. Claude'a uygulama notu

Bu belge bir inceleme raporudur; hiçbir düzeltme uygulanmış değildir. Bulgular commit 38cd8341f22a3cdee4beda042d362e2a158e52f5 için hazırlanmıştır. Kod değişmişse ilgili akışı yeniden doğrula.

Uygulamaya ayrıca yetki verilirse:

- Önce F01–F08 için küçük, ayrı regresyon fixture'ları oluştur; mevcut davranışı yeniden üret.
- Ortak güvenli dosya işlemi ve CheckResult modelini kur; yalnızca UI mesajlarını değiştirerek kritik maddeleri kapatma.
- Auto sınıflaması için açık kanıt iste; belirsiz dönüşümü güvenli varsayma.
- Kaynakları düzenlerken demo projeleri ve kullanıcı projesi fixture'larını ayır; gerçek kullanıcı kodunda doğrulama çalıştırma.
- Test raporunda gerçekten çalışan Rector/IPC kontrollerini erken return eden testlerden ayır.
- Windows link/rename/readonly semantiğini ve Linux izinlerini ayrıca test et; bu rapordaki dinamik dosya sistemi kanıtı macOS'tadır.

Özellikle F01, F04, F05, F06 ve F08'deki gözlenen sonuçlar düzeltilmeden “güvenli yükseltme” güvencesi güçlendirilmiş sayılmamalı.
