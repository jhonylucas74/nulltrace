import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import commonEn from "./en/common.json";
import appsEn from "./en/apps.json";
import emailEn from "./en/email.json";
import notificationsEn from "./en/notifications.json";
import settingsEn from "./en/settings.json";
import terminalEn from "./en/terminal.json";
import loginEn from "./en/login.json";
import codeEn from "./en/code.json";
import sysinfoEn from "./en/sysinfo.json";
import procspyEn from "./en/procspy.json";
import systemmonitorEn from "./en/systemmonitor.json";
import diskmanagerEn from "./en/diskmanager.json";
import startupEn from "./en/startup.json";
import backgroundEn from "./en/background.json";
import themeEn from "./en/theme.json";

import commonPtBr from "./pt-br/common.json";
import appsPtBr from "./pt-br/apps.json";
import emailPtBr from "./pt-br/email.json";
import notificationsPtBr from "./pt-br/notifications.json";
import settingsPtBr from "./pt-br/settings.json";
import terminalPtBr from "./pt-br/terminal.json";
import loginPtBr from "./pt-br/login.json";
import codePtBr from "./pt-br/code.json";
import sysinfoPtBr from "./pt-br/sysinfo.json";
import procspyPtBr from "./pt-br/procspy.json";
import systemmonitorPtBr from "./pt-br/systemmonitor.json";
import diskmanagerPtBr from "./pt-br/diskmanager.json";
import startupPtBr from "./pt-br/startup.json";
import backgroundPtBr from "./pt-br/background.json";
import themePtBr from "./pt-br/theme.json";

const LOCALE_STORAGE_KEY = "nulltrace-locale";
const SUPPORTED_LOCALES = ["en", "pt-br"] as const;

function getInitialLocale(): string {
  if (typeof window === "undefined") return "en";
  const stored = localStorage.getItem(LOCALE_STORAGE_KEY);
  if (stored && SUPPORTED_LOCALES.includes(stored as (typeof SUPPORTED_LOCALES)[number])) {
    return stored;
  }
  return "en";
}

export function getLocaleStorageKey(): string {
  return LOCALE_STORAGE_KEY;
}

export function setStoredLocale(lng: string): void {
  if (typeof window === "undefined") return;
  localStorage.setItem(LOCALE_STORAGE_KEY, lng);
}

i18n.use(initReactI18next).init({
  lng: getInitialLocale(),
  fallbackLng: "en",
  supportedLngs: ["en", "pt-br"],
  lowerCaseLng: true,
  ns: ["common", "apps", "email", "notifications", "settings", "terminal", "login", "code", "sysinfo", "procspy", "systemmonitor", "diskmanager", "startup", "background", "theme"],
  defaultNS: "common",
  resources: {
    en: {
      common: commonEn,
      apps: appsEn,
      email: emailEn,
      notifications: notificationsEn,
      settings: settingsEn,
      terminal: terminalEn,
      login: loginEn,
      code: codeEn,
      sysinfo: sysinfoEn,
      procspy: procspyEn,
      systemmonitor: systemmonitorEn,
      diskmanager: diskmanagerEn,
      startup: startupEn,
      background: backgroundEn,
      theme: themeEn,
    },
    "pt-br": {
      common: commonPtBr,
      apps: appsPtBr,
      email: emailPtBr,
      notifications: notificationsPtBr,
      settings: settingsPtBr,
      terminal: terminalPtBr,
      login: loginPtBr,
      code: codePtBr,
      sysinfo: sysinfoPtBr,
      procspy: procspyPtBr,
      systemmonitor: systemmonitorPtBr,
      diskmanager: diskmanagerPtBr,
      startup: startupPtBr,
      background: backgroundPtBr,
      theme: themePtBr,
    },
  },
  interpolation: {
    escapeValue: false,
  },
  initImmediate: false,
});

export default i18n;
