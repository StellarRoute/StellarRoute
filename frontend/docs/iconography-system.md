# Iconography System

This document defines the iconography rules for StellarRoute's web UI.

## Venue Types

- **SDEX**: Order-book based trading venues.
- **AMM**: Automated market maker liquidity pools.
- **Hybrid**: Routes that combine both SDEX and AMM hops.

### Venue icon rules

- Use a dedicated badge icon for each venue type.
- Keep icon sizes consistent with badges and small route labels: **16px** for badge labels, **20px** for summary icons.
- Use distinct color families for clarity: blue for SDEX, purple for AMM, emerald for Hybrid.

## Transaction States

The transaction lifecycle states are represented by icons and badges:

- **Pending**: Clock icon, awaiting user signature.
- **Submitted**: Spinner icon, transaction submitted to the network.
- **Confirmed**: Checkmark icon, successful confirmation.
- **Failed**: X icon, execution failure.
- **Dropped**: Alert icon, transaction timed out or was not included.

### State icon rules

- Use icons with stroke weights appropriate for 16/20/24px sizes.
- Keep state badges compact and readable with `badge` patterns.
- Use meaningful color semantics: green for success, red for failure, neutral for dropped, and secondary for in-flight states.

## Asset Icons

- Asset icons should display the asset symbol when an image URL is not available.
- Fallback strategy:
  - If the asset icon image fails to load, show a stable uppercase fallback label.
  - Use the first two characters of the asset code by default.
  - For longer symbols, truncate to the first character or one symbol when necessary.

## Implementation Notes

- Align icon usage with `lucide-react` patterns and existing `AssetIcon` fallbacks.
- Prefer explicit `16px`, `20px`, and `24px` sizing in badge and route display components.
- Document any additional iconography changes in this file.
