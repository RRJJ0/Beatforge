# UI design workspace

The source of truth for the first desktop wireframe is:

- `osu-ai-desktop-wireframes.drawio`

The file contains 14 pages covering information architecture, first-run setup, generation, job states, results, history, models, settings, dialogs, notifications, and the component reference.

## Visual direction

- Desktop canvas: 1440 × 900
- Navigation: persistent left sidebar
- Default appearance: dark
- Primary action color: osu!-inspired pink `#FF66AA`
- Surfaces: restrained navy/slate layers with subtle borders and elevation
- Base spacing grid: 8 px
- UI implementation target: Vue 3 + Naive UI

### Product-style rules

- Use a persistent application header and workspace navigation to make the tool feel like a finished desktop product.
- Reserve pink for the selected navigation marker, primary CTA, active progress, and focused controls.
- Keep ordinary cards neutral; hierarchy comes from spacing, typography, border contrast, and elevation.
- Use uppercase 11–12 px section labels for technical groups such as system health and generation summary.
- Place advanced or diagnostic information in secondary panels instead of mixing it into the primary workflow.
- Show a review summary before execution and a structured quality report after generation.
- Prefer compact status badges and explicit stage labels over decorative illustrations.

## Review checklist

- A user can move from first launch to a generated `.osz` without using a terminal.
- Missing runtime, missing model, unsupported GPU, low VRAM, cancellation, failure, and success states are visible.
- Advanced model parameters do not overwhelm the default generation form.
- Long-running operations always expose their current state and a safe exit or recovery action.
- Destructive actions distinguish removing a database record from deleting generated files.
- Native filesystem selection will be implemented through Tauri rather than browser upload controls.

## Export convention

When exporting reviewed pages, place SVG or PNG files in `docs/ui/exports/` and use the page name with a numeric prefix, for example:

```text
01-information-architecture.svg
04-generate-audio-selected.png
09-generation-result.png
```
