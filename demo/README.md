# Ferry demo projeleri

| Klasör | Framework | Ne gösterir |
|---|---|---|
| `kuzey-magaza` | Düz PHP | Ferry'nin bildiği her kural en az bir kez geçer; uyumsuz `composer.json` kısıtı ve karışık `composer.lock` |
| `wp-blog` | WordPress | Tema ve eklentilerde birkaç sorun; `wp-admin` ve `wp-includes` taranmaz |
| `laravel-api` | Laravel | Neredeyse hazır, modern bir proje; `storage` taranmaz |

Demoları her zaman temiz bir kopya üzerinde deneyin:

```bash
pnpm demo
```

Bu komut projeleri `.demo/` klasörüne sıfırdan kopyalar ve yollarını yazar. Ferry'de bu yolları açın;
değişiklik uyguladıktan sonra tekrar çalıştırarak başa dönebilirsiniz.
