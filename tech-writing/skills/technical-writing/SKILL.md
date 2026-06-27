---
name: technical-writing
user_invocable: false
description: >
  This skill should be used whenever Claude writes, edits, or reviews documentation or
  non-trivial code comments and docstrings: doc comments, READMEs, API references,
  guides, design docs, module and function docs. It also fires on requests like "write a
  docstring", "document this function/module", "clean up the comments", "improve the
  docs", "make this less verbose", "tighten this up", or "this reads like AI wrote it".
  It produces terse, present-tense prose for the future reader of the merged code:
  grounded in what the code actually does, no filler or hedging, no changelog or "used
  to" narration, no duplication, no emoji or decorative unicode, no AI tells. Apply it
  even when the user never says "documentation".
---

# Technical writing: docs and comments

Comments and docs are a contract for a future reader who has only the merged code, not
the chat, the diff, or the PR. Write the present state, say each thing once, use the
fewest words that stay clear, and add a comment only where it earns its place. This
covers writing new docs and cleaning up existing ones.

Left alone, an LLM writes a verbose first draft for the current session: too long,
restating the code, narrating the change, decorating with unicode, sometimes inventing
behavior the code lacks. The rules below counter that; the checklist at the end enforces
them.

## Stay in scope

When you edit docs as part of a code change, touch only the comments and docs your change
adds, changes, or makes false. Leave every other pre-existing comment alone, even one
that reads like an LLM. Retightening unrelated prose is a separate task, and a clean but
out-of-scope diff is a defect a reviewer has to revert.

The exception is an explicit "clean up all the comments in X" request: then X is the
scope.

## Ground every claim in the code

Docs fail factually before they fail stylistically. A confident "raises ValueError on
empty input" for a function that does not is worse than any verbosity.

- Verify every documented behavior, parameter, default, error, unit, and invariant
  against the actual code. Don't document a failure mode or parameter you haven't
  confirmed exists.
- Don't invent to fill a section. An empty `# Errors` beats a fabricated one.
- Before naming or explaining a concept, search the codebase for what it is already
  called and whether it is already documented. Conform to that term and link the home
  instead of coining a synonym or writing a second copy.

A wrong comment is worse than none.

## The core rules

Write for the next competent contributor to this codebase: fluent in the language and the
domain, but without your session context. Spend words only on what that reader cannot
infer from the code in front of them.

- **Be terse.** Lead with the point. Cut filler and hedges. Don't explain the obvious or
  justify mechanics a domain peer already knows. (`references/word-lists.md`)
- **Present state, not the change.** Describe what the code does now; history belongs in
  git, the changelog, and the commit message. (`references/present-state.md`)
- **Comments add what the code can't.** A comment earns its place only at a different
  altitude than the code: the why, a precision the signature omits, or a surprise. Never
  a restatement of the line. (`references/comments.md`)
- **Docstrings are a contract.** A self-contained summary line, the real failure modes,
  and the parameter facts the type can't carry. (`references/docstrings.md`)
- **Say each thing once.** Every fact has one home; link to it by a stable handle.
  (`references/structure-and-reuse.md`)
- **Don't sound like an AI.** Plain words, ASCII punctuation, no decorative unicode, and
  plain phrasing, not clever. State facts; don't perform them. (`references/word-lists.md`)

## Length budgets

A ceiling, not a target. Exceed it only for a real failure contract, a load-bearing why,
or a documented usage protocol, and only by as much as that needs.

| Artifact | Budget |
| --- | --- |
| Inline comment | 1-2 lines |
| Simple docstring (clear name, obvious behavior) | one sentence, or none |
| Typical docstring | summary line + only the sections that carry real content |
| Module / file doc | a short paragraph naming the purpose and entry points |
| README section | a lead sentence + the minimum to act |

If a doc on a simple item runs to multiple paragraphs or `#` sections, that is the
defect. Collapse it to the contract plus the non-obvious why.

## Cleaning up: classify before you cut

Most cleanup is deletion, but classify each existing comment first. Over-cutting a why is
as much a defect as verbosity: a deleted why is unrecoverable, while an over-long one only
costs reading time.

- **Protect** (tighten the words, never drop the fact): a why or rationale; a constraint
  or trade-off; a workaround and its issue link; an ordering or timing dependency; an
  invariant or precondition; a precision the signature omits (units, ranges, null
  meaning, ownership, thread-safety).
- **Delete whole** (don't just shorten): a restatement of the code; a banner or divider;
  a journal, changelog, or date stamp; throat-clearing; commented-out code; a tautology.
- **When unsure, keep the fact and cut its words.** Delete a why only if the same fact is
  recoverable from the code, a name, a test, or a named link, or it is factually wrong.

## Before you finish

Run this on every comment and doc you touched:

1. **Scope.** Every changed line traces to code you changed or a comment it made false.
   Revert any pure restyling of an untouched comment.
2. **Budget.** Each comment is within its length budget, or has a stated reason to exceed.
3. **Classify and cut.** Protected facts kept and tightened; droppable comments deleted
   whole. For each non-trivial comment, name what every surviving clause carries
   (contract, why, precision, or warning); a clause you can't label is a deletion
   candidate.
4. **Tells.** Grep the draft against the blocklists (filler, flowery vocab, transition
   adverbs, emoji, non-ASCII punctuation, chat residue like "Certainly" or "[INSERT ...]").
   Fix each hit; they are invisible while generating.
5. **Hostile re-read.** Read it back as a maintainer who rejects anything that reads like
   an LLM. Name the most performative sentence and flatten it to the fact; confirm nothing
   over-explains the adjacent code.

## References

Read the matching reference when the situation calls for it:

- `references/word-lists.md` - the deletion and tells passes: substitution tables and
  blocklists (filler, hedges, transition adverbs, flowery vocab) plus the ASCII map.
- `references/present-state.md` - history, change, migration, time-bound facts, TODOs.
- `references/comments.md` - writing or cleaning inline, module, or class/type comments.
- `references/docstrings.md` - writing or reviewing a docstring or API doc.
- `references/structure-and-reuse.md` - structuring a doc, or deciding where a fact lives.
