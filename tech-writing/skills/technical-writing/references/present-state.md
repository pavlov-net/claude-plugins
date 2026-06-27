# Present state, not history

Comments and docs describe what the code does and requires *now*, for a reader who never
saw any earlier version. History — what changed, what it used to do, what was removed,
who changed it — belongs in version control, changelogs, and issue trackers. A comment
that describes a moment in time rots at the next edit, and a stale comment is worse than
none: inconsistent comments correlate with bug-introducing commits.

## Present tense by default

Describe behavior in the simple present ("returns", "calls", "retries"), not the future
or conditional ("will return", "would then call", "is going to"). Reserve `will` for
deferred actions the present tense can't express ("the job will retry on the next
scheduler tick") and `should`/`might` for
real, flagged uncertainty — not for a definite behavior.

```
// bad:  This function will iterate over the list and should return the first match.
// good: Returns the first matching element, or null if none match.
```

Mood (indicative "Returns" vs imperative "Return") follows the ecosystem rule in
`references/docstrings.md`; this section governs present-vs-future tense only.

## Describe the state, not the change

Ban transition framing: `used to`, `previously`, `changed to`, `now we`, `switched
from`, `as of this PR`, `instead of the old`. If you catch a past-tense verb about the
code's own evolution, or a "now/now that" pivot, rewrite it as a flat present-tense
statement of behavior.

```
// bad:  We used to fetch users one at a time, but now we batch them for performance.
// good: Batched: one request per 500 users to avoid N+1 round-trips.
```

## No journal, changelog, or author/date stamps in source

No dated change logs, `@modified`, author/date banners, or "v1.2: added retry" lists in
or above a function. Delete such blocks when you encounter them. Git records who changed
each line, when, and why — more accurately than a header nobody updates.

## Delete dead code; never commit it commented-out

When code is unused, delete it. No commented-out blocks, no "old value" beside a new
one, no `if (false)` / `#if 0`. Readers can't tell a commented-out block from intentional
documentation, and version control restores the prior version exactly if it ever matters.

## Timeless documentation

Remove words that anchor prose to the moment of writing: `currently`, `now`, `new`,
`recently`, `soon`, `latest`, `as of this writing`, `presently`, `for now`. State
capabilities as plain present facts. Anchor a genuinely time-bound fact to a concrete
version or issue ("Since v2.4", "tracked in #1290") instead of a relative word.

Carve-out: terms of art keep their words. "Eventually consistent" is a distributed-
systems term; "eventually completes" describes real async behavior; a `latest` tag may
be a real API concept. Don't mechanically strip these.

## Don't narrate removals, absences, or comparisons to the old version

Don't describe what the code no longer does or what was taken out (`no longer`,
`removed`, `this replaces`, `unlike the old dashboard`, `the previous API required`).
State what's true now, self-contained. Migration differences belong in a migration
guide.

The legitimate kernel — "don't re-add X" — is a present constraint, not history:

```
// bad:  The legacy XML config parser was removed in favor of JSON.
// good: Config must be JSON. XML is intentionally unsupported (perf + injection risk);
//       see #842 before re-adding.
```

## Capture the invariant behind a fix, not the fix event

When you fix a bug, don't comment the event ("fixed crash on negative values", "added
null check to fix #123"). Record the durable constraint the code must preserve, and link
the issue. Test: would this sentence still be true and useful to someone who never knew
the bug existed? A constraint comment protects the fix from being innocently reverted; a
"fixed X" comment is archaeology git blame already holds.

```
// bad:  Fixed bug where negative quantities crashed checkout; added a check.
// good: Quantity must be >= 0; legacy CSV imports carry negatives (#1187).
```

## Document transitional coexistence as present state

When two paths genuinely coexist *right now* — an in-progress migration, a feature-flag
rollout, a deprecation window, a strangler-fig wrapper — that's present state, not
history, and must not be omitted. Describe the current division of responsibility in
present tense and state the concrete removal trigger with a link:

```
// Legacy and v2 billing both run; v2 is gated on the `billing_v2` flag.
// Remove this path when the flag reaches 100% (#842).
```

The bans above target narration of a vanished past, not description of a present in
which the old thing still exists. The failure to prevent is narrating the transition for
the PR reviewer — not omitting that the transition is live.

## Recognize history-by-design artifacts

Before applying these rules, check the artifact's purpose. CHANGELOGs, release notes,
ADRs, migration scripts, RFCs, and postmortems exist to record history: past tense,
dates, version anchors, and "changed from X to Y" are correct content there, not defects.
The present-state discipline applies to code comments, reference docs, how-to guides, and
tutorials — the artifacts that describe what's true now. Misclassifying a changelog as
"prose to make timeless" is as much a failure as putting a changelog inside a function.

## Route real history to its proper home

History isn't banned from the project — only from the artifact's inline description.
Route it:

- Rationale and "what/why changed" -> the commit message.
- User-facing changes -> CHANGELOG.md / release notes.
- Obsolescence -> a structured marker that names the replacement (`@deprecated Use
  newMethod()`), not a prose backstory.
- Broader context -> the linked issue or ADR.

## TODO/FIXME points to a tracked issue

Use a consistent, greppable format that references an issue holding the real context:
`TODO(#4521): drop this shim once the upstream rate-limit fix ships`. Never trigger a
TODO on a date (`TODO(2022)`), a passing release ("after Q3"), a person's name as the
context, or a vague "fix later". If there's no issue, file one first. A bare `TODO: make
it work` is worse than no comment — it pretends to convey intent.

## Sources

[Comments are not Version Control](https://coding.abel.nu/2012/07/comments-are-not-version-control/);
Google [Timeless documentation](https://developers.google.com/style/timeless-documentation)
and [Tense](https://developers.google.com/style/tense); [code-comment inconsistency
study](https://arxiv.org/abs/2409.10781); Google [TODO Comments](https://google.github.io/styleguide/cppguide.html#TODO_Comments);
[Keep a Changelog](https://keepachangelog.com/); Fowler, [Branch By Abstraction](https://martinfowler.com/bliki/BranchByAbstraction.html)
and [Feature Toggles](https://martinfowler.com/articles/feature-toggles.html); Nygard,
[Documenting Architecture Decisions](https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions).
