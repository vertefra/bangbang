---
name: git-worktree-new
description: Creates a new git worktree on a fresh branch under a sibling ../branches folder, after syncing main, and copies local-only files (e.g. .env). Use only when the user explicitly asks to create a worktree, a parallel checkout, or a branch in ../branches — not for routine git operations.
---

# Git Worktree (New Branch)

**On-demand only.** Do not apply this skill unless the user asked for a new worktree or parallel branch checkout.

Assume the **repository root** is `$(git rev-parse --show-toplevel)`. All paths below are relative to that root unless noted.

## Preconditions

- If the working tree has **uncommitted changes** on the current branch, stop and ask: stash, commit, or discard before switching to `main`.
- If `main` does not exist locally but `origin/main` does: `git fetch origin` and `git checkout -B main origin/main` (or the project’s default branch name — confirm if not `main`).

## Workflow

### 1. Sync `main`

From repo root:

```bash
git checkout main
git pull origin main
```

If pull fails (merge conflicts, network), fix or report and **do not** create the worktree until `main` is clean and up to date.

### 2. Branch name

Ask the user for the **new branch name**. Reject empty names and path separators (`/`, `\`). Prefer `kebab-case` or the project’s branch naming convention if known.

### 3. Ensure `../branches` exists

From repo root:

```bash
mkdir -p ../branches
```

`../branches` is the parent of the repo folder, next to the clone — e.g. `repo` → `../branches/<branchname>`.

### 4. Add worktree + create branch

From repo root, with `main` checked out in the primary worktree:

```bash
git worktree add ../branches/<branchname> -b <branchname>
```

If the branch already exists: `git worktree add ../branches/<branchname> <branchname>` (no `-b`).

If the path is already occupied or Git reports an error, stop, show the message, and do not overwrite.

### 5. Copy local-only / untracked files

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

### 6. Verify

```bash
git worktree list
```

Confirm the new entry points at `../branches/<branchname>` and the branch name is correct.

## Cleanup (optional reference)

Remove a worktree when finished:

```bash
git worktree remove ../branches/<branchname>
git branch -d <branchname>   # after merge, if desired
```

## Notes

- `../branches` is relative to **repo root**, not the current shell directory — `cd` to `$(git rev-parse --show-toplevel)` first if needed.
- Never commit copied `.env` content into git; only ensure the new worktree has the same local files for running the project.
