// SPDX-License-Identifier: MIT

import en from "./locales/en.js";
import fr from "./locales/fr.js";

const LOCALES = { en, fr };
const FALLBACK = "en";

function detectLocale() {
  const candidates = navigator.languages?.length ? navigator.languages : [navigator.language];
  for (const lang of candidates) {
    const primary = lang?.split("-")[0]?.toLowerCase();
    if (primary && LOCALES[primary]) return primary;
  }
  return FALLBACK;
}

export const locale = detectLocale();

function lookup(dict, key) {
  return key.split(".").reduce((acc, part) => acc?.[part], dict);
}

export function t(key, vars) {
  const raw = lookup(LOCALES[locale], key) ?? lookup(LOCALES[FALLBACK], key) ?? key;
  if (!vars) return raw;
  return Object.entries(vars).reduce((str, [k, v]) => str.replaceAll(`{${k}}`, v), raw);
}

export function applyStaticTranslations() {
  document.documentElement.lang = locale;
  document.querySelectorAll("[data-i18n]").forEach((elm) => {
    elm.textContent = t(elm.dataset.i18n);
  });
}
