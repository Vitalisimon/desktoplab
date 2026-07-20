import { createContext, useContext, useEffect, useMemo, useState, type ReactNode } from "react";
import { themePreferenceValues, type ResolvedTheme, type ThemePreference } from "../design/tokens";

const storageKey = "desktoplab.themePreference";

type ThemeContextValue = {
  preference: ThemePreference;
  resolvedTheme: ResolvedTheme;
  setPreference: (preference: ThemePreference) => void;
};

const ThemeContext = createContext<ThemeContextValue | null>(null);

export function ThemeProvider({ children }: { children: ReactNode }) {
  const [preference, setPreferenceState] = useState<ThemePreference>(readStoredPreference);
  const [systemTheme, setSystemTheme] = useState<ResolvedTheme>(readSystemTheme);
  const resolvedTheme = preference === "system" ? systemTheme : preference;

  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    if (!media) return undefined;
    const listener = () => setSystemTheme(media.matches ? "dark" : "light");
    media.addEventListener?.("change", listener);
    return () => media.removeEventListener?.("change", listener);
  }, []);

  useEffect(() => {
    document.documentElement.dataset.themePreference = preference;
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [preference, resolvedTheme]);

  const value = useMemo<ThemeContextValue>(
    () => ({
      preference,
      resolvedTheme,
      setPreference: (nextPreference) => {
        setPreferenceState(nextPreference);
        window.localStorage.setItem(storageKey, nextPreference);
      },
    }),
    [preference, resolvedTheme],
  );

  return <ThemeContext.Provider value={value}>{children}</ThemeContext.Provider>;
}

export function useThemePreference() {
  const context = useContext(ThemeContext);
  if (!context) {
    throw new Error("useThemePreference must be used inside ThemeProvider");
  }
  return context;
}

function readStoredPreference(): ThemePreference {
  const stored = window.localStorage.getItem(storageKey);
  return isThemePreference(stored) ? stored : "system";
}

function readSystemTheme(): ResolvedTheme {
  return window.matchMedia?.("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

function isThemePreference(value: string | null): value is ThemePreference {
  return themePreferenceValues.includes(value as ThemePreference);
}
