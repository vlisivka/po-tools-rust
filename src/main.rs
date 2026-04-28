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

mod util;

mod dictionary;

fn main() -> Result<()> {
    // Initial localization call
    localization::load_translations(&Parser::new(None));

    // Options
    let mut number_of_plural_cases: Option<usize> = None;
    let mut strip_comments = false;

    // Parse arguments
    let args = std::env::args().collect::<Vec<String>>();
    let tail = &args[1..].iter().map(|s| s as &str).collect::<Vec<&str>>();
    let mut tail = &tail[..];

    let mut stdout = std::io::stdout();
    let mut stderr = std::io::stderr();
    let mut ctx = util::IoContext {
        out: &mut stdout,
        err: &mut stderr,
    };

    // Parse options
    loop {
        match tail[..] {
            ["-c", n, ref rest @ ..] | ["--cases", n, ref rest @ ..] => {
                match n.parse::<usize>() {
                    Ok(n) if (1..10).contains(&n) => {
                        number_of_plural_cases = Some(n);
                        tail = rest;
                    }
                    _ => bail!(tr!("Invalid argument for -c | --cases option. Expected: number of plural cases between 1 and 9. Actual value: \"{value}\".")
              .replace("{value}", n)),
                }
            }
            ["--strip-comments", ref rest @ ..] => {
                strip_comments = true;
                tail = rest;
            }

            ["-h", ..] | ["--help", ..] => {
                help(&mut ctx)?;
                return Ok(());
            }
            ["--", ref rest @ ..] => {
                tail = rest;
                break;
            }
            [arg, ..] if arg.starts_with('-') => bail!(
                "{}",
                tr!("Unknown option: \"{option}\". Use --help for list of options.").replace("{option}", arg)
            ),
            _ => break,
        }
    }

    let mut parser = Parser::new(number_of_plural_cases);
    parser.strip_comments = strip_comments;

    // Parse arguments
    match tail[..] {
        ["parse", ref cmdline @ ..] => command_parse_and_dump(&parser, cmdline, &mut ctx)?,
        ["translate", ref cmdline @ ..] => command_translate_and_print(&parser, cmdline, &mut ctx)?,
        ["erase", ref cmdline @ ..] => command_erase_and_print(&parser, cmdline, &mut ctx)?,
        ["review", ref cmdline @ ..] => command_review_files_and_print(&parser, cmdline, &mut ctx)?,
        ["compare", ref cmdline @ ..] => {
            command_compare_files_and_print(&parser, cmdline, &mut ctx)?
        }
        ["sort", ref cmdline @ ..] => command_sort_and_print(&parser, cmdline, &mut ctx)?,
        ["merge", ref cmdline @ ..] => command_merge_and_print(&parser, cmdline, &mut ctx)?,
        ["diff", ref cmdline @ ..] => command_diff_by_id_and_print(&parser, cmdline, &mut ctx)?,
        ["diffstr", ref cmdline @ ..] => command_diff_by_str_and_print(&parser, cmdline, &mut ctx)?,
        ["same", ref cmdline @ ..] => command_find_same_and_print(&parser, cmdline, &mut ctx)?,
        ["added", ref cmdline @ ..] => command_print_added(&parser, cmdline, &mut ctx)?,
        ["removed", ref cmdline @ ..] => command_print_removed(&parser, cmdline, &mut ctx)?,
        ["translated", ref cmdline @ ..] => command_print_translated(&parser, cmdline, &mut ctx)?,
        ["untranslated", ref cmdline @ ..] => {
            command_print_untranslated(&parser, cmdline, &mut ctx)?
        }
        ["regular", ref cmdline @ ..] => command_print_regular(&parser, cmdline, &mut ctx)?,
        ["plural", ref cmdline @ ..] => command_print_plural(&parser, cmdline, &mut ctx)?,
        ["with-context", ref cmdline @ ..] => {
            command_print_with_context(&parser, cmdline, &mut ctx)?
        }
        ["with-word", ref cmdline @ ..] => command_print_with_word(&parser, cmdline, &mut ctx)?,
        ["with-wordstr", ref cmdline @ ..] => {
            command_print_with_wordstr(&parser, cmdline, &mut ctx)?
        }
        ["with-unequal-linebreaks", ref cmdline @ ..] => {
            command_print_with_unequal_linebreaks(&parser, cmdline, &mut ctx)?
        }
        ["check-symbols", ref cmdline @ ..] => command_check_symbols(&parser, cmdline, &mut ctx)?,

        ["help", ..] | [] => help(&mut ctx)?,
        [arg, ..] => bail!(
            "{}",
            tr!("Unknown command: \"{command}\". Use --help for list of commands.")
                .replace("{command}", arg)
        ),
    }

    Ok(())
}

fn help(ctx: &mut util::IoContext) -> Result<()> {
    writeln!(
        ctx.out,
        "{}",
        tr!(
            r#"Usage: po-tools [OPTIONS] [--] COMMAND [COMMAND_OPTIONS] [--] [COMMAND_ARGUMENTS]

COMMANDS

  * translate [OPTIONS] FILE - WIP! Translate PO file using AI.
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
  --strip-comments             Strip comments from PO files during parsing (ignore all lines starting with #).
"#
        )
    )?;
    Ok(())
}
