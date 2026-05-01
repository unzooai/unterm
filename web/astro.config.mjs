import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";

// Locales served at:
//   /              -> en-US (default, no prefix)
//   /zh-CN/        -> Simplified Chinese
//   /zh-TW/        -> Traditional Chinese
//   /ja-JP/        -> Japanese
//   /ko-KR/        -> Korean
//   /de-DE/        -> German
//   /fr-FR/        -> French
//   /it-IT/        -> Italian
//   /hi-IN/        -> Hindi (Devanagari)
export default defineConfig({
  site: "https://unterm.app",
  integrations: [tailwind()],
  i18n: {
    defaultLocale: "en-US",
    locales: [
      "en-US",
      "zh-CN",
      "zh-TW",
      "ja-JP",
      "ko-KR",
      "de-DE",
      "fr-FR",
      "it-IT",
      "hi-IN",
    ],
    routing: {
      prefixDefaultLocale: false,
    },
  },
});
