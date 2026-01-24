# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is the documentation website for Alchemy, built with React 19, Vite 7, and Tailwind CSS v4. It processes markdown documentation files into a structured JSON format for client-side rendering.

## Commands

```bash
bun install          # Install dependencies
bun run dev          # Start development server
bun run build        # Build docs manifest + production build
bun run docs:build   # Process markdown files from /docs into JSON
bun run lint         # Run ESLint
bun run preview      # Preview production build
```

## Architecture

### Documentation Pipeline

1. **Source**: Markdown files in `../docs/` (parent directory)
2. **Build script**: `scripts/build-docs.ts` processes markdown with frontmatter
3. **Output**: `public/docs/manifest.json` + `public/docs/pages/{category}/{slug}.json`
4. **Runtime**: React components fetch and render the JSON

Directory structure in `/docs` maps to navigation:
- Top-level directories become categories
- Subdirectories become subcategories
- Files prefixed with numbers (e.g., `01-intro.md`) are ordered accordingly

### Component Structure

```
src/
  App.tsx                     # Router setup: /, /docs, /docs/*
  main.tsx                    # Entry point
  lib/
    types.ts                  # DocPage, DocCategory, DocsManifest types
    cn.ts                     # classnames utility
  pages/
    Home.tsx                  # Landing page
    DocsLayout.tsx            # Docs wrapper with sidebar
    DocsIndex.tsx             # Docs landing (/docs)
    DocsPage.tsx              # Individual doc page (/docs/*)
  components/
    Header.tsx                # Site header
    DocsSidebar.tsx           # Navigation sidebar
    MarkdownContent.tsx       # Renders markdown with Shiki syntax highlighting
    ThemeToggle.tsx           # Light/dark mode toggle
```

### Styling

- Tailwind CSS v4 with custom OKLCH color variables in `src/index.css`
- Shiki for syntax highlighting (vitesse-light/vitesse-black themes)
- Theme toggle persists to localStorage, respects system preference on first visit

## Required Frontmatter

All markdown files in `/docs` must include:

```yaml
---
summary: "One-line description"
read_when:
  - When this doc is relevant
---
```
