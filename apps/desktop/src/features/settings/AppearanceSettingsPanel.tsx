import { Monitor, Moon, Sun } from "../../design/icons";
import { useThemePreference } from "../../app/theme";
import { themePreferenceValues, type ThemePreference } from "../../design/tokens";

const labels: Record<ThemePreference, string> = {
  system: "System",
  light: "Light",
  dark: "Dark",
};

const descriptions: Record<ThemePreference, string> = {
  system: "Follow system settings.",
  light: "Use a bright workspace.",
  dark: "Use a low-light workspace.",
};

const icons = {
  system: Monitor,
  light: Sun,
  dark: Moon,
};

export function AppearanceSettingsPanel() {
  const { preference, resolvedTheme, setPreference } = useThemePreference();

  return (
    <section aria-labelledby="appearance-title" className="border-b border-line py-4">
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h2 className="text-lg font-semibold" id="appearance-title">
            Appearance
          </h2>
          <p className="mt-1 text-sm leading-6 text-muted">Choose how DesktopLab looks on this device.</p>
        </div>
        <span className="rounded px-2 py-1 text-xs font-semibold capitalize text-muted">Using {resolvedTheme}</span>
      </div>

      <div aria-label="Theme" className="mt-4 grid gap-2 sm:grid-cols-3" role="radiogroup">
        {themePreferenceValues.map((value) => {
          const Icon = icons[value];
          return (
            <label
              className="flex cursor-pointer items-center gap-3 rounded-desktop border border-line bg-elevated px-3 py-3 text-sm transition hover:border-focus"
              data-selected={preference === value ? "true" : "false"}
              key={value}
            >
              <input
                aria-label={labels[value]}
                checked={preference === value}
                className="size-4 accent-current"
                name="theme-preference"
                onChange={() => setPreference(value)}
                type="radio"
              />
              <Icon aria-hidden="true" className="size-4 text-muted" />
              <span>
                <span className="block font-medium">{labels[value]}</span>
                <span className="block text-xs text-muted">{descriptions[value]}</span>
              </span>
            </label>
          );
        })}
      </div>
    </section>
  );
}
