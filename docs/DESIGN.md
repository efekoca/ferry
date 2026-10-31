# Ferry Tasarım Sistemi

Ferry, PHP projelerini yeni sürüme taşıyan bir masaüstü uygulamasıdır. Arayüz bir
geliştirici aracı gibi **yoğun, sakin ve güven veren** olmalı: dikkat, kullanıcının koduna
ve risklere gider; süslemeye değil.

## İlkeler

1. **Önce güven.** Her değişiklik görülebilir, geri alınabilir ve gerekçelidir. Hiçbir
   işlem sessizce olmaz.
2. **Renk anlam taşır.** Vurgu rengi yalnızca birincil eylemler ve seçim içindir. Risk
   renkleri yalnızca risk bildirir; dekor olarak kullanılmaz.
3. **Doğal his.** macOS'ta saydam kenar çubuğu ve gömülü başlık çubuğu, Windows'ta yerel
   pencere çerçevesi. Sistem yazı tipleri, sistem tema tercihi.
4. **Yoğun ama ferah.** 13px taban yazı boyutu (masaüstü standardı), 4px boşluk ızgarası.
5. **Erişilebilir.** WCAG 2.1 AA: metin kontrastı en az 4.5:1, kontrol kenarları ve durum
   göstergeleri en az 3:1, her etkileşim klavyeyle yapılabilir, odak halkası her zaman
   görünür. `src/test/a11y.test.tsx` her ekranı axe ile denetler.

## Tokenlar

Tüm tokenlar `src/styles/tokens.css` içinde CSS değişkeni olarak tanımlıdır. Bileşenlerde
**sabit renk, boşluk veya yazı boyutu kullanılmaz.**

### Renk — nötr

| Token | Açık | Koyu | Kullanım |
|---|---|---|---|
| `--bg` | `#F6F6F8` | `#0D0D11` | Uygulama zemini |
| `--surface` | `#FFFFFF` | `#15151A` | Kart, panel |
| `--surface-2` | `#F1F1F4` | `#1C1C22` | İç içe yüzey, hover |
| `--surface-3` | `#E9E9EE` | `#24242B` | Basılı, seçili satır |
| `--border` | `#E3E3E8` | `#26262E` | Varsayılan çizgi |
| `--border-strong` | `#D0D0D8` | `#34343E` | Belirgin ayırıcı, seçenek kartı |
| `--border-input` | `#84848F` | `#70707E` | Girdi, onay kutusu ve seçili segment kenarı (zemine karşı en az 3:1) |
| `--text` | `#16161D` | `#ECECF1` | Birincil metin |
| `--text-2` | `#585866` | `#A3A3B0` | İkincil metin |
| `--text-3` | `#646471` | `#8A8A97` | İpucu, devre dışı (her yüzeyde en az 4.5:1) |

### Renk — marka

| Token | Açık | Koyu | Kullanım |
|---|---|---|---|
| `--accent` | `#2156D9` | `#5B9BFF` | Birincil buton, seçim, odak |
| `--accent-hover` | `#1A47BD` | `#7AAEFF` | Hover |
| `--accent-soft` | `#2156D91A` | `#5B9BFF24` | Seçili gezinme öğesi zemini |
| `--accent-fg` | `#FFFFFF` | `#0D0D11` | Vurgu üstündeki metin |

Vurgu rengi Ferry logosundaki denizden gelir: güven veren, doygun bir mavi.

### Renk — risk ve durum

Her risk rengi üç tonla gelir: `-fg` (metin/ikon), `-bg` (rozet zemini), `-border`.

| Anlam | Token öneki | Açık fg | Koyu fg | Ne zaman |
|---|---|---|---|---|
| Otomatik | `--risk-auto` | `#127A46` | `#4ADE80` | Kural tabanlı, güvenli dönüşüm |
| Gözden geçir | `--risk-likely` | `#6D3FD8` | `#B29BFF` | Otomatik yapılır ama bakılmalı |
| Elle düzelt | `--risk-manual` | `#B45309` | `#FBBF24` | Dinamik kod, belirsiz niyet |
| Kritik | `--danger` | `#C62A2A` | `#F87171` | Yeni sürümde ölümcül hata |

### Tipografi

| Token | Değer | Kullanım |
|---|---|---|
| `--font-sans` | `-apple-system, BlinkMacSystemFont, "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif` | Arayüz |
| `--font-mono` | `ui-monospace, "SF Mono", "Cascadia Code", "JetBrains Mono", Consolas, monospace` | Kod, yol, sürüm |
| `--text-xs` | 11px / 16px | Rozet, meta |
| `--text-sm` | 12px / 18px | İkincil metin, tablo |
| `--text-md` | 13px / 20px | **Taban** |
| `--text-lg` | 15px / 22px | Kart başlığı |
| `--text-xl` | 20px / 28px | Sayfa başlığı |
| `--text-2xl` | 28px / 34px | Büyük metrik |
| `--text-3xl` | 40px / 44px | Skor |

Ağırlıklar: 400 gövde, 500 etiket ve buton, 600 başlık. Rakamlar `tabular-nums`.

### Boşluk (4px ızgara)

`--space-1` 4 · `--space-2` 8 · `--space-3` 12 · `--space-4` 16 · `--space-5` 20 ·
`--space-6` 24 · `--space-8` 32 · `--space-10` 40 · `--space-12` 48

### Kenar yarıçapı

`--radius-sm` 5px (rozet, onay kutusu) · `--radius-md` 7px (buton, girdi) ·
`--radius-lg` 11px (kart) · `--radius-xl` 16px (modal, boş durum) · `--radius-full`

### Gölge (yükselti)

| Token | Kullanım |
|---|---|
| `--shadow-1` | Kart (çok hafif, koyu temada yerini kenarlığa bırakır) |
| `--shadow-2` | Açılır menü, ipucu |
| `--shadow-3` | Modal, bildirim |

### Hareket

`--ease-out: cubic-bezier(.2,.8,.2,1)` · `--dur-fast` 120ms (hover) ·
`--dur-base` 200ms (panel) · `--dur-slow` 320ms (sayfa geçişi).
`prefers-reduced-motion` açıksa tüm geçişler kapatılır.

## Bileşenler

### Button

| Varyant | Ne zaman |
|---|---|
| `primary` | Ekrandaki tek ana eylem ("Taramayı başlat", "Uygula") |
| `secondary` | Destekleyici eylem ("Raporu dışa aktar") |
| `ghost` | Araç çubuğu, düşük vurgu |
| `danger` | Geri dönüşü zor eylem ("Anlık görüntüyü geri yükle") |

Boyutlar: `sm` 26px, `md` 32px, `lg` 40px. Durumlar: varsayılan, hover (zemin bir ton
koyulaşır), basılı (`scale(.98)`), odak (2px vurgu halkası, 2px boşluk), devre dışı (%45
opaklık), yükleniyor (sol ikon yerini döner göstergeye bırakır, genişlik sabit kalır).

**Erişilebilirlik:** yerel `<button>`; yükleniyorken `aria-busy="true"`.

| ✅ Yap | ❌ Yapma |
|---|---|
| Ekranda tek `primary` | Aynı satırda iki `primary` |
| Fiil ile başla ("Uygula") | "Tamam", "Evet" |

### Badge / RiskBadge

Küçük, yuvarlatılmış etiket. `RiskBadge` her zaman **ikon + metin** taşır; anlam yalnızca
renge bırakılmaz (renk körlüğü). Varyantlar: `auto`, `likely`, `manual`, `danger`,
`neutral`, `accent`.

### Card

`--surface` zemin, `--border` kenarlık, `--radius-lg`, `--shadow-1`. Başlık + isteğe bağlı
eylem alanı + gövde. Tıklanabilir kartta hover'da kenarlık `--border-strong` olur.

### Sidebar

Genişlik 232px. macOS'ta saydam (sistem titreşim efekti), üstte 52px trafik ışığı alanı
sürüklenebilir. Windows'ta düz `--surface-2`. Öğeler 30px yükseklikte; seçili öğe
`--accent-soft` zemin + `--accent` ikon. Sağda sayı rozetleri.

### Metric

Büyük rakam (`--text-2xl`, tabular), üstte küçük etiket, altta isteğe bağlı açıklama.
Skor için halka gösterge (`ScoreRing`): 0–49 tehlike, 50–79 elle kontrol, 80+ otomatik
renginde.

### Progress

İnce (4px) çubuk, `--accent` dolgu, belirsiz durumda kayan animasyon. Adım listesi
(`Steps`) her adım için bekliyor / çalışıyor / tamam / hata durumu gösterir.

### DiffViewer

Birleşik diff görünümü. Satır numaraları iki sütun (eski/yeni), `--font-mono` 12px.
Eklenen satır `--diff-add-bg`, silinen `--diff-del-bg`, değişen kelimeler daha koyu
vurgulanır. Hunk başlıkları `--surface-2` şeritte. Uzun dosyalarda değişmeyen bölümler
katlanır.

### Segmented / Select

Sürüm seçici için segmentli kontrol: 8.1 · 8.2 · 8.3 · 8.4 · 8.5. Seçili segment
`--surface` zemin + `--shadow-1` (macOS tarzı).

Seçeneklerin açıklaması kararı etkiliyorsa (ör. Rector modu) segment yerine **seçenek
kartları** (`.mode-card`) kullanılır: her kart başlık + bir cümle açıklama taşır, önerilen
seçenek "Önerilen" etiketiyle işaretlenir. Kartlar yerel `<input type="radio">` üzerine kuruludur.

### Filtre çubuğu

Liste ekranlarında üstte tam genişlik arama, altında sarılabilen filtre çipleri, en altta
"X / Y bulgu" özeti ve yalnızca filtre etkinken görünen **Filtreleri temizle**. Diğer
ekranlardan gelen filtreler (ör. kural) kaldırılabilir bir çip olarak gösterilir. Filtre durumu
store'da tutulur; ekrandan çıkıp dönünce korunur.

### CommandLine

Kullanıcının terminalde çalıştıracağı komut: mono metin + kopyala düğmesi. Komut her zaman
projeye özgü olarak üretilir; `vendor/package` gibi yer tutucu yazılmaz.

### EmptyState

Ortada ikon (48px, `--text-3`), başlık, tek cümle açıklama, en fazla bir birincil eylem.

### Toast

Sağ altta, `--shadow-3`, 4 saniye sonra kaybolur; hata toast'ları kullanıcı kapatana
kadar kalır. `role="status"` (hata için `role="alert"`).

## Kalıplar

- **Odak halkası:** `outline` ile çizilir (2px `--accent`, 2px boşluk), böylece bileşenlerin
  kendi `box-shadow`'u onu ezemez. Kenarları kırpılan kapsayıcılardaki satırlarda boşluk
  içeri (`-2px`) alınır.
- **Uzun listeler:** tek sekme durağı (roving tabindex), içinde ok tuşları, Home/End. Liste
  ilk 300 öğeyi çizer, "Daha fazla göster" ya da klavyeyle sona gelindiğinde büyür.
- **Dar görünüm ve yakınlaştırma:** ⌘/Ctrl + ile yakınlaştırma açıktır. 1000px altında
  bölünmüş ekranlar alt alta dizilir; 760px altında kenar çubuğu simgelere iner, etiketler
  görsel olarak gizlenir ama erişilebilir ad olarak kalır.
- **Sayfa düzeni:** 52px başlık şeridi (sayfa adı + sağda eylemler), altında kaydırılabilir
  içerik, iç boşluk `--space-6`, en fazla 1180px genişlik.
- **Liste + ayrıntı:** sol listede seçim, sağda ayrıntı paneli (bulgular, değişiklikler).
- **İki aşamalı onay:** kalıcı değişikliklerden önce özet ("12 dosya değişecek, anlık
  görüntü alınacak") ve açık onay. Hesaplanmış bir önizlemeyi silecek işlemler (ör. hedef
  sürümü değiştirmek) de önce onay ister.
- **Özetten ayrıntıya:** özet ekranındaki her sayı tıklanabilir ve ilgili listeyi o filtreyle
  açar (Kritik kartı → yalnızca kritik bulgular).
- **Sonuçları canlı tut:** girdiyi değiştiren bir ayar (hedef sürüm) mevcut sonucu boşaltıp
  kullanıcıyı yeniden başlatmaya zorlamaz; tarama varsa otomatik olarak yenilenir.
- **Sabit stil yok:** bileşenlerde `style={{…}}` yalnızca veriye bağlı değerler için
  (ilerleme genişliği, halka boyutu) kullanılır; geri kalan her şey sınıf + token'dır.

## Dil

Arayüz Türkçe ve İngilizcedir. Metinler `src/i18n/tr.ts` ve `src/i18n/en.ts` sözlüklerindedir;
bileşenlerde sabit metin yazılmaz. Türkçe metinler kısa, doğal ve saygılı "siz" dilindedir
("seçin", "kullanıyorsunuz"); çeviri kokan uzun cümlelerden ve "-mektedir", "-dır" gibi resmî
eklerden kaçınılır. Motorun ürettiği kural açıklamaları her iki
dilde gelir (`{ tr, en }`), böylece dil değişince yeniden taramaya gerek kalmaz.

## Platformlar

| | macOS | Windows | Linux (sonra) |
|---|---|---|---|
| Başlık çubuğu | Gömülü, trafik ışıkları sidebar üstünde | Yerel | Yerel |
| Sidebar zemin | Saydam + titreşim | Düz `--surface-2` | Düz `--surface-2` |
| Yazı tipi | SF Pro | Segoe UI Variable | system-ui |
| Kısayollar | ⌘ | Ctrl | Ctrl |

Platform `<html data-platform="macos|windows|linux">` ile CSS'e bildirilir; platforma özel
stiller yalnızca bu seçiciyle yazılır.
