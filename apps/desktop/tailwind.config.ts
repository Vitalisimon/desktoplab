import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        canvas: "rgb(var(--dl-color-canvas) / <alpha-value>)",
        panel: "rgb(var(--dl-color-panel) / <alpha-value>)",
        elevated: "rgb(var(--dl-color-elevated) / <alpha-value>)",
        line: "rgb(var(--dl-color-line) / <alpha-value>)",
        ink: "rgb(var(--dl-color-ink) / <alpha-value>)",
        muted: "rgb(var(--dl-color-muted) / <alpha-value>)",
        accent: "rgb(var(--dl-color-accent) / <alpha-value>)",
        focus: "rgb(var(--dl-color-focus) / <alpha-value>)",
        success: "rgb(var(--dl-color-success) / <alpha-value>)",
        warning: "rgb(var(--dl-color-warning) / <alpha-value>)",
        danger: "rgb(var(--dl-color-danger) / <alpha-value>)",
        local: "rgb(var(--dl-color-local) / <alpha-value>)",
        provider: "rgb(var(--dl-color-provider) / <alpha-value>)",
        overlay: "rgb(var(--dl-color-overlay) / <alpha-value>)",
      },
      borderRadius: {
        desktop: "8px",
      },
      boxShadow: {
        panel: "var(--dl-shadow-panel)",
      },
      fontFamily: {
        sans: [
          "Inter",
          "ui-sans-serif",
          "system-ui",
          "-apple-system",
          "BlinkMacSystemFont",
          "Segoe UI",
          "sans-serif",
        ],
      },
    },
  },
  plugins: [],
} satisfies Config;
