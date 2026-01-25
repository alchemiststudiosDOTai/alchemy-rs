# Alchemy Website

Documentation and landing page for Alchemy, built with React, Vite, and Tailwind CSS v4.

## Development

```bash
bun install
bun run dev
```

## Build

```bash
bun run build
```

This runs `docs:build` first to generate the documentation manifest, then builds the site.

## Scripts

| Script       | Description                                   |
| ------------ | --------------------------------------------- |
| `dev`        | Start development server                      |
| `build`      | Build docs and site for production            |
| `docs:build` | Process markdown files from `/docs` into JSON |
| `preview`    | Preview production build locally              |
| `lint`       | Run ESLint                                    |

## Documentation System

Markdown files in `/docs` are processed into a structured JSON format for the website.

### Directory Structure

```
docs/
  api/
    error.md
    lib.md
  utils/
    transform.md
```

Directories become categories, subdirectories become subcategories.

### Frontmatter

Each markdown file should include YAML frontmatter:

```yaml
---
summary: "Brief description of this page"
read_when:
  - Condition when this doc is useful
  - Another relevant scenario
---
# Page Title

Content...
```

### Generated Output

The `docs:build` script outputs to `public/docs/`:

- `manifest.json` - Category/page hierarchy for navigation
- `pages/{category}/{slug}.json` - Individual page content

### Adding Documentation

1. Create a markdown file in `/docs/{category}/`
2. Add frontmatter with `summary` and `read_when`
3. Run `bun run docs:build` or `bun run build`

## Tech Stack

- **React 19** - UI framework
- **Vite 7** - Build tool
- **Tailwind CSS v4** - Styling with custom theme
- **Shiki** - Syntax highlighting (vitesse-light/vitesse-black themes)
- **Marked** - Markdown parsing
- **Lucide React** - Icons
- **React Router** - Client-side routing

## Theme

Custom black and white theme with light/dark mode support. Theme toggle persists preference to localStorage and respects system preference on first visit.

Colors are defined as CSS variables in `src/index.css` using OKLCH color space.
