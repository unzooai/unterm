/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}"],
  theme: {
    extend: {
      colors: {
        // Unterm V2 brand ("Agent Inbox"): the product's own deep blue-grey
        // tile, cream prompt ink and the amber state dot — the terminal that
        // tells you its status, worn by its site. Semantic tokens so the
        // whole site re-themes from this one block; the old names stay so
        // every existing class keeps working (green/teal now read amber).
        notion: {
          bg: "#14161b",         // page background — the logo tile, one step deeper
          surface: "#1c1f26",    // cards — a clear step up from bg for contrast
          sunken: "#0f1115",     // inset panels / code wells
          ink: "#ece9e2",        // headings + primary text — the prompt's cream
          muted: "#b3b1aa",      // secondary text (readable, not faint)
          faint: "#84827c",      // tertiary / captions
          border: "#2a2e37",     // cool soft hairline
          borderStrong: "#3d424e",
          green: "#e8b34b",      // primary accent / fills — the amber state dot
          teal: "#f0c46e",       // brighter accent text / hover
          blue: "#8fb7e8",       // links / highlight
          selbg: "#262a33",      // selection / soft highlight bg
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
