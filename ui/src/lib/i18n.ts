import ru from '../locales/ru.json';
import en from '../locales/en.json';

const dictionaries: Record<string, Record<string, string>> = { ru, en };
let currentLang = 'ru'; // По умолчанию русский

// Тип ошибки, соответствующий Rust-структуре
export type ApiError = {
    key: string;
    code: string;
    severity: string;
    args: string[];
};

/**
 * Перевод ключа с подстановкой аргументов {0}, {1} и т.д.
 */
export function t(key: string, args: string[] = []): string {
    const dict = dictionaries[currentLang] || dictionaries['en'];
    let template = dict[key];

    // Если перевода нет — возвращаем сам ключ (fallback)
    if (!template) return key;

    // Подставляем аргументы
    args.forEach((arg, i) => {
        template = template.replace(`{${i}}`, arg);
    });

    return template;
}

/**
 * Центральная функция перевода ошибок согласно error-handling.md
 */
export function translateError(err: ApiError): string {
    const translated = t(err.key, err.args);
    // Если ключ не найден в словаре, t() вернет сам ключ
    if (translated !== err.key) return translated;

    return t('unknown_error');
}