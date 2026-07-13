# AGENTS.md

## Scope

These rules apply to project-scoped Codex configuration and custom agents under `.codex/`.

## Instruction Ownership

- Root `AGENTS.md` owns repository invariants, source-of-truth locations, delegation order, and the cross-surface sync matrix.
- Nested `AGENTS.md` files own surface-specific implementation, documentation, adapter, and validation rules.
- `.codex/config.toml` owns project-specific subagent runtime limits.
- `.codex/agents/*.toml` own only role-specific scope, evidence, output, and safety behavior.
- Do not copy changing repository contracts or mirror-file lists into every agent. Require agents to read root and applicable nested instructions at runtime.

## Role Boundaries

- `shosei-explorer`: pre-implementation context mapping only.
- `shosei-sync-auditor`: exhaustive audit of mirrors required by applicable `AGENTS.md` rules.
- `shosei-reviewer`: correctness, regression, spec alignment, portability, and validation review for a diff or explicitly bounded current-state file set.
- For cross-surface changes, run the sync auditor before the reviewer and pass the audit result forward so the reviewer does not repeat the mirror scan.
- Keep implementation work with the parent agent or the built-in worker; do not add a broad custom implementation agent without a recurring, narrower need.

## Authoring Rules

- Every custom agent must have non-empty `name`, `description`, and `developer_instructions` fields.
- Keep the filename stem equal to `name`, names unique, and nickname candidates non-empty, unique, and presentation-only.
- Keep exploration, audit, and review agents in `read-only` sandbox mode.
- Read-only agents must also prohibit remote mutations and write-capable app or MCP actions; filesystem sandboxing alone is not the complete side-effect boundary.
- Do not add agent-specific MCP servers, skills, or model pins unless the role demonstrably needs them.
- A role description must state both when to use the agent and the adjacent work it does not own.
- Runtime instructions must define the required context packet, scope fallback, evidence standard, success signal, stop conditions, and retry limit.
- Make "no target" distinct from "no findings" for review and audit roles.
- Have agents answer in the parent agent's language while preserving stable section semantics.

## Scope Fallback

For review and sync audit roles when the parent supplies no explicit files or range:

1. Inspect `git status --short`.
2. Inspect unstaged and staged diffs separately.
3. Include relevant untracked files by reading them directly.
4. If the tree is clean, report that there is no target and stop; do not silently broaden into a repository-wide audit.
5. Do not infer a base branch or commit range.

## Validation

After changing `.codex`:

1. Parse every changed TOML file.
2. Check required fields, unique names, filename/name alignment, nickname constraints, and read-only sandbox settings.
3. Run `git diff --check`.
4. Confirm root `AGENTS.md` names and delegation order match the custom files.
5. Smoke-test each role with a bounded prompt, including staged/untracked and clean-tree cases for review/audit.
6. Confirm read-only agents made no filesystem or remote changes.

Prompt-only `.codex` changes do not require Cargo or npm validation unless they also touch product code or product-facing documentation.
