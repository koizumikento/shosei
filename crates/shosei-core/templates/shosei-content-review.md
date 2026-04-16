---
name: "shosei-content-review"
description: "{{DESCRIPTION}}"
compatibility: "Requires a repository initialized by `shosei init` and a local `shosei` CLI in the working environment."
---

# Shosei Content Review

Use this skill for reviewing content in this repository instead of implementing edits or rewrites. It is repo-scoped on purpose: keep the review grounded in the initialized project layout and the current `project.type` / `repo_mode`.

## Repo Notes

- This repo was initialized as `{{REPO_MODE}}` with the `{{PROJECT_TYPE}}` template.
- Primary config entrypoint: {{PRIMARY_CONFIG}}
- Primary content paths: {{PRIMARY_CONTENT_PATHS}}
- Optional sidecar paths: {{OPTIONAL_CONTENT_PATHS}}
- Review lens: {{REVIEW_FOCUS}}
- {{REPO_MODE_RULES}}

## Use For

- user requests like "review this chapter", "does this volume read cleanly?", "check the claims in this section", "is the proof packet ready to hand off?", or "sanity-check the page order"
- chapter, manuscript, volume, proof package, or outline reviews
- fiction: scene goal, causality, character knowledge drift, POV / voice drift, pacing, and setup/payoff
- source-backed nonfiction or business-book content: unsupported claims, stale facts, weak structure, and source-to-text mismatch
- release-readiness checks where the proof packet, editorial sidecars, and manuscript should agree
- manga: page-turn flow, spread logic, dialogue order, and metadata/read-order consistency

## Do Not Use For

- rewriting or line-editing the content before review findings are returned
- generic code review or CLI implementation work
- inventing facts, sources, scenes, or canon not already present in the repo
- broad editorial change requests that are really asking for a rewrite

## Workflow

1. Establish the review scope first.
   - whole book, chapter, volume, proof packet, or a specific manuscript section
   - review goal: continuity, factual rigor, structure, release-readiness, or manga flow
2. Inspect the repo shape before judging.
   - Use `{{EXPLAIN_COMMAND}}` when resolved config or scope matters.
   - Read the relevant content paths and any nearby editorial sidecars before commenting.
3. Pull in adjacent review aids when they exist.
   - Use `{{VALIDATE_COMMAND}}` for schema or repository checks that affect the review.
   - {{PAGE_CHECK_COMMAND}}
   - {{REFERENCE_MAP_COMMAND}}
   - Use `{{STORY_CHECK_COMMAND}}` when story or canon sidecars are present.
   - {{REFERENCE_CHECK_COMMAND}}
   - {{REFERENCE_ALIGNMENT_COMMAND}}
4. Review for substantive issues first.
   - confirm whether the text matches the requested `project.type`
   - for source-backed sections, treat the relevant reference entries and editorial claims as the primary review aids before judging wording, structure, or release-readiness
   - call out source-to-text mismatch, unsupported claims, stale support, and conclusions that outrun the available notes
   - in `series`, distinguish book-scoped references from shared references and flag source-of-truth ambiguity when the same topic appears in both scopes
   - compare the content against the relevant sidecars, proofs, and repo conventions
   - separate confirmed defects from editorial suggestions
5. Return findings first.
   - blockers
   - major issues
   - minor issues
   - open questions
   - for each finding: location, why it matters, and the smallest local fix
6. Keep recommendations small and concrete.
   - point to the smallest local correction that would fix the issue
   - avoid broad rewrite advice unless the whole structure is the problem
7. If nothing actionable remains, say so directly.
   - note residual risk, if any
   - do not bury the lead under process notes

## Guardrails

- do not claim a factual error without support from repo content or explicitly supplied sources
- do not pretend the repo has story/reference support unless those sidecars exist
- do not rewrite the content as part of review unless explicitly asked
- do not use `page check`, `story check`, or `reference check` unless the relevant sidecars or media are actually present
- do not treat `reference check` as a substitute for reading the relevant reference entries when judging claim support or release-readiness
- in `series`, do not assume book-scoped and shared reference entries agree; call out drift or source-of-truth ambiguity explicitly
- keep the findings ordered by severity and anchored to the repo files or sections under review
- do not turn a review request into a rewrite request
