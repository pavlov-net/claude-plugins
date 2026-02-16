---
name: focused-task
description: Use when you have a single implementation task that benefits from an isolated worktree, a dedicated team (lead + engineers), and verification gates before PR creation. Companion to parallel-tasks for when depth matters more than breadth.
user_invocable: true
---

# Focused Task Execution with Worktree

Execute a single task with a dedicated team in an isolated worktree. Produces one PR.

- One git worktree on a feature branch
- A **lead** teammate who plans the work and coordinates the team
- Optional specialized teammates: **engineers**, **architect**, **QA** (lead requests them in their plan)
- Thin **coordinator** in delegate mode — just spawns agents and monitors
- Verification gates (lint/typecheck/test, code review, code simplification, requirements check)
- One PR when complete

**When to use this vs parallel-tasks:** Use `focused-task` when you have one task and want
full team attention on it. Use `parallel-tasks` when you have 2+ independent tasks to run
simultaneously.

## Input

The user will provide:
- A short name (used for branch/worktree naming, e.g., `hybrid-search`)
- A description of what to implement

Optional overrides the user may provide:
- Extra quality check commands beyond what CLAUDE.md specifies
- Team size hints (e.g., "this is complex, use 3 engineers")

## Architecture

One flat team. Coordinator stays in **delegate mode** — a thin dispatcher that spawns
agents and monitors for completion. The **lead** handles all technical coordination
within the team.

```
Coordinator (you — delegate mode, spawns agents and monitors only)
  └── TeamCreate("focused-work")
       ├── task-lead       (plan mode → coordinates team after approval)
       ├── task-architect   (optional — designs approach, advises on technical decisions)
       ├── task-eng1        (spawned after plan approval)
       ├── task-eng2        (spawned after plan approval)
       └── task-qa          (optional — writes tests, validates behavior)
```

The lead's plan determines which roles to request. The lead can request **any role**
that fits the task — the list is open-ended. Some common examples:

| Role | Purpose | When to request |
|------|---------|-----------------|
| `task-eng{N}` | Implement a sub-task (owns specific files) | Parallelizable implementation work |
| `task-architect` | Design technical approach, review implementation | Complex systems with non-obvious design decisions |
| `task-qa` | Write tests, validate edge cases, check requirements | Tasks where correctness is critical |
| `task-pm` | Clarify requirements, write acceptance criteria | Ambiguous specs or user-facing features |
| `task-{custom}` | Whatever the task needs | Lead names it descriptively |

**Key constraints from the agent teams docs:**
- **No nested teams** — teammates cannot spawn their own teams or teammates.
- **Only the coordinator can spawn teammates** — the lead requests roles in their plan; the coordinator spawns them after approval.
- **One team per session** — everything in a single flat team.
- **All teammates can message each other** — the lead coordinates the team via SendMessage; engineers, architect, QA all communicate directly.

Verification subagents (code-reviewer, code-simplifier) are spawned via the Task tool
**without** `team_name` — they're one-shot workers that report results back to the lead.

## Tools Used

- **TeamCreate** / **TeamDelete** — create and tear down the team.
- **Task** — spawn teammates (with `team_name`) and verification subagents (without `team_name`).
- **TaskCreate** / **TaskList** / **TaskUpdate** — manage the shared task list.
- **SendMessage** — DMs between teammates, shutdown requests, plan approval responses.
- **ExitPlanMode** — lead submits their plan for coordinator approval.

## Workflow

### Phase 1: Setup (Coordinator)

1. **Detect worktree base directory** — check in priority order:
   ```bash
   # 1. Check for existing directories (.worktrees takes priority)
   ls -d .worktrees 2>/dev/null || ls -d worktrees 2>/dev/null

   # 2. If neither exists, check CLAUDE.md for a preference
   grep -i "worktree" CLAUDE.md 2>/dev/null

   # 3. If nothing found, ask the user where to put worktrees
   ```

   If creating a new project-local directory, verify it's in `.gitignore`:
   ```bash
   git check-ignore -q worktrees 2>/dev/null
   # If not ignored, add to .gitignore before proceeding
   ```

2. **Detect the base branch and ensure it's up to date:**
   ```bash
   BASE_BRANCH=$(git symbolic-ref refs/remotes/origin/HEAD 2>/dev/null | sed 's@^refs/remotes/origin/@@') || BASE_BRANCH="main"
   git checkout "$BASE_BRANCH" && git pull origin "$BASE_BRANCH"
   ```

3. **Create the worktree:**
   ```bash
   git worktree add {worktree-base}/{task-name} -b feature/{task-name} "$BASE_BRANCH"
   ```

4. **Run project setup in the worktree** — auto-detect from project files:
   ```bash
   cd {worktree-base}/{task-name}
   # Node.js
   [ -f package.json ] && npm install
   # Python (uv)
   [ -f pyproject.toml ] && uv sync --all-extras
   # Python (pip)
   [ -f requirements.txt ] && pip install -r requirements.txt
   # Rust
   [ -f Cargo.toml ] && cargo build
   # Go
   [ -f go.mod ] && go mod download
   ```

5. **Create the team:**
   ```
   TeamCreate(team_name="focused-work", description="Focused task: {task-name}")
   ```

6. **Create the task** in the shared task list:
   ```
   TaskCreate(subject="Implement {task-name}", description="{full task description}", activeForm="Implementing {task-name}")
   ```

### Phase 2: Dispatch Lead (Coordinator)

Switch to **delegate mode** (Shift+Tab) so you stay focused on coordination and don't
start implementing the task yourself.

Spawn the **lead** into the team in **plan mode** — they must submit a plan for your
approval before implementing.

**Task tool settings:**
- `subagent_type`: `"general-purpose"`
- `mode`: `"plan"`
- `team_name`: `"focused-work"`
- `name`: `"task-lead"`

**The lead prompt MUST include:**

1. The working directory constraint (with the actual resolved path):
   ```
   ## CRITICAL: Working directory
   You MUST work ONLY in {worktree-base}/{task-name}/. This is an isolated git
   worktree already on branch feature/{task-name}.

   ALL file paths must be under {worktree-base}/{task-name}/.
   Do NOT touch the main working directory.
   The branch is already created. Do NOT run git checkout or git branch.
   ```

2. The task description from the user.

3. Any user-provided quality check overrides.

4. The coordinator's name for messaging back:
   ```
   ## Coordinator
   The coordinator's name is "{coordinator-name}". Message them via SendMessage when
   you complete your PR or need help. Your task has already been assigned to you in
   the shared task list — update its status to in_progress when you start building,
   and completed when your PR is created.
   ```
   To find your own name for this placeholder, read the team config at
   `~/.claude/teams/focused-work/config.json` after creating the team.

5. The full **Lead Playbook** (below) — paste it into the lead's prompt.

After spawning the lead, use `TaskUpdate` to assign the task to `task-lead`.

### Phase 3: Approve Plan and Spawn Team (Coordinator)

The lead starts in **plan mode** — they orient, read the codebase, and submit a plan via
`ExitPlanMode`. You receive a `plan_approval_request` message.

Your job is to **approve and spawn**, not to redesign the approach:
1. Skim the plan for the staffing request (roles, file ownership, sub-tasks)
2. **Approve** via `SendMessage(type="plan_approval_response", recipient="task-lead", request_id="{id}", approve=true)`
3. Only **reject** if the plan is clearly incomplete or misunderstands the user's request

**After approving**, spawn every role the lead requested. For each teammate:
- `subagent_type`: `"general-purpose"`
- `mode`: `"bypassPermissions"`
- `team_name`: `"focused-work"`
- `name`: use the role name from the lead's plan (e.g., `"task-eng1"`, `"task-architect"`, `"task-qa"`)

Each teammate's prompt MUST include:
- The same worktree path constraint as the lead
- Their specific role and sub-task (from the lead's plan)
- Quality standards from the lead's plan
- Which files they own (to avoid conflicts with other teammates)
- Their lead's name: `"Message task-lead for coordination, questions, or file access conflicts."`

Also create TaskCreate entries for each teammate's sub-task and assign them.
The **lead manages the team from here** — you just monitor for the final PR.

### Phase 4: Monitor (Coordinator)

After approving the plan and spawning the team, the lead coordinates and reports back:

1. **Task list** — Use `TaskList` to check progress. All teammates update status as they work.
2. **Messages** — The lead sends you a message when the PR is created or when they need help. Delivered automatically.
3. **Shutdown** — Once the lead reports the PR, shut down all teammates then the lead:
   ```
   # Shut down all other teammates first
   SendMessage(type="shutdown_request", recipient="task-eng1")
   SendMessage(type="shutdown_request", recipient="task-architect")
   SendMessage(type="shutdown_request", recipient="task-qa")
   # Shut down lead last
   SendMessage(type="shutdown_request", recipient="task-lead")
   ```

### Phase 5: Cleanup (Coordinator)

Once all teammates have shut down (`TeamDelete` will fail if any are still active):
1. If a teammate rejects a shutdown request, wait for them to go idle and retry.
   If they persistently reject, message them explaining the work is complete.
2. `TeamDelete` to remove the team and its task list.
3. Remove the worktree:
   ```bash
   git worktree remove {worktree-base}/{task-name}
   # If removal fails due to uncommitted changes:
   git worktree remove --force {worktree-base}/{task-name}
   ```
4. Present the PR URL to the user.

---

## Lead Playbook

**Paste this entire section into the lead's prompt.** It defines how the lead operates inside the worktree.

### Step 1: Orient

Read the project's CLAUDE.md (if it exists) in your worktree root. Extract:
- **Quality commands** — lint, typecheck, test commands (e.g., `uv run ruff check`, `uv run pyright`, `uv run pytest`)
- **Coding standards** — line length, import style, type annotation policy, suppression rules
- **Project structure** — where code lives, key patterns, naming conventions

These will be used in verification gates and included in your plan.

### Step 2: Plan and Submit

You start in **plan mode** — you can read files and explore, but cannot edit.

Analyze your assigned task:
- Read relevant existing code to understand the codebase
- Break the task into sub-tasks if it has independent parts
- Identify files that will be created or modified
- Note any dependencies between sub-tasks

Write your plan and include a **staffing request**. You can request any combination
of roles — the coordinator will spawn them for you:

| Complexity | Request | When |
|------------|---------|------|
| Simple | "No teammates needed — I'll implement this solo." | Single-file change, straightforward logic |
| Medium | "Requesting 1-2 engineers for {sub-tasks}." | Multi-file but can split cleanly |
| Complex | "Requesting: architect (design data model), eng1 (API layer, owns {files}), eng2 (service layer, owns {files}), qa (integration tests)." | Large task with distinct concerns |

Common roles (not exhaustive — request whatever the task needs):
- **Engineers** (`task-eng{N}`) — implement sub-tasks, each owns specific files
- **Architect** (`task-architect`) — design technical approach, review implementation decisions
- **QA** (`task-qa`) — write tests, validate edge cases, check requirements against spec
- **PM** (`task-pm`) — clarify requirements, write acceptance criteria, validate user-facing behavior
- **Any other role** — name it descriptively (e.g., `task-db-specialist`, `task-security-reviewer`)

Include file ownership in the plan so the coordinator can relay it to each teammate.

Call `ExitPlanMode` to submit your plan. Wait for the coordinator to approve (and spawn engineers if requested) before proceeding.

### Step 3: Build

After plan approval:

**If solo:** Implement directly. Follow the project's coding standards from Step 1.

**If you have teammates:** You are the coordinator within the worktree. Use SendMessage
and TaskList to manage the team:
- Direct engineers to their sub-tasks and clarify priorities
- Consult the architect on design decisions (if you have one)
- Ask QA to write tests early so engineers can validate against them (if you have one)
- All teammates can message each other directly — encourage it
- Monitor TaskList for sub-task completion
- Resolve any file ownership conflicts immediately

**Git workflow:** Teammates edit files but do NOT commit. You (the lead) make the final
commit in Step 5 after all verification gates pass. This avoids sequencing issues with
multiple commits from different agents in the same worktree.

**IMPORTANT:** Wait for ALL teammate sub-tasks to be marked completed in TaskList
before starting Step 4. Running verification on partially-complete work wastes time
and produces misleading results.

### Step 4: Verify (Four Gates)

**All four gates must pass before creating a PR. Do not skip any gate.**

**Gate 1: Project Quality Checks**

Run every quality command found in CLAUDE.md:
```bash
# These are EXAMPLES — use whatever the project's CLAUDE.md specifies
{lint command}      # e.g., uv run ruff check src/
{typecheck command}  # e.g., uv run pyright src/
{test command}       # e.g., uv run pytest
```

Fix any failures before proceeding. If the user provided additional quality check overrides, run those too.

**Gate 2: Code Review**

Spawn a code-reviewer subagent (Task tool, **no** `team_name`) on your changes:
```
Task(
  subagent_type="superpowers:code-reviewer",
  prompt="Review the changes on branch feature/{task-name} in {worktree-path}.
    What was implemented: {task description}
    Requirements: {original task description from user}
    BASE_SHA: {sha before your changes}
    HEAD_SHA: {current HEAD}
    Description: {brief summary of changes made}"
)
```

If the `superpowers:code-reviewer` subagent type is unavailable, review the diff yourself.

Act on feedback:
- **Critical/Important issues** — fix immediately, re-run Gate 1
- **Minor issues** — fix if straightforward, otherwise note in PR description

**Gate 3: Code Simplification**

Spawn a code-simplifier subagent (Task tool, **no** `team_name`) on modified files:
```
Task(
  subagent_type="code-simplifier:code-simplifier",
  prompt="Simplify and refine the recently modified code in {worktree-path}
    for clarity, consistency, and maintainability. Focus only on files
    changed on this branch. Preserve all functionality."
)
```

If the `code-simplifier:code-simplifier` subagent type is unavailable, review the code for simplification opportunities yourself.

If the simplifier makes changes, re-run Gate 1 to confirm nothing broke.

**Gate 4: Requirements Verification**

Re-read your original task description. For each requirement:
- Verify with concrete evidence (test output, file contents, command output)
- Do NOT claim completion based on memory or confidence — run the verification
- If any requirement is not met, go back and implement it

Only proceed when ALL gates pass.

### Step 5: Deliver

```bash
cd {worktree-path}
git add -A
git commit -m "$(cat <<'EOF'
{commit message — summarize what was built and why}

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
git push -u origin feature/{task-name}
gh pr create --title "{PR title}" --body "$(cat <<'EOF'
## Summary
{1-3 bullet points}

## Verification
- [ ] Quality checks passed (lint, typecheck, tests)
- [ ] Code review — {summary of findings and fixes}
- [ ] Code simplification — {summary of changes}
- [ ] Requirements verified against original task

## Test plan
{how to test the changes}

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

Report the PR URL back to the coordinator (use the name from your prompt):
```
SendMessage(type="message", recipient="{coordinator-name}", content="PR created: {url}", summary="PR ready for {task-name}")
```

Then mark your task as completed via TaskUpdate and go idle. The coordinator will shut you down.

---

## Rules

- **Never** let multiple agents edit the same files — the lead must assign clear file ownership
- **Always** create the worktree BEFORE spawning any teammates
- **Always** run all four verification gates before creating a PR
- **Only the coordinator spawns teammates** — the lead requests roles in their plan
- **Coordinator is a thin dispatcher** — approve plans and spawn agents, don't redesign the approach
- The lead must read CLAUDE.md — do not hardcode project-specific commands in this skill
- Teammates may ignore shutdown requests while mid-turn — retry when they go idle
- Shut down all other teammates BEFORE the lead (the lead may need to do final integration)
- Teammates MUST coordinate file access via SendMessage

## Example Invocation

User: "Implement hybrid search with BM25+vector scoring and RRF fusion"

-> Detects `worktrees/` directory, creates worktree `worktrees/hybrid-search` on branch `feature/hybrid-search`.
-> Creates team, spawns task-lead (plan mode).
-> Lead orients, submits plan requesting: architect (design ranking model), eng1 (BM25 indexing), eng2 (RRF fusion scoring).
-> Coordinator approves plan, spawns task-architect, task-eng1, task-eng2.
-> Lead coordinates team: architect designs ranking model, engineers implement, lead integrates.
-> Lead runs 4 verification gates, creates PR.
-> Coordinator shuts down architect + engineers, then lead. Removes worktree, presents PR URL.
