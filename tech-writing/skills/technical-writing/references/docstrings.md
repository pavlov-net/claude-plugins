# Docstrings and API docs: the contract

A docstring is a contract written for unknown future callers reading it in isolation — in
hover tooltips, generated doc sites, and index listings — not a narration of the current
code. The same skeleton recurs across Rust, Python, JS/TS, and Java: a self-contained
summary line, present-tense behavior (not implementation), explicit failure-mode sections,
per-parameter semantics the type can't express, and a runnable example.

## Lead with a one-line, self-contained summary

The first sentence is a single standalone sentence that fully states what the item does,
understandable with zero surrounding context. Doc tools extract only this sentence for
index and hover listings. End it with a period; don't embed `e.g.` or other mid-sentence
periods that truncate the extracted summary. Don't open with "This function is responsible
for...".

```
// bad:  /// This is a helper that, when called, will, e.g. for caching, go through the
//       /// list and return the items.
// good: /// Returns the cached items, computing and storing any that are missing.
```

## Match the ecosystem's voice, and stay consistent

There's no universal mood — conform to the ecosystem and never mix within a file:

- Rust, Java, JS/TS: third-person present indicative — "Returns", "Gets", "Computes".
- Python (PEP 257): imperative — "Return", "Fetch".

Avoid second person ("Get the foo") and "This function...".

## Document the contract, not the implementation

Describe what the item guarantees to callers — inputs, outputs, observable side effects,
errors, invariants — not how it currently computes them. Keep "how" notes inside the body.
Mention implementation only when it affects use (complexity, allocation, thread-safety). A
caller should be able to use the item without reading its body.

```
// bad:  """Loops over items, builds a HashMap, then binary-searches it to find the match."""
// good: /// Returns the index of the first element equal to `target`, or `None`. Runs in O(n).
```

## Add information beyond the signature

A docstring must say something the name, parameters, and types don't already say. Don't
write "the id" for a parameter named `id`, and don't reiterate the signature. In typed
languages (TypeScript, typed Python, Rust), don't restate the type — drop JSDoc
`@param {string} name` tags and `: int` echoes; the compiler owns the type, and a type
copied into prose drifts the instant the signature changes.

```
// bad:  def set_timeout(timeout):
//           """Set the timeout. Args: timeout: the timeout."""
// good: def set_timeout(timeout):
//           """Set the socket read timeout.
//
//           Args:
//               timeout: Maximum seconds to wait for data; 0 blocks forever.
//           """
```

## Document the full failure contract

Document every way a call can fail or surprise the caller: error/`Err` conditions (Rust
`# Errors`, Python `Raises:`, JS/Java `@throws`), panic conditions (Rust `# Panics`),
unsafe invariants the caller must uphold (Rust `# Safety`), and null/None handling for
each non-primitive parameter and return. Reserve these for non-obvious failures, not ones
the type already implies. Callers can't defend against failures they're never told about. Document only the
failures the code actually produces; an invented `# Errors` section is worse than none.

```
/// Returns the parsed config.
///
/// # Errors
/// Returns `Err` if the file is missing or contains invalid TOML.
///
/// # Panics
/// Panics if `path` is empty.
```

## Document parameter semantics the type can't express

For each parameter and return, state what the type and name omit: units (ms, bytes), valid
range and constraints, the default's meaning (not its literal value when the signature
already carries it; see `references/structure-and-reuse.md`), whether null/None is accepted and how
it's treated, and ownership/mutation. Type annotations give the shape; the doc gives the
meaning — `timeout: int` says nothing about units or whether 0 means "no timeout".

## State preconditions vs. guarantees (direction of responsibility)

Make the contract's direction explicit. Separate what the *caller* must guarantee before
calling (preconditions: "buffer must be non-null and at least `len` bytes"; "must hold the
config lock") from what the function *guarantees* on return (postconditions: "the returned
slice is sorted and owned by the caller"). A precondition violation is the caller's bug; a
postcondition failure is the callee's. Don't bury a caller obligation inside a flat
description of behavior.

## Provide a realistic, runnable example for non-trivial public items

Give every non-trivial public item a concrete example (Rust `# Examples`, `@example`) with
imports and assertions, showing realistic usage. Hold examples to a "must compile and run"
standard (Rust enforces this with doctests). Use proper error handling (Rust `?`, not
`unwrap()`) — users copy example code verbatim, and unwrap-laden examples teach panicking
code.

```
/// Parses a `key=value` pair.
/// ```
/// let c = parse("port=8080")?;
/// assert_eq!(c.port, 8080);
/// ```
```

## Use standard sections; split summary from detail

Structure docs with the ecosystem's machine-recognized sections, not a prose blob: Rust
`# Examples / # Panics / # Errors / # Safety`; Python `Args: / Returns: / Raises:`; JS/TS
`@param / @returns / @throws / @remarks / @deprecated`; Java `@param / @return / @throws /
@since`. Keep the conventional order. Keep the lead summary short; push elaboration,
rationale, and edge cases into the body. Doc generators extract these sections, and readers
scan for them.

## Sources

[Rust RFC 1574](https://rust-lang.github.io/rfcs/1574-more-api-documentation-conventions.html)
and [API Guidelines](https://rust-lang.github.io/api-guidelines/documentation.html) (C-FAILURE,
C-EXAMPLE, C-QUESTION-MARK, C-LINK); [PEP 257](https://peps.python.org/pep-0257/);
[Google Python Style Guide](https://google.github.io/styleguide/pyguide.html);
[NumPy docstring guide](https://numpydoc.readthedocs.io/en/latest/format.html);
[TSDoc](https://tsdoc.org/); [Javadoc coding standards](https://blog.joda.org/2012/11/javadoc-coding-standards.html);
[Design by contract](https://en.wikipedia.org/wiki/Design_by_contract).
