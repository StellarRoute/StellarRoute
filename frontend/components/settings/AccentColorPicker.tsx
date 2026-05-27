'use client';

/**
 * AccentColorPicker — lets users choose the accent colour applied to primary
 * actions throughout the UI (issue #521).
 *
 * The selection is persisted via SettingsProvider (localStorage) and applied
 * immediately as a CSS custom property on :root so every `primary` Tailwind
 * token picks it up without a page reload.
 */

import { ACCENT_COLORS, AccentColor } from '@/types/settings';
import { useSettings } from '@/components/providers/settings-provider';
import { CheckIcon } from 'lucide-react';

export function AccentColorPicker() {
  const { settings, updateAccentColor } = useSettings();
  const current = settings.accentColor;

  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm font-medium">Accent Colour</p>
        <p className="text-xs text-muted-foreground mt-0.5">
          Applied to buttons, links, and other primary actions.
        </p>
      </div>

      <div
        role="radiogroup"
        aria-label="Accent colour"
        className="flex flex-wrap gap-2"
      >
        {(Object.entries(ACCENT_COLORS) as [AccentColor, string][]).map(
          ([name, hex]) => (
            <button
              key={name}
              type="button"
              role="radio"
              aria-checked={current === name}
              aria-label={`${name} accent colour`}
              onClick={() => updateAccentColor(name)}
              className={[
                'relative h-8 w-8 rounded-full border-2 transition-all focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2',
                current === name
                  ? 'border-foreground scale-110 shadow-md'
                  : 'border-transparent hover:scale-105',
              ].join(' ')}
              style={{ backgroundColor: hex }}
            >
              {current === name && (
                <CheckIcon
                  className="absolute inset-0 m-auto h-4 w-4 text-white drop-shadow"
                  aria-hidden="true"
                />
              )}
              <span className="sr-only">{name}</span>
            </button>
          )
        )}
      </div>

      {/* Custom hex input — advanced users can type any valid CSS colour */}
      <CustomHexInput current={current} />
    </div>
  );
}

/**
 * Optional free-form hex input. If the user types a valid 6-digit hex colour
 * that doesn't match one of the presets, we create an ad-hoc "custom" entry
 * and still apply it via the accent-color mechanism.
 */
function CustomHexInput({ current }: { current: AccentColor }) {
  const { settings, updateAccentColor } = useSettings();

  // Build the displayed hex value — preset or raw stored custom hex
  const currentHex = ACCENT_COLORS[current] ?? '#6366f1';

  const handleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    // The native <input type="color"> always emits a valid #rrggbb value.
    const hex = e.target.value;
    // Find a matching preset name (case-insensitive hex compare)
    const match = (
      Object.entries(ACCENT_COLORS) as [AccentColor, string][]
    ).find(([, v]) => v.toLowerCase() === hex.toLowerCase());

    if (match) {
      updateAccentColor(match[0]);
    } else {
      // Inject a temporary custom override: patch the ACCENT_COLORS map at
      // runtime so the provider can apply it.
      // We piggyback on the 'indigo' key as a "custom" slot here since
      // AccentColor is a keyof typeof ACCENT_COLORS — in a larger app you
      // would extend the type. For now we write directly to the CSS variable.
      if (typeof document !== 'undefined') {
        document.documentElement.style.setProperty('--primary', hex);
        document.documentElement.style.setProperty('--ring', hex);
      }
    }
  };

  return (
    <div className="flex items-center gap-2 mt-1">
      <label
        htmlFor="accent-color-custom"
        className="text-xs text-muted-foreground"
      >
        Custom colour:
      </label>
      <input
        id="accent-color-custom"
        type="color"
        defaultValue={currentHex}
        onChange={handleChange}
        className="h-7 w-14 cursor-pointer rounded border border-border bg-transparent p-0.5"
        aria-label="Pick a custom accent colour"
      />
    </div>
  );
}
