# Structure and reuse

Two failure modes at the document level: content with different purposes jammed together
(structure), and the same fact copied into several places where the copies drift (reuse).
Diátaxis and BLUF fix the first; single-source-of-truth fixes the second.

## Lead with the answer (BLUF / inverted pyramid)

Put the most important information first — the return value, the contract, the warning,
the conclusion — in the first sentence of a docstring or comment, the first sentence of
each paragraph, and the first paragraph of each doc. Docs are skimmed, not read end to
end, so front-loading lets a reader stop as soon as they have what they need. State a
condition before its instruction ("If X, do Y"), not after.

```
// bad:  After the loader parses the file, resolves includes, and validates the schema,
//       and assuming no errors, it returns a Config.
// good: Returns a Config; raises ConfigError on failure. (It parses the file, resolves
//       includes, then validates the schema.)
```

## Make headings and summary lines state the takeaway

Write section headings, captions, doc titles, and the first docstring line as the
conclusion to remember, not a vague topic label ("Overview", "Notes", "// helper"). For
any summary element, answer "after reading this, what's the one thing the reader should
take away?" and put that in the heading. Trim marketing adjectives from titles
("comprehensive", "ultimate", "powerful").

```
// bad:  Caption: "Diagram of the system."   Heading: "Notes."
// good: Caption: "Requests hit the cache before the DB, cutting DB load ~80%."
//       Heading: "Why writes are batched."
```

## One mode per document (the Diátaxis compass)

Classify the reader's need on two axes and commit the whole document to one mode:

| | Acquisition (learning) | Application (working) |
| --- | --- | --- |
| **Action** | Tutorial — learning by doing | How-to — getting a task done |
| **Cognition** | Explanation — understanding why | Reference — looking up facts |

Mixing modes is "at the heart of a vast number of problems in documentation." A page that
teaches, instructs, lists, and explains at once serves no reader: the learner drowns in
option tables, the looker-up wades through narrative. Asked to "write docs", don't emit
one undifferentiated blob covering install, full config tables, and architecture
rationale.

## Keep action and cognition apart

Strip background, rationale, and theory out of procedural content (tutorials, how-to,
reference); strip step lists and option tables out of explanation. If you feel the urge to
write "This works because..." inside a how-to step, move that sentence to an explanation
doc and link it.

```
// bad:  Step 3. Run `migrate --safe`. We use --safe because migrations could corrupt the
//       index when run concurrently, which is rooted in how the WAL replays...
// good: Step 3. Run `migrate --safe`. (See "Why migrations need --safe".)
```

- **Tutorial**: one guaranteed-to-succeed path. No forks, no "you could also...", no
  decisions left to a learner who doesn't yet know what they don't know.
- **How-to**: titled with the goal ("How to register a webhook"), assumes the basics, and
  includes only what completes that task. Don't re-teach fundamentals.
- **Reference**: one job — describe. Accurate, complete, consistent, structured to mirror
  the code, and boring. State the contract without instructing or motivating.
- **Explanation**: answers "why?" and "what if?" as connected prose bounded to one topic,
  with steps and tables linked out. It may admit uncertainty and discuss rejected
  alternatives.

Code comments have modes too: a docstring is reference (describe the contract); an inline
comment is explanation (the non-obvious why). Don't write tutorial prose in either.

## Organize by reader need, not the shape of the code, diff, or prompt

Decide a doc's existence and placement by asking "what is a future reader trying to do, and
in what situation?" Don't let the diff, the file you're editing, or the order things appear
in the code dictate the documentation's structure. A new `notes-on-auth-refactor.md`
describing every function the current PR touched is stranded next to the diff; the real
home is an update to the existing how-to a reader consults to authenticate.

## Lists for parallel facts; prose for connected reasoning

Use a bulleted list (numbered for sequences) when you'd write three or more parallel items,
with items grammatically parallel and consistently punctuated. But don't fragment connected
reasoning into a bullet wall — bullets sever the causal links ("why A forces B") a reader
needs. Heuristic: 8+ bullets in under ~200 words, or 3+ headings in under ~300 words,
signals scaffolding that should be prose. Keep each paragraph to one topic.

```
// bad:  Retry behavior:
//       - Exponential backoff
//       - Jitter applied
//       - Max 5 attempts
//       - Idempotency required
// good: Retries use exponential backoff with jitter, capped at 5 attempts. Because
//       retries can duplicate a request, the endpoint must be idempotent.
```

## One fact, one home

Every discrete fact, rule, default, or invariant gets exactly one authoritative location,
chosen by where the reader needs it (the declaration, the config, the API reference).
Before writing an explanation, ask "does this already live somewhere?" If yes, reference
it; if no, write it once and make that the canonical home. Two copies are two things to
update; when only one is updated — the usual outcome — readers can't tell which is current.

## Reference, don't restate — by a stable handle

When information already exists, cross-reference it instead of paraphrasing. A reference
inherits its target's correctness for free; a paraphrase must be maintained forever. Rank
handles by stability and verifiability:

1. Tool-checked symbol links — Rust intra-doc `[\`Type::method\`]`, Javadoc/JSDoc `{@link}`.
   These break the build when the target moves.
2. Symbol/identifier names; issue or ADR numbers.
3. File paths — break silently on a move.
4. Doc/URL anchors — 404 silently.
5. Line numbers and "the function above/below" — rot faster than the duplication they
   replace. Avoid.

Prefer the most stable handle available, and prefer one a tool can verify.

## Don't hardcode code-owned values into prose

Don't copy values the code owns — magic numbers, defaults, signatures, enum/option lists,
CLI flags, env-var names, error strings, paths — into comments, docstrings, or READMEs.
Reference the symbol or generate the doc from source. A doc that says "default timeout is
30s" becomes a lie the moment `DEFAULT_TIMEOUT` changes to 60. Document a default's
*meaning* in the docstring ("0 disables the timeout"), not its literal value; see
`references/docstrings.md`.

## Document next to the code (proximity)

Put an explanation as close as possible to what it describes: a docstring over a far-off
wiki page, a comment above the tricky branch over a separate design doc. Co-located docs
are in view when the code changes, so they get updated in the same edit. Reserve
distant/external docs for cross-cutting or conceptual material.

## Assign each doc tier one job; don't re-document common tools

API facts live in docstrings (next to code); conceptual "how it fits" lives in one guide;
setup/run lives in the README. When two tiers need the same fact, one owns it and the
others link. Never write your own guide to a common technology (Git, Docker, a standard
protocol) — link its canonical home.

## Kill low-information-density paragraphs

Every paragraph and comment must add new information. Ask of each one: "what fact, claim,
or constraint is new here?" If it only restates the heading or the previous sentence in
fresh words, delete it. LLMs restate the same idea in varied phrasing (the treadmill
effect), so prose feels substantial while conveying little.

```
// bad:  The retry logic retries failed requests. When a request fails, it attempts the
//       request again. This re-attempting is what the retry mechanism provides.
// good: Failed requests are retried up to 5 times with exponential backoff.
```

## Treat docs as code; enforce with tooling

Update docs in the same commit as the code they describe, store them in the repo, and
review them like code. Prefer a small set of accurate docs over a large pile in disrepair;
delete docs that are certainly wrong rather than leave them to mislead.

"A human will remember to update this" is the weakest guarantee. Prefer doc forms whose
drift a machine catches, wired into CI: runnable examples as doctests (Rust, Python
`doctest`), generated reference/CLI-help/config docs diffed against source, link-checkers
for cross-references, and lints that flag missing or inconsistent docs (clippy
`missing_docs`, `missing_errors_doc`, `missing_panics_doc`).

## Sources

[Diátaxis](https://diataxis.fr/start-here/) and the [compass](https://diataxis.fr/compass/);
[Divio documentation system](https://docs.divio.com/documentation-system/);
[Write the Docs: docs as code](https://www.writethedocs.org/guide/docs-as-code/);
Google [Technical Writing One](https://developers.google.com/tech-writing/one/documents)
and [style highlights](https://developers.google.com/style/highlights);
[BLUF](https://en.wikipedia.org/wiki/BLUF_(communication)); [DRY](https://en.wikipedia.org/wiki/Don%27t_repeat_yourself);
Google [docguide best practices](https://google.github.io/styleguide/docguide/best_practices.html).
