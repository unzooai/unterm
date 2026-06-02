/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        // Unterm "Notion Dark" scheme (config/src/unterm_schemes.rs), tuned for
        // the web: warm dark (not cold black), bright headings + clearly raised
        // cards for crisp hierarchy, warm soft borders + teal-green accent.
        // Semantic tokens so the whole site re-themes from this one block.
        notion: {
          bg: "#191919",         // warm dark page background (softer than black)
          surface: "#232322",    // cards — a clear step up from bg for contrast
          sunken: "#141413",     // inset panels / code wells
          ink: "#f2f1ee",        // headings + primary text (bright = crisp)
          muted: "#b8b7b1",      // secondary text (readable, not faint)
          faint: "#8a8983",      // tertiary / captions
          border: "#33322f",     // warm soft hairline
          borderStrong: "#494841",
          green: "#4dab9a",      // primary accent / fills (Notion teal-green)
          teal: "#6fccb8",       // brighter accent text / hover
          blue: "#7bb4dd",       // links / highlight
          selbg: "#2c2c2c",      // selection / soft highlight bg
        },
      },
      fontFamily: {
        sans: [
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "Helvetica Neue",
          "Noto Sans",
          "Noto Sans CJK SC",
          "Noto Sans CJK TC",
          "Noto Sans JP",
          "Noto Sans KR",
          "Noto Sans Devanagari",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "Monaco",
          "Consolas",
          "Liberation Mono",
          "Courier New",
          "monospace",
        ],
      },
    },
  },
  plugins: [],
};
