# Features

One file per feature, slugged, e.g. `spec/features/btc-momentum-v1.md`.

Feature lifecycle:

1. **Analyst** creates the file with `## Why`, `## Requirements`, and
   `## Backtest Scenarios` filled in.
2. **Architect** adds `## Design` and creates a matching `spec/tasks/<slug>.md`.
3. **Developer** works the task list; appends `## Implementation`.
4. **Tester** runs the backtest scenarios, links reports under `## Verification`.

Use the `spec-update` skill — never write these files directly.
