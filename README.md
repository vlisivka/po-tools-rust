# PO-tools

A collection of tools for working with GNU Gettext PO files, written in Rust. Features AI-assisted translation, review, and various manipulation and filtering commands.

## Installation

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install) (cargo)
*   [aichat](https://github.com/sigoden/aichat) (for AI-related commands: `translate`, `review`, `filter`)

### Building from source

```bash
git clone https://github.com/vlisivka/po-tools-rust.git
cd po-tools-rust
cargo build --release
```

The binary will be available at `target/release/po-tools`.

## Usage

```bash
po-tools [GLOBAL_OPTIONS] COMMAND [ARGS]
```

### Global Options

*   `-c | --cases NUM` — Set the number of plural cases (default is 2).

### AI Commands (WIP)

These commands require `aichat` to be installed and configured.

*   `translate [OPTIONS] FILE` — Translate messages using AI.
    *   `-l | --language LANG` — Target language (default: "Ukrainian").
    *   `-m | --model MODEL` — AI model name.
    *   `--tm FILE` — Translation Memory file for fuzzy matching.
    *   `-d | --dictionary FILE` — TSV dictionary for terminology.
*   `review [OPTIONS] FILE1 FILE2...` — Compare translations and let AI pick/fix the best one.
*   `filter [OPTIONS] FILE` — Use AI to filter messages (e.g., finding specific translation issues).

### Manipulation & Comparison

*   `merge FILE1 FILE2` — Merge two files (FILE2 overwrites messages from FILE1).
*   `sort FILE` — Sort messages in lexical order (msgid).
*   `erase FILE` — Remove all translations (keeps only msgid keys).
*   `diff FILE1 FILE2` — Compare two files by `msgid`.
*   `diffstr FILE1 FILE2` — Compare two files by `msgstr`.
*   `same FILE1 FILE2...` — Print messages that are identical in all files.
*   `added FILE1 FILE2` — Print messages present in FILE2 but not in FILE1.
*   `removed FILE1 FILE2` — Print messages present in FILE1 but not in FILE2.

### Filtering & Inspection

*   `translated FILE` — Print only translated messages.
*   `untranslated FILE` — Print only untranslated messages.
*   `regular FILE` — Print regular messages (no context, no plural).
*   `plural FILE` — Print only plural messages.
*   `with-context FILE` — Print messages with `msgctxt`.
*   `with-word WORD FILE` — Print messages where `msgid` contains WORD.
*   `with-wordstr WORD FILE` — Print messages where `msgstr` contains WORD.
*   `with-unequal-linebreaks FILE` — Find messages where `\n` count in msgid and msgstr differs.
*   `check-symbols FILE` — Verify that special symbols (%, {}, etc.) match between msgid and msgstr.
*   `compare FILE1 FILE2...` — Show differences in translations side-by-side.

### Debugging

*   `parse FILE` — Parse and dump internal representation of the PO file.

## License

GPL 3.0
