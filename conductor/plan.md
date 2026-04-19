# Objective

Resolve merge conflicts in branch `jules-17327648634414670190-77358543` resulting from a merge with `origin/main`. Prioritize the current branch's removal of supply chain commands and additions of pipeline examples.

# Key Files & Context

*   **Source Files:**
    *   `src/main.rs`
    *   `src/cli/mod.rs`
    *   `src/cli/commands/mod.rs`
*   **Documentation Files:**
    *   `docs/2026-04-11-prd.md`
    *   `docs/2026-04-11-man-page.md`
    *   `docs/cli/rulette.md`

# Implementation Steps

1.  **Resolve Source Conflicts:**
    *   `src/main.rs`: Revert to the `HEAD` state, which removes the `Commands::Fetch`, `Commands::Lock`, `Commands::Verify`, `Commands::Archive`, and `Commands::Unarchive` match arms.
    *   `src/cli/mod.rs`: Revert to the `HEAD` state, removing the corresponding variants from the `Commands` enum.
    *   `src/cli/commands/mod.rs`: Resolve by keeping only the `HEAD` state, which excludes `pub mod unarchive;` and `pub mod verify;` and other supply chain modules.
2.  **Resolve Documentation Conflicts:**
    *   `docs/2026-04-11-prd.md`: Resolve by keeping the `HEAD` section "What survives if Agent Skills wins" which emphasizes Transform pipelines over Supply chain integrity, as the supply chain commands were removed.
    *   `docs/2026-04-11-man-page.md`: Resolve by keeping the `HEAD` additions for "EXAMPLES AND PIPELINES" and removing the `fetch` command reference from `origin/main`.
    *   `docs/cli/rulette.md`: Resolve by removing the supply chain commands from the overview and detailed sections, keeping only the commands present in `HEAD` (parse, emit, convert, inspect, schema, transform).
3.  **Finalize Merge:**
    *   Run `cargo fmt` to ensure no formatting issues were introduced.
    *   Stage all resolved files (`git add`).
    *   Commit the merge resolution.
    *   Push the resolved branch to origin to trigger CI.

# Verification & Testing

*   Run `cargo build` to ensure the project compiles successfully after conflict resolution.
*   Verify CI runs after the push.