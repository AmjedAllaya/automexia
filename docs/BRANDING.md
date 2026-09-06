# Automexia brand

## Public product message

Automexia's public promise is:

> A flexible open-source terminal for focused command-line work.

Supporting language may describe current, verified terminal capabilities:

- windows, tabs, panes, and pane-local tabs;
- familiar shells and direct commands;
- search, selection, scrollback, and marked-command navigation;
- images, themes, fonts, shortcuts, and configuration;
- keyboard-first use and visible control;
- explicit security and recovery behavior.

## Voice

Use a simple, precise, and natural voice:

- explain current user value before implementation details;
- say “users” or “people” unless the text is contributor-specific;
- distinguish available, source-complete, release-gated, externally unverified,
  and not implemented;
- use concrete words such as terminal, shell, command, pane, tab, file, output,
  shortcut, and configuration;
- avoid universal claims about platforms, shells, tools, providers, or users;
- state that external tools and credential owners retain their authority;
- describe safety with exact behavior rather than superlatives.

## Public confidentiality boundary

Do not publish or hint at:

- advanced unreleased features;
- future specialist extensions or domain products;
- commercial editions, pricing, packaging, revenue, or market strategy;
- organization or hosted-service designs;
- unreleased provider workflows;
- private algorithms, schemas, state machines, resource limits, UX flows, or
  implementation phases;
- private plan names or links.

Do not use teaser language such as “coming soon,” “planned premium feature,” or
“future platform.” Public pages should simply describe current free
terminal behavior and ordinary open-source maintenance.

## Claims to avoid

Avoid:

- “AI terminal”;
- “one terminal for every environment”;
- “universal command center”;
- “automatic safest route”;
- “complete audit trail”;
- “replaces your shell, provider, credential store, or specialist tool”;
- “production ready” without exact release evidence;
- any price, revenue, customer, compliance, or availability claim that lacks
  published evidence.

## Visual asset workflow

The canonical source image is
`assets/brand/automexia-terminal-source-512.png`. Its provenance and digest are
recorded in `assets/brand/ASSET-MANIFEST.toml`.

On Windows with ImageMagick 7 installed:

```powershell
powershell -NoProfile -File tools/brand/generate-platform-assets.ps1
```

The generator produces the documented Linux PNG sizes, Windows ICO, and macOS
ICNS assets. Run `cargo ready` afterward.

## Stable-release approval

Generated raster assets are development inputs, not automatic stable-release
approval. Stable publication additionally requires the editable sources,
platform variants, redistribution-rights evidence, named review, and the exact
release state required by `ASSET-MANIFEST.toml`.
