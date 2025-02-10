use anyhow::{Result, bail};

mod parser;
use crate::parser::Parser;

mod command_sort;
use crate::command_sort::command_sort_and_print;

mod command_parse_and_dump;
use crate::command_parse_and_dump::command_parse_and_dump;

mod command_merge_and_print;
use crate::command_merge_and_print::command_merge_and_print;

mod command_print_added;
use crate::command_print_added::{command_print_added, command_print_removed, command_diff_by_id_and_print};

mod command_find_same_and_print;
use crate::command_find_same_and_print::command_find_same_and_print;

mod command_diff_by_str_and_print;
use crate::command_diff_by_str_and_print::command_diff_by_str_and_print;

mod command_print_translated;
use crate::command_print_translated::command_print_translated;

mod command_print_untranslated;
use crate::command_print_untranslated::command_print_untranslated;

mod command_print_regular;
use crate::command_print_regular::command_print_regular;

mod command_print_plural;
use crate::command_print_plural::command_print_plural;

mod command_print_with_context;
use crate::command_print_with_context::command_print_with_context;

mod command_print_with_word;
use crate::command_print_with_word::command_print_with_word;

mod command_print_with_wordstr;
use crate::command_print_with_wordstr::command_print_with_wordstr;

mod command_print_with_unequal_linebreaks;
use crate::command_print_with_unequal_linebreaks::command_print_with_unequal_linebreaks;

mod command_compare_files_and_print;
use crate::command_compare_files_and_print::command_compare_files_and_print;

mod command_translate_and_print;
use crate::command_translate_and_print::{command_translate_and_print, command_review_files_and_print};

mod command_check_symbols;
use crate::command_check_symbols::command_check_symbols;

fn main() -> Result<()> {
  // Options
  let mut number_of_plural_cases: Option<usize> = None;

  // Parse aruments
  let args = std::env::args().collect::<Vec<String>>();
  let tail = &args[1..].iter().map(|s| &s as &str).collect::<Vec<&str>>();
  let mut tail = &tail[..];

  // Parse options
  loop {
    match tail[..] {
      [ "-c", n, ..] | [ "--cases", n, ..] => {
        match n.parse::<usize>() {
          Ok(n) if n >= 1 && n < 10 => {
            number_of_plural_cases = Some(n);
            tail = &tail[2..];
          }
          _ => bail!("Invalid argument for -c | --cases option. Expected: number of plural cases between 1 and 9. Actual value: \"{n}\"."),
        }
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help();
        return Ok(());
      }
      [ "--", .. ] => {
        tail = &tail[1..];
        break;
      }
      [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
      _ => break,
    }
  }

  let parser = Parser::new(number_of_plural_cases);

  // Parse arguments
  match tail[..] {
    [ "parse", ref cmdline @ ..] => command_parse_and_dump(&parser, cmdline)?,
    [ "translate", ref cmdline @ ..] => command_translate_and_print(&parser, cmdline)?,
    [ "review", ref cmdline @ .. ] => command_review_files_and_print(&parser, cmdline)?,
    [ "compare", ref cmdline @ ..  ] => command_compare_files_and_print(&parser, cmdline)?,
    [ "sort", ref cmdline @ .. ] => command_sort_and_print(&parser, cmdline)?,
    [ "merge", ref cmdline @ .. ] => command_merge_and_print(&parser, cmdline)?,
    [ "diff", ref cmdline @ .. ] => command_diff_by_id_and_print(&parser, cmdline)?,
    [ "diffstr", ref cmdline @ .. ] => command_diff_by_str_and_print(&parser, cmdline)?,
    [ "same", ref cmdline @ .. ] => command_find_same_and_print(&parser, cmdline)?,
    [ "added", ref cmdline @ .. ] => command_print_added(&parser, cmdline)?,
    [ "removed", ref cmdline @ .. ] => command_print_removed(&parser, cmdline)?,
    [ "translated", ref cmdline @ .. ] => command_print_translated(&parser, cmdline)?,
    [ "untranslated", ref cmdline @ .. ] => command_print_untranslated(&parser, cmdline)?,
    [ "regular", ref cmdline @ .. ] => command_print_regular(&parser, cmdline)?,
    [ "plural", ref cmdline @ .. ] => command_print_plural(&parser, cmdline)?,
    [ "with-context", ref cmdline @ .. ] => command_print_with_context(&parser, cmdline)?,
    [ "with-word", ref cmdline @ .. ] => command_print_with_word(&parser, cmdline)?,
    [ "with-wordstr", ref cmdline @ .. ] => command_print_with_wordstr(&parser, cmdline)?,
    [ "with-unequal-linebreaks", ref cmdline @ .. ] => command_print_with_unequal_linebreaks(&parser, cmdline)?,
    [ "check-symbols", ref cmdline @ .. ] => command_check_symbols(&parser, cmdline)?,

    // TODO: split commands and their arguments into separate files
    // TODO: check: count of special tokens in msgid vs msgstr
    // TODO: check: strip spaces, lettes and numbers, then compare strings, to check correctness of special symbols
    // TODO: check: spaces at beginning/ending of msgstr as in msgid
    // TODO: check: capital letter at beginning of msgs as in msgid
    // TODO: filter: without words
    // TODO: try to fix messages after an problem with message is found after translation or review
    // TODO: multiline/singleline
    // TODO: check: spelling

    [ "help", .. ] | [] => help(),
    [ arg, ..] => bail!("Unknown command: \"{arg}\". Use --help for list of commands."),

  }

  Ok(())
}

fn help() {
  println!(r#"
Usage: po-tools [OPTIONS] [--] COMMAND [COMMAND_OPTIONS] [--] [COMMAND_ARGUMENTS]

COMMANDS:

  * translate [OPTIONS] FILE - WIP! translate PO file using AI.
  * review [OPTIONS] FILE [FILE...] - WIP! review multiple translations of _same_ file using AI.
  * compare FILE1 FILE[...] - list different variants of translation for the same file.

  * merge FILE1 FILE2 - merge two files by overwritting messages from FILE1 by messages from FILE2.

  * diff FILE1 FILE2 - diff two files by msgid.
  * diffstr FILE1 FILE2 - diff two files by msgstr.
  * added FILE1 FILE2 - print new messages in FILE2 only.
  * deleted FILE1 FILE2 - print missing messages from FILE1 only.

  * translated FILE - print messages with non-empty msgstr.
  * untranslated FILE - print messages with empty msgstr (even if just one msgstr is empty for plural messages).
  * regular FILE - print regular PO messages, not ones with context or plural messages.
  * plural FILE - print plural messages only.
  * with-context FILE - print messages with msgctxt field.
  * with-word WORD FILE - print messages with given word in msgid.
  * with-wordstr WORD FILE - print messages with given word in msgstr.
  * with-unequal-linebreaks - print messages where msgstr doesn't contain same number of linebreaks as msgid.
  * check-symbols - print messages where special symbols are not same

  * sort FILE - sort messages in lexical order.
  * parse - parse file and dump (for debugging)

"#);
}

