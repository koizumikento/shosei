---
name: "shosei-content-review"
description: "{{DESCRIPTION}}"
compatibility: "Requires a repository initialized by `shosei init` and a local `shosei` CLI in the working environment."
---

# Shosei Content Review

Review the requested manuscript, chapter, volume, outline, or proof packet using the project-specific lens below.

## Repo Notes

- This repo was initialized as `{{REPO_MODE}}` with the `{{PROJECT_TYPE}}` template.
- Primary config entrypoint: {{PRIMARY_CONFIG}}
- Primary content paths: {{PRIMARY_CONTENT_PATHS}}
- Optional sidecar paths: {{OPTIONAL_CONTENT_PATHS}}
- Review lens: {{REVIEW_FOCUS}}
- {{REPO_MODE_RULES}}

## Workflow

1. Establish the review scope first.
   - whole book, chapter, volume, proof packet, or a specific manuscript section
   - use the review lens above and the user's requested focus
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
   - assess the requested content using the review lens above
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
