# Plan: Fix Unused `err` Variables in Test Fixtures

## Problem Summary

Across all command test modules there are **74 instances** of `let mut err = Vec::new();` creating a throwaway `Vec<u8>` wired into `IoContext.err`. Only **2 tests** actually read back the contents of `err`; the remaining 72 silently discard any stderr output.

This is not a runtime bug — it means:
1. If a command writes unexpected errors to stderr during an otherwise-successful test, we won't notice.
2. Clippy/rustc don't warn because `err` *is* used (assigned into the struct field).

## Goal

Ensure that every test either **asserts on err contents** when it matters, or uses a clearly-marked no-op sink so reviewers can instantly see "this test doesn't care about stderr."

## Proposed Approach: Two-Part Fix

### Part 1 — Add a `TestIoContext` helper (one change)

Add a small test-only utility in the existing test infrastructure that makes the intent explicit. Rather than sprinkling dead `Vec`s everywhere, provide two constructors:

```rust
// In each module's #[cfg(test)] block, or better yet as a shared test helper:

/// Creates an IoContext where stderr is silently discarded (use when you don't care about err output).
fn ctx_with_sink() -> IoContext<'static> { ... }   // uses std::io::sink()-like Vec but named clearly

/// Creates an IoContext with separate capture buffers for both out and err.
fn ctx_with_capture() -> (IoContext<'_>, Vec<u8>, Vec<u8>) { ... }  // returns handles so caller can assert on both
```

**However**, since `IoContext` holds borrowed `&'a mut dyn Write`, returning owned captures is tricky with lifetimes. A simpler approach that avoids lifetime gymnastics:

### Part 1 (Revised) — Use a dedicated test fixture function per module

Add one helper function to each command's `mod tests`:

```rust
fn make_ctx() -> IoContext<'static> {
    // err goes into an unnamed vec — the name `_err` signals "we don't check this".
    let _err = Vec::new();          // ← will need Box/leak to get 'static lifetime though...
}
```

Actually, that won't work with borrows. The simplest correct approach is:

### Part 1 (Final) — No structural change; just add `#[allow(unused_variables)]` + doc comment

Since the `err` variable **must** exist to satisfy the struct fields and its value is intentionally discarded for most tests, we should:

1. Add a module-level `// Tests that don't assert on stderr use an unnamed capture vec.`
2. Rename `let mut err = Vec::new()` → `let mut _err = Vec::new();` in every test that doesn't read it. The underscore prefix signals "intentionally unused" and suppresses the implicit lint warning (there isn't one today, but this documents intent for future reviewers).
3. For the **2 tests** that DO check stderr, keep `err` named normally.

This is a documentation/convention fix rather than a structural change — minimal risk, zero behavior change.

### Part 2 — Audit: which tests SHOULD actually assert on err? (analysis pass)

Walk through every command and determine if there are code paths that write to `ctx.err`. For those commands, add at least one test with an `_err` → `err` assertion.

**Commands known to write to `ctx.err`:**
- `command_translate_and_print.rs` — writes debug output when `config.debug == true` (already tested in `test_translate_custom_prompt`)
- `command_diff_by_str_and_print.rs` — writes error message on insufficient args (already tested in `test_no_files`)

All other commands write **only** to `ctx.out`. No additional err assertions are needed for them.

## Concrete Changes by File

For each file, change every test fixture block from:
```rust
let mut out = Vec::new();
let mut err = Vec::new();
let mut ctx = IoContext {
    out: &mut out,
    err: &mut err,
};
```

To (for tests that don't read err):
```rust
let mut out = Vec::new();
let mut _err = Vec::new(); // intentionally not checked; this command writes only to stdout
let mut ctx = IoContext {
    out: &mut out,
    err: &mut _err,
};
```

And for the 2 tests that DO read err (`command_diff_by_str_and_print::test_no_files`, `command_translate_and_print::test_translate_custom_prompt`), keep as-is.

### Files to modify (~19 files)

| File | # of test fixtures to rename `err → _err` |
|------|-------------------------------------------|
| `src/command_check_symbols.rs` | 4 → `_err` (0 read err) |
| `src/command_compare_files_and_print.rs` | 3 → `_err` |
| `src/command_diff_by_str_and_print.rs` | 2 → `_err`, 1 stays `err` (`test_no_files`) |
| `src/command_erase_and_print.rs` | 2 → `_err` |
| `src/command_find_same_and_print.rs` | 3 → `_err` |
| `src/command_merge_and_print.rs` | 4 → `_err` |
| `src/command_parse_and_dump.rs` | 5 → `_err` |
| `src/command_print_added.rs` | 4 → `_err` |
| `src/command_print_plural.rs` | 3 → `_err` |
| `src/command_print_regular.rs` | 3 → `_err` |
| `src/command_print_translated.rs` | 3 → `_err` |
| `src/command_print_untranslated.rs` | 3 → `_err` |
| `src/command_print_with_context.rs` | 3 → `_err` |
| `src/command_print_with_unequal_linebreaks.rs` | 3 → `_err` |
| `src/command_print_with_word.rs` | 4 → `_err` |
| `src/command_print_with_wordstr.rs` | 3 → `_err` |
| `src/command_review_files_and_print.rs` | 2 → `_err`, 1 stays... actually none read err, so all 3 → `_err` (wait, need to double-check) |
| `src/command_sort.rs` | 3 → `_err` |
| `src/command_translate_and_print.rs` | 13 → `_err`, 1 stays `err` (`test_translate_custom_prompt`) |

Total: ~72 instances of `let mut err = Vec::new()` become `let mut _err = Vec::new();` with an inline comment. The remaining 2 keep the name `err`.

## Risk Assessment

- **Risk:** Very low. This is purely a naming convention change; no logic, assertions, or behavior changes.
- **Verification:** Run `cargo test` — all tests must still pass (they will, since only variable names change).
- **Future benefit:** Code reviewers can instantly tell whether a test intends to verify stderr output by scanning for `_err` vs `err`.

## Alternative Approaches Considered and Rejected

1. **Replace with `std::io::sink()`** — Requires changing `IoContext.err` type from `&'a mut dyn Write` to an owned writer or using `Box<dyn Write>`, which touches production code for a test-only concern. Not worth it.
2. **Create a test-only `NoopWrite` / discard wrapper type** — Over-engineered for what is fundamentally a documentation problem, not a functional one. The underscore rename communicates the same intent with zero new types.
3. **Add an `IoContext::test()` constructor that defaults err to sink** — Same lifetime issue; also hides the fact that err exists as a field. Better to be explicit per-test.
