export const appChrome = {
  sidebarWidth: 244,
  minContentWidth: 736,
  headerHeight: 56,
} as const;

export const themePreferenceValues = ["system", "light", "dark"] as const;

export type ThemePreference = (typeof themePreferenceValues)[number];

export type ResolvedTheme = Exclude<ThemePreference, "system">;

export const viewportClassName =
  "min-h-screen min-w-[980px] bg-canvas text-ink antialiased";
