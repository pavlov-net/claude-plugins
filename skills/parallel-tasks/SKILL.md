---
name: parallel-tasks
description: Run multiple implementation tasks in parallel using git worktrees and a team of agents. Each task gets an isolated worktree with a captain (and optionally engineers), verification gates (code review, simplification, requirements check), and its own PR. Use when you have 2+ independent tasks that can run simultaneously.
user_invocable: true
---

# Parallel Task Execution with Worktrees

Run multiple independent implementation tasks in parallel. Each task gets:
- Its own git worktree (isolated working directory)
- Its own feature branch
- A **captain** who autonomously plans, builds, and coordinates their worktree team
- Optional **engineer** teammates for complex tasks (captain requests them)
- Verification gates (lint/typecheck/test, code review, code simplification, requirements check)
- Its own PR when complete

## Input

The user will provide a list of tasks. Each task needs:
- A short name (used for branch/worktree naming, e.g., `item-2-mailing-list-tiers`)
- A description of what to implement

Optional overrides the user may provide:
- Extra quality check commands beyond what CLAUDE.md specifies
- Team size hints (e.g., "this one is complex, use 3 engineers")

## Architecture

One flat team. All agents (captains and engineers) are teammates in the same team.
Naming convention groups them by worktree:

```
Coordinator (you — delegate mode, thin dispatcher only)
  └── TeamCreate("parallel-work")
       ├── task1-captain                ← worktree 1 (autonomous)
       ├── task1-eng1                   ← spawned on captain's request
       ├── task1-eng2                   ← spawned on captain's request
       ├── task2-captain                ← worktree 2
       ├── task2-eng1
       └── task3-captain                ← worktree 3 (solo, no engineers)
```

**Key constraints from the agent teams docs:**
- **No nested teams** — teammates cannot spawn their own teams or teammates.
- **Only the coordinator can spawn teammates** — captains message the coordinator with staffing requests; the coordinator spawns them.
- **One team per session** — everything in a single flat team.
- **All teammates can message each other** — engineers in the same worktree coordinate directly via SendMessage (critical for file ownership).

Verification subagents (code-reviewer, code-simplifier) are spawned via the Task tool
**without** `team_name` — they're one-shot workers that report results back to the captain.

## Tools Used

- **TeamCreate** / **TeamDelete** — create and tear down the single team.
- **Task** — spawn teammates (with `team_name`) and verification subagents (without `team_name`).
- **TaskCreate** / **TaskList** / **TaskUpdate** — manage the shared task list.
- **SendMessage** — DMs between any teammates, shutdown requests, staffing requests.

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

3. **Create worktrees** — one per task, all branching from the base branch:
   ```bash
   git worktree add {worktree-base}/{task-name} -b feature/{task-name} "$BASE_BRANCH"
   ```

4. **Create the team:**
   ```
   TeamCreate(team_name="parallel-work", description="Parallel task execution across {N} worktrees")
   ```

5. **Create a task for each worktree** in the shared task list:
   ```
   TaskCreate(subject="Implement {task-name}", description="{full task description}", activeForm="Implementing {task-name}")
   ```

### Phase 2: Dispatch Captains (Coordinator)

Switch to **delegate mode** (Shift+Tab) so you stay focused on coordination and don't
start implementing tasks yourself.

Spawn one **captain** per worktree. Captains are autonomous — they plan, build, and
deliver without needing coordinator approval.

**Task tool settings for each captain:**
- `subagent_type`: `"general-purpose"`
- `mode`: `"bypassPermissions"`
- `team_name`: `"parallel-work"`
- `name`: `"{task-name}-captain"`

**Every captain prompt MUST include:**

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
   The coordinator's name is "{coordinator-name}". Message them via SendMessage to:
   - Request teammates (engineers, etc.) — include role names, sub-tasks, and file ownership
   - Report your PR when complete
   - Ask for help if blocked

   Your task has already been assigned to you in the shared task list — update its
   status to in_progress when you start building, and completed when your PR is created.
   ```
   To find your own name for this placeholder, read the team config at
   `~/.claude/teams/parallel-work/config.json` after creating the team.

5. The full **Captain Playbook** (below) — paste it into each captain's prompt.

After spawning all captains, use `TaskUpdate` to assign each task to the corresponding captain.

### Phase 3: Handle Staffing Requests (Coordinator)

Captains are autonomous — they orient, plan, and may start building immediately.
If a captain needs teammates, they'll message you with a staffing request.

When you receive a staffing request, spawn the requested roles:
- `subagent_type`: `"general-purpose"`
- `mode`: `"bypassPermissions"`
- `team_name`: `"parallel-work"`
- `name`: `"{task-name}-eng1"`, `"{task-name}-eng2"`, etc.

Each teammate's prompt MUST include:
- The same worktree path constraint as the captain
- Their specific sub-task (from the captain's request)
- Which files they own (to avoid conflicts)
- Their captain's name for coordination: `"Message {task-name}-captain if you have questions or need to coordinate file access."`

Also create TaskCreate entries for each teammate's sub-task and assign them.

### Phase 4: Monitor (Coordinator)

Captains implement and report back:

1. **Task list** — Use `TaskList` to see all tasks at a glance. Captains and engineers update status as they progress.
2. **Messages** — Captains send you messages when they complete (with PR URLs) or need help. Delivered automatically.
3. **Shutdown** — As each captain reports a PR, shut down the captain's team:
   ```
   SendMessage(type="shutdown_request", recipient="{task-name}-eng1")
   SendMessage(type="shutdown_request", recipient="{task-name}-eng2")
   SendMessage(type="shutdown_request", recipient="{task-name}-captain")
   ```
4. **Track progress** in a summary table:
   ```
   | Task | Captain | Engineers | Status | PR |
   |------|---------|-----------|--------|----|
   | item-4-hybrid-search | task4-captain | task4-eng1, task4-eng2 | PR created | #42 |
   | item-7-merge-suggestions | task7-captain | (solo) | In progress | — |
   ```

### Phase 5: Cleanup (Coordinator)

Once all teammates have shut down (`TeamDelete` will fail if any are still active):
1. If a teammate rejects a shutdown request, wait for them to go idle and retry.
   If they persistently reject, message them explaining the work is complete.
2. `TeamDelete` to remove the team and its task list.
3. Remove worktrees:
   ```bash
   git worktree list
   git worktree remove {worktree-base}/{task-name}  # for each
   # If removal fails due to uncommitted changes:
   git worktree remove --force {worktree-base}/{task-name}
   ```
4. Present final summary with all PR URLs.

---

## Captain Playbook

**Paste this entire section into each captain's prompt.** It defines how a captain operates inside their worktree.

### Step 1: Orient

**Run project setup** — auto-detect from project files in your worktree:
```bash
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

**Read the project's CLAUDE.md** (if it exists) in your worktree root. Extract:
- **Quality commands** — lint, typecheck, test commands (e.g., `uv run ruff check`, `uv run pyright`, `uv run pytest`)
- **Coding standards** — line length, import style, type annotation policy, suppression rules
- **Project structure** — where code lives, key patterns, naming conventions

These will be used in verification gates and in your plan.

### Step 2: Plan

Analyze your assigned task:
- Read relevant existing code to understand the codebase
- Break the task into sub-tasks if it has independent parts
- Identify files that will be created or modified
- Note any dependencies between sub-tasks

Decide on staffing:

| Complexity | Action | When |
|------------|--------|------|
| Simple | Proceed solo — skip to Step 3. | Single-file change, straightforward logic |
| Medium | Message coordinator requesting 1 engineer. | Multi-file but can split cleanly |
| Complex | Message coordinator requesting 2-3 engineers with file ownership. | Independent sub-tasks that parallelize |

**If you need teammates**, message the coordinator with your staffing request. Include
role names, sub-tasks, and file ownership for each. Then wait for the coordinator to
confirm they've been spawned before proceeding.

```
SendMessage(type="message", recipient="{coordinator-name}",
  content="Staffing request for {task-name}:
    - {task-name}-eng1: {sub-task}, owns {files}
    - {task-name}-eng2: {sub-task}, owns {files}",
  summary="Staffing request for {task-name}")
```

**If solo**, proceed directly to Step 3.

### Step 3: Build

**If solo:** Implement directly. Follow the project's coding standards from Step 1.

**If you have engineers:** Coordinate via SendMessage and TaskList:
- Message engineers with clarifications or priority changes
- Engineers can message you and each other directly
- Monitor TaskList for sub-task completion
- Resolve any file ownership conflicts immediately

**Git workflow:** Engineers edit files but do NOT commit. You (the captain) make the final
commit in Step 5 after all verification gates pass. This avoids sequencing issues with
multiple commits from different agents in the same worktree.

**IMPORTANT:** Wait for ALL engineer sub-tasks to be marked completed in TaskList
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

- **Never** let multiple agents edit the same files — captains must assign clear file ownership
- **Always** create worktrees BEFORE spawning any teammates
- **Always** run all four verification gates before creating a PR
- **Only the coordinator spawns teammates** — captains message the coordinator with staffing requests
- **Coordinator is a thin dispatcher** — spawn agents, handle staffing requests, monitor for PRs
- **Captains are autonomous** — they plan, build, and verify without needing coordinator approval
- Captains must read CLAUDE.md — do not hardcode project-specific commands in this skill
- Teammates may ignore shutdown requests while mid-turn — retry when they go idle
- Shut down engineers BEFORE their captain (captains may need to do final integration)
- Engineers in the same worktree MUST coordinate file access via SendMessage

## Example Invocation

User: "Run these 3 tasks in parallel:
1. item-4-hybrid-search: Add BM25+vector hybrid search with RRF fusion scoring
2. item-7-merge-suggestions: Implement merge suggestion system with confidence thresholds
3. item-14-halfvec: Migrate embeddings from vector to halfvec for 50% storage reduction"

-> Detects `worktrees/` directory, creates 3 worktrees, creates team, spawns 3 captains.
-> Captains orient, plan, and request engineers as needed. Coordinator spawns requested engineers.
-> Captains coordinate their teams, run 4 verification gates, create PRs.
-> Coordinator collects 3 PR URLs, shuts down all teammates, cleans up, presents summary.
