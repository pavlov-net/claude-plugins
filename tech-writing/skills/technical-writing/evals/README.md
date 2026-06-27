# Evals for the technical-writing skill

A regression set that pins the behaviors the skill should produce, including the four
failures observed when the skill was first used on a real codebase: out-of-scope
rewriting, over-deletion of a load-bearing why, surviving verbosity, and performative
prose.

## Cases

`evals.json` holds six cases over the fixtures in `fixtures/`:

| # | Fixture | Tests |
| --- | --- | --- |
| 1 | `verbose_type_doc.rs` | Cut an essay-style public doc to the contract; keep every caveat |
| 2 | `parrot_comments.rs` | Delete whole comments that restate the code |
| 3 | `changelog_comment.rs` | Convert change narration to present state |
| 4 | `out_of_scope.rs` | Add one docstring; leave the unrelated verbose comment untouched |
| 5 | `load_bearing_why.rs` | Tighten a verbose comment without deleting its why |
| 6 | `failed_haiku.rs` | Flatten performative prose to the plain fact |

Cases 4-6 are guardrails: the wrong answer is to over-reach (4), over-delete (5), or keep
the performance (6). They are the regression tests for the gaps the real-world run exposed.

## Running

Each case is a task plus an input file. Run the task against the skill (for example via
the skill-creator eval loop, or by hand: apply the skill to the fixture and diff the
result), then check the `assertions`. Most assertions are mechanical and greppable; the
rest need an LLM judge or a human.

## Scoring rubric

Score each output on five dimensions. A regression on any of the first four is a failure,
not a style nit.

1. **Scope respected.** Only the comments the task targets changed. No unrelated
   pre-existing comment was rewritten, retightened, or reflowed. (Case 4 is the direct
   test; applies to all.)
2. **Grounded.** No invented behavior, error, parameter, or unit. Every surviving claim
   matches the code.
3. **Load-bearing facts preserved.** No why, constraint, workaround, issue link, or
   precision (units/ranges/null meaning/ownership/thread-safety) was dropped. Tightening
   wording is fine; dropping the fact is a failure. (Case 5.)
4. **AI tells: zero.** No flowery vocab, transition-adverb studding, decorative unicode,
   non-ASCII punctuation, chat residue, or performative epigrams. (Case 6; grep
   `references/word-lists.md` blocklists.)
5. **Terse and within budget.** Each comment is within its length budget (see SKILL.md)
   or has a stated reason. The result is shorter where the input was bloated, but not
   telegraphic.

This rubric is the same as the skill's "Before you finish" checklist; an output that
passes the checklist passes the evals.
