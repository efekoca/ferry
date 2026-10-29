import { createContext, useCallback, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { en } from "./en";
import { tr, type Dict } from "./tr";

export type Lang = "tr" | "en";
export type { Dict };

export interface Localized {
  tr: string;
  en: string;
}

export const DICTS: Record<Lang, Dict> = { tr, en };
export const LANGS: { value: Lang; label: string }[] = [
  { value: "tr", label: "Türkçe" },
  { value: "en", label: "English" },
];

const KEY = "ferry.lang";

export function detectLang(): Lang {
  try {
    const stored = JSON.parse(localStorage.getItem(KEY) ?? "null");
    if (stored === "tr" || stored === "en") return stored;
  } catch {}
  const system = typeof navigator === "undefined" ? "" : navigator.language;
  return system.toLowerCase().startsWith("tr") ? "tr" : "en";
}

interface I18n {
  lang: Lang;
  setLang(lang: Lang): void;
  t: Dict;
  loc(text: Localized): string;
}

const I18nContext = createContext<I18n>({ lang: "tr", setLang: () => {}, t: tr, loc: (x) => x.tr });

export function I18nProvider({ children }: { children: ReactNode }) {
  const [lang, setLangState] = useState<Lang>(detectLang);

  useEffect(() => {
    document.documentElement.lang = lang;
  }, [lang]);

  const setLang = useCallback((next: Lang) => {
    setLangState(next);
    try {
      localStorage.setItem(KEY, JSON.stringify(next));
    } catch {}
  }, []);

  const value = useMemo<I18n>(() => ({ lang, setLang, t: DICTS[lang], loc: (x) => x[lang] }), [lang, setLang]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export const useI18n = () => useContext(I18nContext);
