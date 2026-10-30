use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Lang {
    #[default]
    Tr,
    En,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(lang: Lang) {
    CURRENT.store(lang as u8, Ordering::Relaxed);
}

pub fn lang() -> Lang {
    if CURRENT.load(Ordering::Relaxed) == Lang::En as u8 {
        Lang::En
    } else {
        Lang::Tr
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Text {
    pub tr: &'static str,
    pub en: &'static str,
}

impl Text {
    pub const fn new(tr: &'static str, en: &'static str) -> Self {
        Self { tr, en }
    }

    pub const fn same(both: &'static str) -> Self {
        Self { tr: both, en: both }
    }

    pub fn get(&self, lang: Lang) -> &'static str {
        match lang {
            Lang::Tr => self.tr,
            Lang::En => self.en,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Localized {
    pub tr: String,
    pub en: String,
}

impl Localized {
    pub fn same(both: impl Into<String>) -> Self {
        let both = both.into();
        Self { tr: both.clone(), en: both }
    }

    pub fn get(&self, lang: Lang) -> &str {
        match lang {
            Lang::Tr => &self.tr,
            Lang::En => &self.en,
        }
    }
}

impl From<Text> for Localized {
    fn from(t: Text) -> Self {
        Self {
            tr: t.tr.to_string(),
            en: t.en.to_string(),
        }
    }
}

#[macro_export]
macro_rules! t {
    ($tr:literal, $en:literal $(,)?) => {
        match $crate::i18n::lang() {
            $crate::i18n::Lang::Tr => format!($tr),
            $crate::i18n::Lang::En => format!($en),
        }
    };
}

pub fn io_error(e: &std::io::Error) -> String {
    use std::io::ErrorKind::*;
    if cfg!(windows) && matches!(e.raw_os_error(), Some(32 | 33)) {
        return Text::new("dosya başka bir program tarafından kullanılıyor", "the file is in use by another program").get(lang()).to_string();
    }
    let text = match e.kind() {
        NotFound => Text::new("dosya ya da klasör bulunamadı", "file or folder not found"),
        PermissionDenied => Text::new("erişim izni yok", "permission denied"),
        AlreadyExists => Text::new("zaten var", "already exists"),
        ReadOnlyFilesystem => Text::new("disk salt okunur", "the disk is read-only"),
        StorageFull => Text::new("diskte yer kalmadı", "the disk is full"),
        ResourceBusy => Text::new("dosya başka bir program tarafından kullanılıyor", "the file is in use by another program"),
        InvalidFilename => Text::new("geçersiz dosya adı", "invalid file name"),
        _ => return e.to_string(),
    };
    text.get(lang()).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_language() {
        let n = 3;
        assert_eq!(lang(), Lang::Tr);
        assert_eq!(t!("{n} dosya", "{n} files"), "3 dosya");
        set_lang(Lang::En);
        assert_eq!(t!("{n} dosya", "{n} files"), "3 files");
        set_lang(Lang::Tr);
        assert_eq!(Text::new("a", "b").get(Lang::En), "b");
        assert_eq!(Localized::from(Text::same("x")).get(Lang::Tr), "x");
    }
}

