import i18n from "i18next";
import { initReactI18next } from "react-i18next";

import commonEn from "./en/common.json";
import appsEn from "./en/apps.json";
import emailEn from "./en/email.json";
import notificationsEn from "./en/notifications.json";
import settingsEn from "./en/settings.json";
import terminalEn from "./en/terminal.json";
import loginEn from "./en/login.json";

i18n.use(initReactI18next).init({
  lng: "en",
  fallbackLng: "en",
  ns: ["common", "apps", "email", "notifications", "settings", "terminal", "login"],
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
    },
  },
  interpolation: {
    escapeValue: false,
  },
  initImmediate: false,
});

export default i18n;
