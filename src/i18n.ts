import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import HttpApi from "i18next-http-backend";

i18n
  .use(HttpApi) // load translations using http (default public/locales)
  .use(LanguageDetector) // detect user language
  .use(initReactI18next) // pass the i18n instance to react-i18next.
  .init({
    supportedLngs: ["en", "zh-CN", "zh-TW", "ja"],
    fallbackLng: "en",
    defaultNS: "common",
    // path where resources get loaded from
    backend: {
      loadPath: "/locales/{{lng}}/{{ns}}.json",
    },
    // detection order
    detection: {
      order: ["localStorage", "navigator", "htmlTag"],
      caches: ["localStorage"],
    },
    interpolation: {
      escapeValue: false, // react already safes from xss
    },
  });

export default i18n;
