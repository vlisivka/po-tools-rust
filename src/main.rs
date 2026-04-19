use anyhow::{Result, bail};

mod parser;
use crate::parser::Parser;

#[macro_use]
mod localization;

mod command_sort;
use crate::command_sort::command_sort_and_print;

mod command_parse_and_dump;
use crate::command_parse_and_dump::command_parse_and_dump;

mod command_merge_and_print;
use crate::command_merge_and_print::command_merge_and_print;

mod command_print_added;
use crate::command_print_added::{
    command_diff_by_id_and_print, command_print_added, command_print_removed,
};

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
use crate::command_translate_and_print::command_translate_and_print;

mod command_review_files_and_print;
use crate::command_review_files_and_print::command_review_files_and_print;

mod command_erase_and_print;
use crate::command_erase_and_print::command_erase_and_print;

mod command_check_symbols;
use crate::command_check_symbols::command_check_symbols;

mod command_filter_with_ai_and_print;
use crate::command_filter_with_ai_and_print::command_filter_with_ai_and_print;

mod util;

mod dictionary;

fn main() -> Result<()> {
    // Initial localization call
    localization::load_translations(&Parser::new(None));

    // Options
    let mut number_of_plural_cases: Option<usize> = None;

    // Parse arguments
    let args = std::env::args().collect::<Vec<String>>();
    let tail = &args[1..].iter().map(|s| s as &str).collect::<Vec<&str>>();
    let mut tail = &tail[..];

    // Parse options
    loop {
        match tail[..] {
      [ "-c", n, ref rest @ ..] | [ "--cases", n, ref rest @ ..] => {
        match n.parse::<usize>() {
          Ok(n) if (1..10).contains(&n) => {
            number_of_plural_cases = Some(n);
            tail = rest;
          }
          _ => bail!(
            tr!("Invalid argument for -c | --cases option. Expected: number of plural cases between 1 and 9. Actual value: \"{value}\".")
              .replace("{value}", n)
          ),
        }
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help();
        return Ok(());
      }
      [ "--", ref rest @ .. ] => {
        tail = rest;
        break;
      }
      [ arg, ..] if arg.starts_with('-') => bail!(
        "{}",
        tr!("Unknown option: \"{option}\". Use --help for list of options.").replace("{option}", arg)
      ),
      _ => break,
    }
    }

    let parser = Parser::new(number_of_plural_cases);

    // Parse arguments
    match tail[..] {
        ["parse", ref cmdline @ ..] => command_parse_and_dump(&parser, cmdline)?,
        ["translate", ref cmdline @ ..] => command_translate_and_print(&parser, cmdline)?,
        ["erase", ref cmdline @ ..] => command_erase_and_print(&parser, cmdline)?,
        ["review", ref cmdline @ ..] => command_review_files_and_print(&parser, cmdline)?,
        ["compare", ref cmdline @ ..] => command_compare_files_and_print(&parser, cmdline)?,
        ["sort", ref cmdline @ ..] => command_sort_and_print(&parser, cmdline)?,
        ["merge", ref cmdline @ ..] => command_merge_and_print(&parser, cmdline)?,
        ["diff", ref cmdline @ ..] => command_diff_by_id_and_print(&parser, cmdline)?,
        ["diffstr", ref cmdline @ ..] => command_diff_by_str_and_print(&parser, cmdline)?,
        ["same", ref cmdline @ ..] => command_find_same_and_print(&parser, cmdline)?,
        ["added", ref cmdline @ ..] => command_print_added(&parser, cmdline)?,
        ["removed", ref cmdline @ ..] => command_print_removed(&parser, cmdline)?,
        ["translated", ref cmdline @ ..] => command_print_translated(&parser, cmdline)?,
        ["untranslated", ref cmdline @ ..] => command_print_untranslated(&parser, cmdline)?,
        ["regular", ref cmdline @ ..] => command_print_regular(&parser, cmdline)?,
        ["plural", ref cmdline @ ..] => command_print_plural(&parser, cmdline)?,
        ["with-context", ref cmdline @ ..] => command_print_with_context(&parser, cmdline)?,
        ["with-word", ref cmdline @ ..] => command_print_with_word(&parser, cmdline)?,
        ["with-wordstr", ref cmdline @ ..] => command_print_with_wordstr(&parser, cmdline)?,
        ["with-unequal-linebreaks", ref cmdline @ ..] => {
            command_print_with_unequal_linebreaks(&parser, cmdline)?
        }
        ["check-symbols", ref cmdline @ ..] => command_check_symbols(&parser, cmdline)?,
        ["filter", ref cmdline @ ..] => command_filter_with_ai_and_print(&parser, cmdline)?,

        // TODO: Parse comments to support fuzzy messages
        // TODO: dictionary. If message contains a word from the dictionary, then add dictionary record as a hint for AI
        // TODO: sort messages by size, by msgstr, by first letter, by first special symbol, etc.
        // TODO: split large po file into smaller chunks
        // TODO: check: spaces at beginning/ending of msgstr as in msgid
        // TODO: check: capital letter at beginning of msgs as in msgid
        // TODO: filter: without words
        // TODO: try to fix messages after an problem with message is found after translation or review
        // TODO: multiline/singleline
        // TODO: check: spelling
        ["help", ..] | [] => help(),
        [arg, ..] => bail!(
            "{}",
            tr!("Unknown command: \"{command}\". Use --help for list of commands.")
                .replace("{command}", arg)
        ),
    }

    Ok(())
}

fn help() {
    println!(
        "{}",
        tr!(
            r#"Usage: po-tools [OPTIONS] [--] COMMAND [COMMAND_OPTIONS] [--] [COMMAND_ARGUMENTS]

COMMANDS

  * translate [OPTIONS] FILE - WIP! Translate PO file using AI.
  * filter [OPTIONS] FILE - WIP! Filter messages using AI.
  * review [OPTIONS] FILE [FILE...] - WIP! Review multiple translations of _same_ file using AI.
  * compare FILE1 FILE[...] - List different variants of translation for the same file.

  * merge FILE1 FILE2 - Merge two files by overwriting messages from FILE1 with messages from FILE2.

  * erase FILE[...] - Erase translations of messages.

  * diff FILE1 FILE2 - Diff two files by msgid.
  * diffstr FILE1 FILE2 - Diff two files by msgstr.
  * added FILE1 FILE2 - Print new messages from FILE2 only.
  * deleted FILE1 FILE2 - Print missing messages from FILE1 only.

  * translated FILE - Print messages with non-empty msgstr.
  * untranslated FILE - Print messages with empty msgstr (even if just one msgstr is empty for plural messages).
  * regular FILE - Print regular PO messages, excluding ones with context or plural messages.
  * plural FILE - Print plural messages only.
  * with-context FILE - Print messages with msgctxt field.
  * with-word WORD FILE - Print messages with given word in msgid.
  * with-wordstr WORD FILE - Print messages with given word in msgstr.
  * with-unequal-linebreaks - Print messages where msgstr does not contain same number of linebreaks as msgid.
  * check-symbols - Print messages where special symbols are not same.

  * sort FILE - Sort messages in lexical order.
  * parse - Parse file and dump (for debugging).

OPTIONS

  -c | --cases PLURAL_CASES    Number of plural cases to use in messages. If message has fewer than PLURAL_CASES, then empty ones will be added.
"#
        )
    );
}
