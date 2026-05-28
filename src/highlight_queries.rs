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
