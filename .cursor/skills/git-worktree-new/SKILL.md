---
name: git-worktree-new
description: Creates a new git worktree on a fresh branch under ../branches after syncing main, creates a related GitHub issue (linked by naming and cross-references), copies local-only files (e.g. .env). Use only when the user explicitly asks to create a worktree, a parallel checkout, or a branch in ../branches — not for routine git operations.
---

# Git Worktree (New Branch + GitHub Issue)

**On-demand only.** Do not apply this skill unless the user asked for a new worktree or parallel branch checkout.

Assume the **repository root** is `$(git rev-parse --show-toplevel)`. All paths below are relative to that root unless noted.

## Preconditions

- If the working tree has **uncommitted changes** on the current branch, stop and ask: stash, commit, or discard before switching to `main`.
- If `main` does not exist locally but `origin/main` does: `git fetch origin` and `git checkout -B main origin/main` (or the project’s default branch name — confirm if not `main`).
- **GitHub:** If `origin` points at `github.com`, plan to create an issue via the GitHub MCP (`issue_write`). If the user explicitly wants a **local-only** worktree (no GitHub), skip the GitHub sections and use **Branch name (local-only)** below.

## Resolve GitHub `owner` / `repo`

From repo root:

```bash
git remote get-url origin
```

Parse `owner` and `repo` (strip `.git`). Examples:

- `https://github.com/acme/myproject.git` → `owner=acme`, `repo=myproject`
- `git@github.com:acme/myproject.git` → same

If the host is not `github.com`, skip GitHub steps unless the user provides another forge workflow.

## Workflow

### 1. Sync `main`

From repo root:

```bash
git checkout main
git pull origin main
```

If pull fails (merge conflicts, network), fix or report and **do not** create the worktree until `main` is clean and up to date.

### 2. GitHub issue (related work item)

**Goal:** One open issue describes the work; the **branch name embeds the issue number** so commits/PRs can reference `Fixes #N` / `Refs #N`, and the issue documents the branch and worktree path.

1. **Avoid duplicates:** Call `search_issues` on `owner`/`repo` with keywords from the user’s planned title before creating.
2. **Create the issue:** Use `issue_write` with `method: "create"`, `owner`, `repo`, `title`, and `body`. The body should briefly state intent (user-supplied detail). You may add a line such as: `Branch and local worktree path will be added after creation.`
3. **Read the new issue number** from the tool result (or `issue_read` / `list_issues` if needed). Call it `N`.
4. **Derive the branch name** (must relate to the issue):

   - Build a **kebab-case slug** from the issue title: lowercase, spaces → `-`, remove characters unsafe for git branch names (`/`, `\`, `..`, control chars). Keep it reasonably short (e.g. ≤ 50 chars excluding the numeric prefix).
   - **Branch:** `N-<slug>`  
     Example: issue `47` titled “Fix door spawn at mumhome” → `47-fix-door-spawn-at-mumhome`.

   This pairs the branch to the issue number for cross-reference and PR linking.

5. **After the worktree exists** (step 4 below), **update the issue** with `issue_write` `method: "update"`, `issue_number: N`, and append to the body (or replace with a clear section):

   - `**Branch:** \`<branchname>\``
   - `**Worktree:** \`../branches/<branchname>\` (relative to repo root)`
   - Optional: `Push: \`git push -u origin <branchname>\` from the worktree when ready so the remote branch exists on GitHub.`

Opening a PR later with `Fixes #N` or `Closes #N` in the description completes the GitHub link.

### 3. Branch name (local-only)

If GitHub steps are skipped, ask the user for the **new branch name**. Reject empty names and path separators (`/`, `\`). Prefer `kebab-case` or the project’s convention.

### 4. Ensure `../branches` exists

From repo root:

```bash
mkdir -p ../branches
```

`../branches` is the parent of the repo folder, next to the clone — e.g. `repo` → `../branches/<branchname>`.

### 5. Add worktree + create branch

From repo root, with `main` checked out in the primary worktree, use the branch name from **§2** (GitHub) or **§3** (local-only):

```bash
git worktree add ../branches/<branchname> -b <branchname>
```

If the branch already exists: `git worktree add ../branches/<branchname> <branchname>` (no `-b`).

If the path is already occupied or Git reports an error, stop, show the message, and do not overwrite.

Then complete **§2 step 5** (update issue body with branch + worktree) when using GitHub.

### 6. Copy local-only / untracked files

The new worktree does **not** include ignored or untracked files from the original tree. Copy what the project needs:

**Always check and copy if present** (adjust list to match this repo):

- `.env`
- `.env.local`
- `.env.development.local` / `.env.production.local` (if used)

Example (run from **repo root**, bash):

```bash
WT="../branches/<branchname>"
for f in .env .env.local; do
  [ -f "$f" ] && cp -a "$f" "$WT/$f"
done
```

For **other** untracked (non-ignored) files the user cares about, list with `git status -u` from the original worktree and copy explicitly — do not bulk-copy unrelated scratch files unless the user asks.

**Gitignored secrets** (e.g. `.env`) never appear in `git status` as tracked; rely on the fixed list above and user confirmation for anything extra.

### 7. Verify

```bash
git worktree list
```

Confirm the new entry points at `../branches/<branchname>` and the branch name is correct.

### 8. Optional: publish branch to GitHub

From the new worktree (or repo root with `-C`):

```bash
git -C ../branches/<branchname> push -u origin <branchname>
```

Use this when the user wants the remote branch to exist for PRs; it does not replace creating/updating the issue in §2.

## Cleanup (optional reference)

Remove a worktree when finished:

```bash
git worktree remove ../branches/<branchname>
git branch -d <branchname>   # after merge, if desired
```

Close the related issue on GitHub when work is done (`issue_write` update `state` / `state_reason` as appropriate).

## Notes

- `../branches` is relative to **repo root**, not the current shell directory — `cd` to `$(git rev-parse --show-toplevel)` first if needed.
- Never commit copied `.env` content into git; only ensure the new worktree has the same local files for running the project.
- **Order matters:** create the GitHub issue first so `N` appears in the branch name; then worktree; then update the issue with the exact branch and path.
