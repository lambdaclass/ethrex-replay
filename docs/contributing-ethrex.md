## Contributing to Ethrex: Coordinating with Replay CI

Ethrex’s CI triggers a remote workflow in the `ethrex-replay` repository. By default, that remote call targets the `main` branch of `ethrex-replay`. When you push a branch in `ethrex`, your PR CI may fail due to a mismatch between your branch and whatever is currently on `ethrex-replay/main`.

This guide explains how to diagnose the failure and how to connect branches across both repos so CI runs against the correct versions.

### TL;DR Checklist
- Confirm the failing step is the remote `ethrex-replay` workflow.
- Check if `ethrex-replay/main` changed recently and introduced incompatible changes.
  - If yes and `ethrex/main` already has the matching fixes, update your `ethrex` branch from `main`.
  - If the fixes aren’t in `ethrex/main` yet, ensure they’re at least in the merge queue; otherwise ping the author of the `ethrex-replay` PR.
- If the failure is due to your changes, create the matching branch pairing across repos using the mapping rules below and open the accompanying PR.
- Once both PRs are approved, merge `ethrex-replay` first, then `ethrex`. Try to keep merge times close to minimize out-of-sync windows. If the order flips, it’s usually fine—just keep the window short.

---

## 1) Understand the Remote Workflow Link

- Ethrex CI calls a remote workflow in `ethrex-replay`.
- Default behavior: If `ethrex-replay` does not have a branch matching your `ethrex` branch (per the mapping rules below), CI runs against `ethrex-replay/main`.
- Consequence: Any recent change to `ethrex-replay/main` can break your `ethrex` branch’s CI even if you didn’t change anything related.

## 2) If Replay Broke First (Recent change on `ethrex-replay/main`)

1. Verify replay changed recently and your branch simply isn’t aligned.
2. Check whether `ethrex/main` already includes the corresponding compatibility changes.
   - If yes, update your branch:
     ```sh
     # inside the ethrex repository
     git fetch origin
     git checkout <your-ethrex-branch>
     git merge origin/main   # or: git rebase origin/main
     git push
     ```
   - Re-run your CI and confirm green.
3. If the needed changes are not yet in `ethrex/main`, confirm they are at least in the merge queue. If not, contact the author of the relevant `ethrex-replay` PR to coordinate.

## 3) If Your Ethrex Change Broke Replay

If your branch introduces changes that require updates in the other repository, connect them so CI runs against the correct pair. There are **only two valid mappings**:

| Starting repo | Branch you push | Linked branch in the other repo |
| ------------- | ---------------- | -------------------------------- |
| `ethrex`      | `<feature>`      | `ethrex-replay`: `ethrex/<feature>` |
| `ethrex`      | `replay/<feature>` | `ethrex-replay`: `<feature>` |
| `ethrex-replay` | `<feature>`    | `ethrex`: `replay/<feature>` |
| `ethrex-replay` | `ethrex/<feature>` | `ethrex`: `<feature>` |

If the linked branch from the table does not exist, the workflow falls back to `ethrex-replay/main`.

### 3a) Starting from Ethrex
- If your `ethrex` branch is plain (e.g. `feature/new-opcode`), create `ethrex/feature/new-opcode` in `ethrex-replay`.
- If your `ethrex` branch is already prefixed as `replay/<feature>`, create `<feature>` in `ethrex-replay`.
- Open a PR in `ethrex-replay` with the needed updates.
- Re-run the `ethrex` CI so it picks up the new replay branch.

### 3b) Starting from Replay
- If you start work in `ethrex-replay` on branch `<feature>`, create `replay/<feature>` in `ethrex`.
- If you start from `ethrex-replay` on `ethrex/<feature>`, create `<feature>` in `ethrex`.
- Update `Cargo.toml` (or other dependencies) as needed and open the companion `ethrex` PR.
- Trigger the relevant CI (either via push or workflow dispatch) to confirm the pair is linked.

## 4) Merge Order and Timing

- Preferred order: merge `ethrex-replay` PR first, then `ethrex` PR.
- Rationale: This reduces the chance that `ethrex` is using outdated replay code right after merge.
- Not critical if inverted: If something merges in the opposite order, it’s usually fine. Just minimize the time they are out of sync.

## 5) Troubleshooting & Common Pitfalls

- Branch naming mismatch: Only the two mappings listed above are recognized; anything else falls back to `ethrex-replay/main`.
- Branch not pushed or PR not open: Ensure the replay branch exists on the remote and a PR is open so others can review and the CI can reference it.
- Stale `ethrex` branch: If replay changed, make sure you’ve merged or rebased the latest `ethrex/main` into your feature branch.
- Merge queue confusion: If replay broke your CI and the fix isn’t visible in `ethrex/main`, check that it’s at least queued for merge; otherwise ping the replay PR author.

## 6) Communication

- When a replay change breaks `ethrex` CI and the corresponding `ethrex` updates aren’t yet available, reach out to the author of the `ethrex-replay` PR.
- Include links to the failing CI run, the relevant replay PR, and your `ethrex` PR to speed up context sharing.

## 7) Quick Reference

- Naming rules for connected branches: use only the two mappings in the table above.
- Default fallback: If no matching replay branch exists, CI uses `ethrex-replay/main`.
- Merge order: `ethrex-replay` first, then `ethrex` (order not critical, but minimize drift).
