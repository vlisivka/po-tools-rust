# Plan: Implement `benchmark` mode for `translate` command

## Overview
Instead of a separate command, we will add a `--benchmark` flag to the existing `translate` command. This mode allows users to compare AI-generated translations against an existing "gold standard" (human translation) within the same PO file without overwriting the human work.

## Goals
1.  **Comparison**: Compare AI-generated `msgstr` with existing human `msgstr`.
2.  **Metrics**: Use **Normalized Levenshtein distance** (via `strsim`) to calculate a similarity score (0.0 to 1.0).
3.  **Output (STDOUT)**: 
    - A PO file where the original human `msgstr` is preserved.
    - A new comment is added to each message containing the score and the AI's attempt.
    - Format: `# Benchmark: Score=0.85 | AI: machine translation\nmsgid "..." \nmsgstr "human translation"`
4.  **Output (STDERR)**:
    - Per-message scores for real-time monitoring: `[Score: 0.XXXX]`
    - A final summary report with the average score for the entire file.
5.  **Flexibility**: Support all existing `translate` flags (`--model`, `--dictionary`, etc.) to allow testing different configurations.

## Implementation Steps

### 1. Update Data Structures ✅
- [x] Add `benchmark: bool` to `TranslateConfig` in `src/command_translate_and_print.rs`.

### 2. Update CLI Parsing ✅
- [x] Add `--benchmark` option parsing in `command_translate_and_print`.
- [x] Ensure the flag is passed through to the configuration.

### 3. Core Logic Implementation ✅
- [x] Create `translate_benchmark_message()` — processes a single message in benchmark mode:
    - Get AI translation via `translate_and_get_ai_msgstr()`.
    - Calculate `score = normalized_levenshtein(human_msgstr, ai_msgstr)`.
    - Print `[Score: 0.XX]` to `ctx.err`.
    - Create comment `# Benchmark: Score=0.XX | AI: "<ai_msgstr>"`.
    - Write original message (with human `msgstr` and new comment) to `ctx.out`.
- [x] Create `translate_and_get_ai_msgstr()` — helper that runs the full AI translation pipeline and returns only the msgstr string.
- [x] Modify `translate_and_print()`:
    - In benchmark mode, process all non-header messages through `translate_benchmark_message()`.
    - Track score sum and count for the summary.

### 4. Summary Reporting ✅
- [x] At the end of `translate_and_print`, print average score to `ctx.err`.

### 5. Help Text ✅
- [x] Add `--benchmark` description to `help_translate()`.

### 6. Testing ✅
- [x] `test_benchmark_basic` — verifies human translation preserved, benchmark comment added, scores in stderr.
- [x] `test_benchmark_header_passthrough` — headers pass through unchanged.
- [x] `test_benchmark_ai_failure` — graceful handling when AI returns unparseable output.

## Acceptance Criteria
- [x] The command `po-tools translate --benchmark file.po -m model` works as expected.
- [x] The output file preserves human translations but adds AI context in comments.
- [x] The summary score is printed to `stderr`.
- [x] Users can redirect stdout to a file and still see scores in the terminal (via stderr).
