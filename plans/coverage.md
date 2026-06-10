# Code Coverage Improvement Plan

## Current State

- **Total functions analyzed**: 79
- **Functions with 0% coverage**: 5 (`Dictionary::from_file`, `AiBackend::from_command_line`, `AiBackend::with_aichat_defaults`, `Keyword::fmt`, `AiBackend::new`)
- **Functions exceeding CRAP threshold (>30)**: 9/79

### Worst CRAP Scores (untested or hard to test)

| Function | File | CRAP | Coverage |
|---|---|---|---|
| `command_translate_and_print` | `src/command_translate_and_print.rs:16` | 590.6 | 20.0% |
| `main` | `src/main.rs:74` | 482.6 | 48.6% |
| `diff_by_str_and_print` | `src/command_diff_by_str_and_print.rs:11` | 172.8 | 21.8% |
| `Dictionary::from_file` | `src/dictionary.rs:34` | 110.0 | 0.0% |

### Low-Coverage Functions (0–25%)

| Function | File | CRAP | Coverage |
|---|---|---|---|
| `AiBackend::new` | `src/util.rs:28` | 1.0 | 0.0% |
| `AiBackend::from_command_line` | `src/util.rs:37` | 2.0 | 0.0% |
| `AiBackend::with_aichat_defaults` | `src/util.rs:51` | 2.0 | 0.0% |
| `Keyword::fmt` | `src/parser.rs:214` | 1.0 | 0.0% |
| `command_translate_and_print` | `src/command_translate_and_print.rs:16` | 590.6 | 20.0% |
| `main` | `src/main.rs:74` | 482.6 | 48.6% |
| `diff_by_str_and_print` | `src/command_diff_by_str_and_print.rs:11` | 172.8 | 21.8% |
| `Dictionary::from_file` | `src/dictionary.rs:34` | 110.0 | 0.0% |
| `command_review_files_and_print` | `src/command_review_files_and_print.rs:12` | 97.7 | 31.7% |
| `translate_and_print` | `src/command_translate_and_print.rs:231` | 37.2 | 94.4% |
| `review_files_and_print` | `src/command_review_files_and_print.rs:98` | 37.0 | 59.0% |
| `command_diff_by_id_and_print` | `src/command_print_added.rs:57` | 30.0 | 0.0% |

---

## Phase 1: Quick Wins — Zero-Coverage Functions (Priority: HIGH)

**Goal**: Get 5 functions from 0% to covered with minimal effort.

### Task 1.1: Cover `Keyword::fmt` (parser.rs:214)
- **Type**: Unit test
- **Effort**: ~5 minutes
- **Steps**:
  1. Add a test module to `parser.rs` or create a new test file
  2. Write an assertion that formats each `Keyword` variant and checks the output string
  3. Example:
     ```rust
     #[test]
     fn test_keyword_fmt() {
         assert_eq!(format!("{}", Keyword::Msgid), "msgid");
         assert_eq!(format!("{}", Keyword::Msgctxt), "msgctxt");
         assert_eq!(format!("{}", Keyword::Msgstr), "msgstr");
         assert_eq!(format!("{}", Keyword::MsgidPlural), "msgid_plural");
         assert_eq!(format!("{}", Keyword::MsgstrPlural(2)), "msgstr[N]");
     }
     ```

### Task 1.2: Cover `AiBackend::new` (util.rs:28)
- **Type**: Unit test
- **Effort**: ~5 minutes
- **Steps**:
  1. Add tests in `util.rs` that construct `AiBackend::new("echo", vec!["hello"])` and verify fields
  2. Example:
     ```rust
     #[test]
     fn test_ai_backend_new() {
         let backend = AiBackend::new("aichat".to_string(), vec!["-m".to_string(), "gpt-4".to_string()]);
         assert_eq!(backend.command, "aichat");
         assert_eq!(backend.args, vec!["-m", "gpt-4"]);
     }
     ```

### Task 1.3: Cover `AiBackend::from_command_line` (util.rs:37)
- **Type**: Unit test
- **Effort**: ~5 minutes
- **Steps**:
  1. Add tests that parse command-line strings and verify parsing
  2. Example:
     ```rust
     #[test]
     fn test_from_command_line() {
         let backend = AiBackend::from_command_line("aichat -m gpt-4");
         assert_eq!(backend.command, "aichat");
         assert_eq!(backend.args, vec!["-m".to_string(), "gpt-4".to_string()]);
     }
     ```

### Task 1.4: Cover `AiBackend::with_aichat_defaults` (util.rs:51)
- **Type**: Unit test
- **Effort**: ~5 minutes
- **Steps**:
  1. Add tests that call the method and verify the constructed args vector
  2. Example:
     ```rust
     #[test]
     fn test_with_aichat_defaults() {
         let backend = AiBackend::with_aichat_defaults("ollama:gemma", "translate", None);
         assert_eq!(backend.command, "aichat");
         assert!(backend.args.contains(&"-r".to_string()));
         assert!(backend.args.contains(&"translate".to_string()));
     }
     ```

### Task 1.5: Cover `Dictionary::from_file` (dictionary.rs:34)
- **Type**: Integration test
- **Effort**: ~15 minutes
- **Steps**:
  1. Create a temporary TSV file with known dictionary entries
  2. Call `Dictionary::from_file` on that path
  3. Assert the parsed entries match expectations
  4. Example:
     ```rust
     #[test]
     fn test_from_file() {
         let temp_file = tempfile::NamedTempFile::new().unwrap();
         writeln!(temp_file.as_file(), "hello\tbonjour\nworld\tmonde").unwrap();
         let dict = Dictionary::from_file(temp_file.path()).unwrap();
         // Verify entries were loaded
     }
     ```

**Expected CRAP reduction**: Eliminates all 5 zero-coverage functions. CRAP scores for these will drop significantly since they'll be exercised by tests.

---

## Phase 2: High-CRAP Functions (Priority: MEDIUM)

**Goal**: Reduce CRAP scores for the top 3 worst functions.

### Task 2.1: Refactor `command_translate_and_print` (CRAP 590.6)
- **Location**: `src/command_translate_and_print.rs:16`
- **Current coverage**: 20.0%
- **Strategy**:
  1. Extract nested logic into smaller, independently testable functions
  2. Break the large match/conditional blocks into separate helper functions
  3. Add unit tests for each extracted function
  4. Target: Reduce CRAP score below 100

### Task 2.2: Improve `main` coverage (CRAP 482.6)
- **Location**: `src/main.rs:74`
- **Current coverage**: 48.6%
- **Strategy**:
  1. Extract CLI argument parsing into a separate testable function
  2. Add integration tests that invoke the binary with different argument combinations
  3. Ensure all subcommands (`translate`, `review`, `sort`, etc.) are tested via CLI

### Task 2.3: Test `diff_by_str_and_print` (CRAP 172.8)
- **Location**: `src/command_diff_by_str_and_print.rs:11`
- **Current coverage**: 21.8%
- **Strategy**:
  1. Add unit tests that exercise the match arms for different diff scenarios
  2. Create test PO files with known differences and verify output

---

## Phase 3: Medium-Coverage Functions (Priority: LOW)

**Goal**: Bring remaining low-coverage functions above 50%.

### Task 3.1: Cover `command_review_files_and_print` (31.7%)
- **Location**: `src/command_review_files_and_print.rs:12`
- **Strategy**: Add integration tests with sample PO files and review workflows

### Task 3.2: Cover `review_files_and_print` (59.0%)
- **Location**: `src/command_review_files_and_print.rs:98`
- **Strategy**: Add tests for the review logic paths not currently exercised

### Task 3.3: Cover `command_diff_by_id_and_print` (0.0%)
- **Location**: `src/command_print_added.rs:57`
- **Strategy**: Add unit test that exercises the diff-by-id printing path

---

## Phase 4: Validation

### Task 4.1: Run full coverage suite
- Execute `./crap.sh` to verify improvements
- Compare before/after CRAP scores and coverage percentages

### Task 4.2: Set coverage thresholds
- Consider adding a CI check that fails if CRAP scores exceed a threshold (e.g., 50)
- Consider adding a minimum coverage percentage gate (e.g., 80%)

---

## Summary of Expected Impact

| Phase | Functions Affected | Estimated CRAP Reduction | Est. Effort |
|---|---|---|---|
| Phase 1 | 5 | Eliminates all 0% functions | ~30 min |
| Phase 2 | 3 | Top 3 worst CRAP scores | ~2 hours |
| Phase 3 | 3 | Brings mid-range to >50% | ~1 hour |
| **Total** | **11** | **Significant overall improvement** | **~3.5 hours** |

---

## Notes

- The `Dictionary::from_file` function has a high CRAP score (110.0) even though it's untested, because the CRAP metric penalizes complexity without tests
- Several functions at 100% coverage already exist (e.g., `command_sort_and_print`, `PoMessage::is_header`, etc.) — these are good reference patterns
- The `ai_backend_mock` test already exercises part of `AiBackend::execute`, which is a useful template for additional backend tests
