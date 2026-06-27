# Word lists: cut, swap, and scan

The grep-and-fix reference for the deletion pass and the tells pass. Each list is a
mechanical edit: find the pattern, apply the fix. Most entries are near-lossless — the
meaning survives the cut.

## Circumlocutions -> one word

| Instead of | Use |
| --- | --- |
| in order to | to |
| due to the fact that | because |
| in the event that | if |
| at this point in time | now |
| has the ability to / is able to | can |
| a number of | several |
| in addition | also |
| for the purpose of | to |
| with the exception of | except |
| in the process of | (delete) |
| establish connectivity to | connect to |
| make use of | use |
| a large number of | many |

## Throat-clearing and reinforcing filler -> delete

`it is important to note that`, `it's worth noting`, `please note`, `note that`, `keep
in mind that`, `needless to say`, `as you can clearly see`, `obviously`, `clearly`, `of
course`, `basically`, `essentially`, `as we all know`.

Prefixing a fact with "it is important to note that" does not make it truer. If it's a
real caveat, state the caveat, not a label announcing one.

## Hedges and intensifiers -> delete or make precise

`very`, `really`, `quite`, `fairly`, `somewhat`, `generally`, `actually`, `simply`,
`just`, `relatively`, `a bit`, `sort of`, `kind of`, `pretty`.

Never stack hedges ("might possibly", "could perhaps eventually"). Keep a qualifier
only for real uncertainty, then name what's uncertain: not "may sometimes block" but
"blocks when the queue is full".

## Transition-adverb studding -> delete or merge

`Furthermore`, `Moreover`, `Additionally`, `Consequently`, `Hence`, `Thus`,
`Therefore` (as an opener), `Notably`, `Importantly`, `Significantly`, `Ultimately`,
`That said`, `With that being said`, `In essence`, `It's worth mentioning`.

Two sentences in order usually imply their own link. If they need one, use a plain word
("so", "but", "because") or merge them. Never open consecutive sentences with these.

## Empty conditional qualifiers -> name the condition or cut

`as needed`, `as appropriate`, `where appropriate`, `as required`, `if necessary`,
`when applicable`, `depending on your needs`, `based on your use case`, `as desired`.

These dodge the commitment the reader came for. Either state the real condition ("set
retries above 0 if the endpoint is flaky") or drop the qualifier and state the fact.

## Weak verbs and nominalizations -> strong verb

| Instead of | Use |
| --- | --- |
| make a decision | decide |
| perform a calculation | calculate |
| provide validation of | validate |
| is responsible for handling | handles |
| has a dependency on | depends on |
| performs the initialization of | initializes |
| make use of | use |
| give consideration to | consider |

Restore the verb buried in a noun propped up by be/have/make/do/perform.

## Expletive openers -> promote the subject

- `There is / There are / There exists ...` -> delete and lead with the real subject.
- `It is X that ...` -> state X directly.
- `You can use X to ...` -> `Use X to ...`.

"There are three options that you can use to configure retries" -> "Configure retries
with one of three options:".

## Redundant pairs and negatives -> trim, state positively

- `end result` -> result, `completely eliminate` -> eliminate, `merge together` -> merge,
  `each and every` -> each, `still remains` -> remains, `past history` -> history.
- `not able to` -> cannot, `does not have` -> lacks, `did not allow` -> prevented.
- Never use a double negative in a contract; it's a defect.

## Flowery AI vocabulary -> plain

| Instead of | Use |
| --- | --- |
| delve into | look at |
| leverage / utilize | use |
| robust | reliable |
| comprehensive | complete |
| seamless / seamlessly | smooth / (delete) |
| facilitate | help |
| meticulous | careful |
| pivotal / crucial | key |
| underscore | show |
| intricate | complex |
| showcase | show |
| realm / landscape / ecosystem | (name the thing) |
| commence / initiate | start |
| prior to | before |
| modify | change |
| sufficient | enough |

These words spiked in human writing only after ChatGPT shipped; they signal style, not
content.

## Significance inflation and self-praise -> delete

`powerful`, `elegant`, `vibrant`, `cutting-edge`, `state-of-the-art`, `game-changing`,
`best-in-class`, `blazing fast`, `this elegant solution`, `a powerful abstraction`,
`cleanly handles`, `gracefully handles`, `marks a major improvement`.

State what the code does, not how impressive it is. Self-praise ages badly and erodes
trust.

## Copula avoidance -> is / has / lets

`serves as` / `functions as` / `represents` -> `is`; `boasts` / `features` / `offers` ->
`has`; `enables you to` -> `lets you`.

## Syntactic templates -> direct statement

- `It's not just X, it's Y` / `not only X but also Y` / `more than just X` -> state Y.
- Rule of three (`fast, scalable, and reliable`; `configure, validate, and deploy`) ->
  use the number of items that are actually true and distinct, often one or two. The
  invented third item becomes a false claim about behavior.

## Chat residue and scaffold openers -> delete

Sycophancy: `Certainly!`, `Great question!`, `You're absolutely right`. Closers: `Hope
this helps!`, `In conclusion`, `In summary`. Scaffolding: `Let me think step by step`,
`First, let's consider`, `Let's dive in`, `Let's create a function that`. Hooks: `Ever
wondered how X works?`, `Here's the thing`, `Plot twist:`. Start with the substantive
first word.

## Placeholder and generation artifacts -> grep and remove

Before committing, grep for and delete: `oai_citation`, `contentReference`,
`utm_source=chatgpt.com`, `[INSERT ...]`, `[Your Name]`, `TODO: make it work`,
`2025-XX-XX`, `As of my last update`, `As an AI`. Fill placeholders with real content
or delete the line.

## ASCII punctuation map

Use plain ASCII punctuation in comments, docstrings, and repo docs. Replace lookalike
and invisible glyphs — they break grep, diffs, compilers, YAML, and pasted example code.

| Glyph | Name | Replace with |
| --- | --- | --- |
| `’ ‘` | curly single quotes | `'` |
| `” “` | curly double quotes | `"` |
| `…` | ellipsis (U+2026) | `...` |
| `–` | en dash | `-` |
| `→ ⇒ ⟶` | arrows | `->` / `=>` |
| `×` | multiplication sign | `x` |
| `•  ·` | bullet / middot in prose | a real list marker |
| (U+00A0) | non-breaking space | normal space |

Treat any byte above 0x7F in a code comment as suspect unless it's intentional content
(a unit symbol the API actually uses). Em-dashes are a style call, not a banned glyph:
the tell is several dramatic asides per paragraph, not the character.

## Terseness has a floor

After cutting, do a rhythm check. Over-terseness is its own tell: don't strip articles
and connectives into telegraphic stubs ("Returns config. Throws on fail. Validates
first."), and don't emit a wall of identical short sentences. Keep the words a reader
needs on the first pass, and vary sentence length. The goal is the shortest text that
still reads naturally, not the shortest text.

## Sources

Strunk & White, *The Elements of Style* (Rules 11-13); Zinsser, *On Writing Well*
(Clutter); plainlanguage.gov; Google [developer style word list](https://developers.google.com/style/word-list)
and [Technical Writing One](https://developers.google.com/tech-writing/one/words);
Microsoft Writing Style Guide; [Wikipedia: Signs of AI writing](https://en.wikipedia.org/wiki/Wikipedia:Signs_of_AI_writing);
[avoid-ai-writing](https://github.com/conorbronsdon/avoid-ai-writing); Science Advances
on [post-ChatGPT vocabulary shift](https://www.science.org/doi/10.1126/sciadv.adt3813).
