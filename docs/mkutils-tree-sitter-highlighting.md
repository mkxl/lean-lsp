# mkutils Tree-Sitter Highlighting Updates

This branch assumes `mkutils` owns reusable Tree-sitter highlighting for `ratatui`.
The `lean-lsp` crate should only provide a visual theme and call `highlight(language, source)`.

## Public API Needed

`mkutils` should expose these types from its TUI feature:

```rust
pub struct RatatuiTreeSitterHighlighter;
pub struct TreeSitterHighlightTheme;
```

Expected usage from `lean-lsp`:

```rust
static HOVER_HIGHLIGHTER: LazyLock<RatatuiTreeSitterHighlighter> = LazyLock::new(|| {
  let theme = TreeSitterHighlightTheme::new(Style::new().white())
    .with_style("keyword", Style::new().magenta())
    .with_style("string", Style::new().yellow())
    .with_style("comment", Style::new().dark_gray().italic())
    .with_style("type", Style::new().cyan().bold());

  RatatuiTreeSitterHighlighter::new(theme)
});

let lines = HOVER_HIGHLIGHTER.highlight("markdown", source)?;
```

## Language Ownership

`mkutils` should own all language registration needed by this use case.

Required built-in languages:

- `markdown`
- `markdown_inline`
- `lean`

Required alias:

- `lean4` should resolve to `lean`

This means `lean-lsp` should not depend directly on:

- `arborium-lean`
- `tree-sitter-md`
- `tree-sitter`
- `tree-sitter-highlight`

## Query Ownership

`mkutils` should own the Tree-sitter language crates and query constants.

Markdown should use `tree-sitter-md-025`:

- `LANGUAGE`
- `INLINE_LANGUAGE`
- `HIGHLIGHT_QUERY_BLOCK`
- `HIGHLIGHT_QUERY_INLINE`
- `INJECTION_QUERY_BLOCK`
- `INJECTION_QUERY_INLINE`

Lean should use `arborium-lean`:

- `language()`
- `HIGHLIGHTS_QUERY`
- `INJECTIONS_QUERY`
- `LOCALS_QUERY`

## Highlight Config Ownership

`mkutils` should build and cache `tree_sitter_highlight::HighlightConfiguration` values for its built-in languages.

During config construction it should call:

```rust
config.configure(HIGHLIGHT_NAMES);
```

`HIGHLIGHT_NAMES` should be a standard capture list broad enough for markdown and Arborium Lean captures, including:

```text
attribute
character
comment
constant
constructor
function
keyword
markup.raw
number
operator
property
punctuation
string
string.escape
text.emphasis
text.literal
text.reference
text.strong
text.title
text.uri
type
variable
warning
```

The exact list can be larger. Unknown captures should fall back to the default style.

## Theme Ownership Split

`mkutils` should own applying theme styles to highlight captures.

`lean-lsp` should own the actual style choices, e.g.:

```rust
TreeSitterHighlightTheme::new(Style::new().white())
  .with_style("keyword", Style::new().magenta())
  .with_style("string", Style::new().yellow())
```

`TreeSitterHighlightTheme` should support capture fallback. For example, if a query emits `function.definition` and only `function` is styled, the `function` style should apply.

## Highlight Method Behavior

`RatatuiTreeSitterHighlighter::highlight(language, source)` should:

1. Resolve language aliases, e.g. `lean4 -> lean`.
2. Look up the cached `HighlightConfiguration` for the language.
3. Call `tree_sitter_highlight::Highlighter::highlight`.
4. Resolve injected languages through the registered built-ins.
5. Process `HighlightEvent`s with a style stack.
6. Split Tree-sitter byte ranges into lines using `mkutils::Rope` byte summaries.
7. Return `Vec<ratatui::text::Line<'static>>`.

The method may take `&self` if internal mutation is hidden behind synchronization/interior mutability, or `&mut self` if the caller owns a mutable highlighter. This branch currently assumes `&self` because the highlighter is stored in a `LazyLock`.

## Dependencies and Features

The Tree-sitter dependencies should be optional in `mkutils`, ideally behind a feature such as `tree-sitter` or `tui-tree-sitter`.

Suggested dependency ownership in `mkutils`:

- `tree-sitter`
- `tree-sitter-highlight`
- `tree-sitter-md-025`
- `arborium-lean`

These should not be required for non-TUI/non-highlighting users if avoidable.
