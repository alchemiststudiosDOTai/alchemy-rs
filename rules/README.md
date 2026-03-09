# ast-grep rules

Docs frontmatter enforcement for markdown files under `docs/`.

## Enforced pattern

Every docs markdown file must start with this exact frontmatter shape:

```yaml
---
summary: "..."
status: current
last_verified: 2026-03-09
---
```

Allowed `status` values:

- `current`
- `draft`
- `needs review`
- `generated`

## Run scan

From repository root:

```bash
sg scan --config rules/sgconfig.yml docs --error
```

## Notes

- ast-grep does not natively parse markdown, so `rules/sgconfig.yml` maps `**/*.md` to the `html` parser and the rule matches the whole document node with a regex.
- The rule lives in `rules/docs/docs-frontmatter.yml`.
