# PO-tools

A collection of tools, which I use for better productivity with GNU
Gettext PO files when translating them.


## Commands:

  * translate [OPTIONS] FILE - WIP! translate PO file using AI.
  * review [OPTIONS] FILE [FILE...] - WIP! review multiple translations of _same_ file using AI.

  * merge FILE1 FILE2 - merge two files by overwritting messages from FILE1 by messages from FILE2.

  * diff FILE1 FILE2 - diff two files by msgid.
  * diffstr FILE1 FILE2 - diff two files by msgstr.
  * added FILE1 FILE2 - print new messages in FILE2 only.
  * deleted FILE1 FILE2 - print missing messages from FILE1 only.

  * translated FILE - print messages with non-empty msgstr.
  * untranslated FILE - print messages with empty msgstr (even if just one msgstr is empty for plural messages).
  * translated-untranslated - print translated messages first, then untranslated.
  * regular FILE - print regular PO messages, not ones with context or plural messages.
  * plural FILE - print plural messages only.
  * with-context FILE - print messages with msgctxt field.

  * sort FILE - sort messages in lexical order.
  * parse - parse file and dump (for debugging)
