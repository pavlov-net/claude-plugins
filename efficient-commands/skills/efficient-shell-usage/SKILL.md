---
name: efficient-shell-usage
description: >
  This skill should be used when implementing features, fixing bugs, running tests,
  running linting or formatting, building projects, verifying changes, debugging
  test failures, checking CI results, or executing any shell commands as part of
  development work. It prevents wasteful patterns like running the same command
  repeatedly with different tail/head/grep pipes, and re-running expensive commands
  when nothing has changed. Use this skill whenever shell commands are part of the
  workflow — even for simple verification steps.
---

# Efficient Shell Usage

Shell commands like test suites, linters, typecheckers, and builds are expensive — they
take wall-clock time, consume tokens reading output, and burn through rate limits.

## Rule 1: Run Once, Triage the Output

Never re-run a command to see a different slice of its output, and never pipe commands
through `tail`, `head`, or `grep` — that re-executes the entire command each time.

Instead, run once, save to file, and use tools to inspect:

```bash
command 2>&1 | tee /tmp/cmd-output.txt; echo "EXIT: $?"
```

Then triage:

- **Exit 0 (pass):** Move on. Do not read the output — there is nothing to fix.
- **Non-zero (fail):** Use Grep to find error lines in `/tmp/cmd-output.txt` (e.g.,
  "FAIL", "error", "Error:"). Then use Read with `offset` and `limit` to see context
  around those lines — typically 5-10 lines before and after is enough.
- **Last resort:** If Grep finds nothing useful, Read just the last 50 lines. Error
  summaries almost always appear at the bottom.

This applies to any output inspection. To search saved output, use the Grep tool. To
view a section, use Read with offset/limit. Never re-run the command or pipe it through
shell utilities to see different parts.

For short commands with small output (e.g., `git status`, `ls`), saving to a file is
unnecessary — just read the Bash tool result directly.

## Rule 2: Never Re-Run Without Changes

Do not re-run an expensive command unless something has changed that would affect its
result — a source file edit, a dependency change, a config change.

Do not re-run to "double-check" a passing result, to hope for a different outcome, or
to see the output differently (use saved output from Rule 1 instead). Do not re-run the
same tool through a different entry point (e.g., `npx eslint` after `npm run lint`
already ran eslint).

The sequence is: run → see failure → fix the cause → re-run.

## Rule 3: One Verification Pass

After making changes, run each verification step once. Only re-run a step if your fix
could plausibly affect its result.

```
typecheck → pass
lint → fail → fix → re-run lint → pass
tests → fail → fix → re-run tests → pass
# Done. The lint fix didn't affect typecheck. The test fix didn't affect lint.
```

Do not re-verify steps that already passed unless the fix touched something they check.
