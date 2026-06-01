'use client';

/**
 * FontScaleControl — lets users scale the root font size from 100 % to 200 %
 * in 25 % steps for accessibility (issue #522).
 *
 * The multiplier is stored via SettingsProvider (localStorage) and applied
 * immediately by setting `font-size` on <html>, so every `rem`-based value
 * in the UI (padding, layout, text) scales proportionally without layout
 * breakage.
 */

import { FONT_SCALE_OPTIONS, FontScale } from '@/types/settings';
import { useSettings } from '@/components/providers/settings-provider';

const LABELS: Record<string, string> = {
  '1': '100%',
  '1.25': '125%',
  '1.5': '150%',
  '1.75': '175%',
  '2': '200%',
};

function label(scale: FontScale): string {
  return LABELS[String(scale)] ?? `${Math.round(scale * 100)}%`;
}

export function FontScaleControl() {
  const { settings, updateFontScale } = useSettings();
  const current = settings.fontScale;

  return (
    <div className="space-y-3">
      <div>
        <p className="text-sm font-medium">Text Size</p>
        <p className="text-xs text-muted-foreground mt-0.5">
          Scale the interface font size up to 200 % without breaking the layout.
        </p>
      </div>

      {/* Segmented control */}
      <div
        role="radiogroup"
        aria-label="Font scale"
        className="flex flex-wrap gap-1.5"
      >
        {FONT_SCALE_OPTIONS.map((scale) => (
          <button
            key={scale}
            type="button"
            role="radio"
            aria-checked={current === scale}
            onClick={() => updateFontScale(scale)}
            className={[
              'px-3 py-1 rounded-md text-sm font-medium transition-colors',
              'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-1',
              current === scale
                ? 'bg-primary text-primary-foreground'
                : 'bg-muted text-muted-foreground hover:bg-muted/80',
            ].join(' ')}
          >
            {label(scale)}
          </button>
        ))}
      </div>

      {/* Slider for fine-grained control between preset steps */}
      <div className="space-y-1">
        <input
          id="font-scale-slider"
          type="range"
          min={1}
          max={2}
          step={0.25}
          value={current}
          onChange={(e) =>
            updateFontScale(parseFloat(e.target.value) as FontScale)
          }
          aria-label={`Font scale slider, current value ${label(current)}`}
          aria-valuemin={1}
          aria-valuemax={2}
          aria-valuenow={current}
          aria-valuetext={label(current)}
          className="w-full accent-primary"
        />
        <div className="flex justify-between text-xs text-muted-foreground select-none">
          <span>100%</span>
          <span aria-live="polite" aria-atomic="true">
            {label(current)}
          </span>
          <span>200%</span>
        </div>
      </div>

      {/* Live preview */}
      <div
        aria-label="Font scale preview"
        className="rounded-md border border-border p-3 bg-muted/40"
        style={{ fontSize: `${current}rem` }}
      >
        <p className="font-semibold" style={{ fontSize: '1em' }}>
          Preview — StellarRoute
        </p>
        <p className="text-muted-foreground" style={{ fontSize: '0.875em' }}>
          Swap · Quote · Route · Settings
        </p>
      </div>
    </div>
  );
}
