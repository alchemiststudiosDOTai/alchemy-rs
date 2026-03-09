---
summary: "How planning docs are organized"
status: current
last_verified: 2026-03-09
---

# Plans

Use `docs/exec-plans/` for temporary execution detail.

## Layout

- [exec-plans/active/index.md](./exec-plans/active/index.md) - in-progress plans
- [exec-plans/completed/index.md](./exec-plans/completed/index.md) - archived completed plans
- [exec-plans/tech-debt-tracker.md](./exec-plans/tech-debt-tracker.md) - durable debt that is not an active plan yet

## Rules

- Create a plan when work is multi-step or spans multiple modules.
- Move finished plans out of `active/` quickly.
- Promote stable architecture or product truth out of plans and into the relevant reference doc.
