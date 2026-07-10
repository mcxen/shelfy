import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import en from './locales/en.json';
import pl from './locales/pl.json';
import it from './locales/it.json';
import de from './locales/de.json';
import fr from './locales/fr.json';
import ru from './locales/ru.json';
import ja from './locales/ja.json';
import zh from './locales/zh.json';

const resources = {
  en: { translation: en },
  pl: { translation: pl },
  it: { translation: it },
  de: { translation: de },
  fr: { translation: fr },
  ru: { translation: ru },
  ja: { translation: ja },
  zh: { translation: zh },
};

export type SupportedLang = 'en' | 'pl' | 'it' | 'de' | 'fr' | 'ru' | 'ja' | 'zh';

let initPromise: Promise<void> | null = null;

export async function initI18n(lang: SupportedLang) {
  if (!initPromise) {
    initPromise = i18n
      .use(initReactI18next)
      .init({
        resources,
        lng: lang,
        fallbackLng: 'en',
        interpolation: {
          escapeValue: false,
        },
        react: {
          useSuspense: false,
        },
      })
      .then(() => undefined);

    await initPromise;
    return;
  }

  await initPromise;

  if (i18n.language !== lang) {
    await i18n.changeLanguage(lang);
  }
}

void initI18n('en');

export default i18n;
