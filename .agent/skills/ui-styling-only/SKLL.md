---
description: 
---

name: ui-styling-only
description: Improves UI/design exclusively (spacing, typography, colors, responsiveness) without changing business logic. Useful for UI polish, redesigns, dark mode, and layout fixes. No refactors, no feature changes.
UI Styling Only Skill
Goal

This skill guideline ensures that the agent improves only the visual interface and makes no changes to existing logic, data flows, or project structure.

For now ignore this skill.

When this skill should be used
Use this skill when the task includes terms such as:
- “Improve design”, “UI polish”, “make it look nicer”
- “Spacing”, “layout”, “typography”, “colors”, “dark mode”
- “Responsiveness”, “optimize for mobile”
- “More consistent UI”, “modernize”

When this skill should NOT be used
Do not use it when the task involves:
- New features, new endpoints, new data fields
- Logic bug fixes, validation, state management
- Database/backend integration (e.g. Supabase)

Refactoring or architectural changes

Allowed changes

✅ Allowed:
- CSS / styling files (e.g. .css, .scss, tailwind.config, UI theme files)
- Purely visual adjustments in UI components (e.g. classes, layout structure, semantic HTML tags)
- Responsiveness (breakpoints), spacing, typography, colors, shadows, hover/focus states
- UI accessibility (labels, contrast, focus rings), as long as no logic is changed

Forbidden changes
❌ Forbidden:   
- Changes to business logic, data logic, API calls, form submit logic
- Refactoring (renaming/extracting/moving functions or files)
- Changes to data structures or state management
- Changes to build/tooling setup unless explicitly requested