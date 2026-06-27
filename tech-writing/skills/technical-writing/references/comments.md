# Code comments: what earns a comment's place

A comment exists to carry information the code cannot — intent, rationale, precision, a
warning. Any comment that restates, decorates, or journals the code is a liability that
must be read, maintained, and kept in sync, and it's the first thing to drift into a lie.
LLMs over-produce the worthless kind (line-by-line narration, banners) and under-produce
the valuable kind (rationale, precision, workaround notes).

## Altitude: a comment sits at a different level than the code

This is the core rule; "comment the why, not the what" is a useful shorthand for it but
too narrow taken literally. A comment earns its place by adding information at a
*different altitude* than the line it describes:

- **Higher** than the code: intent, rationale, what the block accomplishes as a whole, or
  a plain-language gloss of genuinely opaque code.
- **Lower** than the code: precision the signature can't carry (units, ranges, null
  meaning, ownership).

A comment at the *same* altitude — restating the statement in English — is the failure.

The "different words" test: could someone who never saw the code write this comment just
by reading the line next to it? If yes, delete it.

```
// bad:  i++; // increment i by one
// good: i++; // skip the trailing newline so the parser resumes at the next record
```

Do not enforce a rigid "no why means no comment" rule: a precision "what" (`// ms; 0
disables`) and a gloss of a dense regex are valuable whats. The test is altitude, not the
word "why".

## Prefer the why

Spend comments on what the code can't express: why this approach, what constraint or
trade-off forced it, why the obvious alternative was rejected. A future reader can
reconstruct *what* and *how* from the code; the *why* lived only in the author's head and
is unrecoverable once lost — and its absence leads people to "fix" deliberate code and
reintroduce bugs.

```
// good: 8% = combined state+county tax for our single jurisdiction; revisit before we
//       expand. TAX-412
total = price * 1.08;
```

## Prefer clearer code over a comment

A comment is justified only when the information is both needed and can't be made obvious
in the code. If a clearer name, a named constant, a type, an assertion, or a test closes
the gap, do that instead — those stay honest automatically because they break the build
when reality diverges. Never use a comment to apologize for confusing code; fix the code.

```
// bad:  int d; // elapsed time in days
// good: int elapsedDays;
```

## Add precision the names can't carry

Document the contract details a signature omits: units (ms, bytes), valid ranges,
inclusive/exclusive bounds, the meaning of null/empty, the meaning of special values (0 =
disabled), resource ownership and lifetime (who closes/frees), thread-safety, and
invariants. These are exactly the facts that cause off-by-one, wrong-unit, use-after-free,
and null bugs when omitted.

```
// good: Idle-socket timeout in milliseconds; 0 disables it. Must be >= 0.
int timeoutMs;
```

## Comment the surprise

Where a line looks wrong, redundant, or removable but is required, state why it must stay
— the bug it works around, the ordering or timing dependency, the spec quirk — with a
link. This is the comment that most repays its cost: without it, a future reader (or an
LLM doing cleanup) deletes the line and reintroduces the bug. Chesterton's fence.

```
// good: time.sleep(0.1)  # Vendor API 429s on calls < 100ms apart (SUPPORT-3391). Do not remove.
```

## Gloss genuinely opaque code

When a line is intrinsically hard to read — a dense regex, bit-twiddling, a non-obvious
algorithm, a gnarly boolean — add a one-line plain-language summary of *what* it computes,
even though it's a "what". The code is the hard part here, so a higher-altitude
restatement saves every future reader from re-deriving the meaning.

## Document usage protocol and cross-module coupling

Document obligations that span calls or files and are invisible in any single location:
required call order ("call connect() before send()"), state that must hold ("valid only
after init() returns"), locks held or required on entry, and dependencies on distant code.
When two separated pieces are coupled — one assumes the other ran, or holds a lock — state
the coupling at both ends and link them.

## Document at module, file, and class/type level

Write doc comments at every altitude, not only on functions:

- **Module / file / crate**: the unit's purpose, its primary entry points, how the pieces
  fit, so a reader knows where to start.
- **Class / type**: what the abstraction represents, the invariants it maintains, its
  usage protocol (required call order, lifecycle, ownership), and thread-safety.

These facts belong to no single method and have no other home. Function-only docs leave a
reader unable to form a mental model or pick the right entry point.

## Write the comment first (a design check)

For a non-trivial function or class, draft the interface comment before the body. If the
comment is hard to write, long, or forced to enumerate many special cases to describe one
signature, the interface is probably too complex — fix the design, not the prose. A clean
abstraction is one whose contract is easy to state.

## Keep comments adjacent and true

Treat every comment touching the code you change as part of the change set: re-read each
one and update or delete any whose claim is no longer exactly true. Place each comment
immediately next to what it describes. A comment you leave behind unverified is assumed
stale.

```
// bad:  // retry up to 3 times
//       for (int i = 0; i < 5; i++) { ... }
// good: for (int i = 0; i < MAX_RETRIES; i++) { ... }
```

## No decorative banners or attribution

Drop ASCII-art section banners, `/////` dividers, closing-brace labels (`} // end for`),
and author/byline stamps. Git blame supplies authorship; folding and small functions
supply structure.

Carve-out: keep required, machine-read headers — SPDX license identifiers, copyright
notices, codegen markers (`@generated`), and lint/policy-mandated module docstrings. These
aren't decorative; they're often non-negotiable.

## Public items: minimal doc, never a tautology

Most ecosystems and CI gates expect a doc on every public/exported symbol (Rust
`missing_docs`, pydocstyle, doc-site generators, IDE hover). Never write a tautology — but
when a public item's doc would be tautological, fix the name or add the one
non-obvious fact (units, nullability, ownership, error). Fall back to no doc only for
private/internal trivia.

## Sources

Ousterhout, [A Philosophy of Software Design](https://web.stanford.edu/~ouster/cgi-bin/cs190-winter18/lecture.php?topic=comments)
(comments at a different level; write comments first; interface vs implementation);
Martin, *Clean Code* ch.4 (redundant/noise/journal comments); Google [engineering
practices](https://google.github.io/eng-practices/review/reviewer/looking-for.html);
Atwood, [Code Tells You How, Comments Tell You Why](https://blog.codinghorror.com/code-tells-you-how-comments-tell-you-why/);
[Stack Overflow blog on code comments](https://stackoverflow.blog/2021/12/23/best-practices-for-writing-code-comments/).
