use indoc::indoc;

pub const LEAN_HIGHLIGHT_QUERY: &str = indoc! {r"
  (comment) @comment

  [
    (string)
    (interpolated_string)
    (char)
  ] @string

  [
    (number)
    (float)
  ] @number

  [
    (abbrev)
    (axiom)
    (by)
    (class)
    (constant)
    (def)
    (do)
    (else)
    (end)
    (example)
    (forall)
    (fun)
    (have)
    (if)
    (import)
    (inductive)
    (instance)
    (lemma)
    (let)
    (namespace)
    (opaque)
    (open)
    (return)
    (section)
    (show)
    (sorry)
    (structure)
    (then)
    (theorem)
    (try)
    (variable)
    (where)
    (with)
  ] @keyword

  ((definition name: (identifier) @function))
  ((inductive name: (identifier) @type))
  ((structure name: (identifier) @type))
"};

pub const HIGHLIGHT_NAMES: [&str; 18] = [
  "none",
  "comment",
  "function",
  "keyword",
  "number",
  "punctuation.delimiter",
  "punctuation.special",
  "string.escape",
  "string",
  "text.emphasis",
  "text.literal",
  "text.reference",
  "text.strong",
  "text.title",
  "text.uri",
  "type",
  "markup.raw",
  "markup.raw.block",
];

pub const HIGHLIGHT_COMMENT: usize = 1;
pub const HIGHLIGHT_FUNCTION: usize = 2;
pub const HIGHLIGHT_KEYWORD: usize = 3;
pub const HIGHLIGHT_NUMBER: usize = 4;
pub const HIGHLIGHT_PUNCTUATION_DELIMITER: usize = 5;
pub const HIGHLIGHT_PUNCTUATION_SPECIAL: usize = 6;
pub const HIGHLIGHT_STRING_ESCAPE: usize = 7;
pub const HIGHLIGHT_STRING: usize = 8;
pub const HIGHLIGHT_TEXT_EMPHASIS: usize = 9;
pub const HIGHLIGHT_TEXT_LITERAL: usize = 10;
pub const HIGHLIGHT_TEXT_REFERENCE: usize = 11;
pub const HIGHLIGHT_TEXT_STRONG: usize = 12;
pub const HIGHLIGHT_TEXT_TITLE: usize = 13;
pub const HIGHLIGHT_TEXT_URI: usize = 14;
pub const HIGHLIGHT_TYPE: usize = 15;
pub const HIGHLIGHT_MARKUP_RAW: usize = 16;
pub const HIGHLIGHT_MARKUP_RAW_BLOCK: usize = 17;
