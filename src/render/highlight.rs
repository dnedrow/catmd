//! Syntax highlighting of fenced code blocks.
//!
//! The language named on a fence selects a grammar from the bundled set; each
//! line is tokenized and every token's scope stack is classified into one of
//! a handful of theme roles (keyword, string, comment, …). Anything that
//! matches no role keeps the plain `code_block` style, and anything that goes
//! wrong degrades to that style too — highlighting changes styles, never text.
//!
//! The grammar set is deserialized on first use, so documents without a
//! recognizable fenced language never pay for it.

use std::str::FromStr;
use std::sync::OnceLock;

use anstyle::Style;
use syntect::easy::ScopeRegionIterator;
use syntect::highlighting::ScopeSelectors;
use syntect::parsing::{ParseState, Scope, ScopeStack, ScopeStackOp, SyntaxReference, SyntaxSet};
use two_face::re_exports::syntect;

use super::layout::{Span, coalesce};
use crate::theme::Theme;

/// Info-string words that explicitly ask for no highlighting.
const PLAIN: &[&str] = &["text", "plaintext", "plain", "txt", "nohighlight"];

/// Info-string words that grammars do not know under that name, mapped to a
/// token they do know. `console` and `shell-session` are deliberately absent:
/// such blocks mix commands with program output, and the Bash grammar treats
/// an apostrophe in the output as an unterminated string that swallows the
/// rest of the block, so they render in the plain code style instead.
const ALIASES: &[(&str, &str)] = &[
    ("shell", "bash"),
    ("zsh", "bash"),
    ("jsonc", "json"),
    ("json5", "json"),
    ("tf", "terraform"),
    ("hcl", "terraform"),
    ("csharp", "cs"),
    ("c#", "cs"),
    ("fsharp", "fs"),
    ("f#", "fs"),
    ("golang", "go"),
    ("docker", "dockerfile"),
];

/// The grammar set, deserialized on first use.
fn syntaxes() -> &'static SyntaxSet {
    static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();
    SYNTAXES.get_or_init(two_face::syntax::extra_newlines)
}

/// A language the bundled grammar set can highlight.
#[derive(Debug, Clone, Copy)]
pub struct Language(&'static SyntaxReference);

impl Language {
    /// The grammar's display name, e.g. `Rust`.
    pub fn name(&self) -> &'static str {
        &self.0.name
    }
}

/// Resolve the language a fence's info string names, if any.
///
/// The language is the first whitespace-delimited word, cut at the first `,`
/// (so `rust,ignore` is Rust) and stripped of `{`/`}`, matched
/// case-insensitively by grammar name or file extension after the alias table.
pub fn language(info: &str) -> Option<Language> {
    let word = info.split_whitespace().next()?;
    let word = word.split(',').next().unwrap_or_default();
    let word = word.trim_matches(['{', '}']).to_ascii_lowercase();
    if word.is_empty() || PLAIN.contains(&word.as_str()) {
        return None;
    }
    let token = ALIASES
        .iter()
        .find(|(alias, _)| *alias == word)
        .map_or(word.as_str(), |(_, token)| token);
    syntaxes().find_syntax_by_token(token).map(Language)
}

/// The syntactic roles highlighting distinguishes; each has a theme role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Keyword,
    String,
    Comment,
    Constant,
    Type,
    Function,
    Tag,
}

impl Role {
    /// The theme style for this role.
    pub fn style(self, theme: &Theme) -> Style {
        match self {
            Role::Keyword => theme.syntax_keyword,
            Role::String => theme.syntax_string,
            Role::Comment => theme.syntax_comment,
            Role::Constant => theme.syntax_constant,
            Role::Type => theme.syntax_type,
            Role::Function => theme.syntax_function,
            Role::Tag => theme.syntax_tag,
        }
    }
}

/// Scope selectors per role, in tmTheme selector syntax. A token takes the
/// role whose selector matches deepest in its scope stack, so string
/// delimiters take the string role and escapes inside strings take the
/// constant role. Operators are excluded from the keyword role so `=` and
/// `->` stay in the plain code style. `storage.type` is a keyword: grammars
/// use it for declaration words (`fn`, `struct`, `var`, `const`) and scope
/// type names as `support.type` or `entity.name.*`.
const SELECTORS: &[(&str, Role)] = &[
    ("keyword - keyword.operator, storage", Role::Keyword),
    ("string", Role::String),
    ("comment", Role::Comment),
    ("constant, keyword.constant", Role::Constant),
    (
        "entity.name.type, entity.name.class, entity.name.struct, entity.name.enum, \
         entity.name.trait, entity.name.interface, entity.name.impl, \
         entity.other.inherited-class, support.type, support.class",
        Role::Type,
    ),
    (
        "entity.name.function, entity.type.function, support.function, variable.function",
        Role::Function,
    ),
    (
        "entity.name.tag, entity.name.table, meta.mapping.key string",
        Role::Tag,
    ),
];

fn classifier() -> &'static [(ScopeSelectors, Role)] {
    static TABLE: OnceLock<Vec<(ScopeSelectors, Role)>> = OnceLock::new();
    TABLE.get_or_init(|| {
        SELECTORS
            .iter()
            .map(|(selector, role)| {
                let selectors = ScopeSelectors::from_str(selector)
                    .unwrap_or_else(|e| panic!("built-in selector `{selector}`: {e:?}"));
                (selectors, *role)
            })
            .collect()
    })
}

/// The role of a token with the given scope stack, or `None` for plain code.
pub fn classify(stack: &[Scope]) -> Option<Role> {
    classifier()
        .iter()
        .filter_map(|(selectors, role)| selectors.does_match(stack).map(|power| (power, *role)))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, role)| role)
}

/// The scope operations of one line, or `None` when the grammar fails on it.
type LineParser<'a> = dyn FnMut(&str) -> Option<Vec<(usize, ScopeStackOp)>> + 'a;

/// Highlight `literal` as `language`: one span list per line, without line
/// endings, styled from `theme`. Lines after an analysis failure are emitted
/// as plain code.
pub fn lines(language: Language, literal: &str, theme: &Theme) -> Vec<Vec<Span>> {
    let set = syntaxes();
    let mut state = ParseState::new(language.0);
    let mut parser = |line: &str| state.parse_line(line, set).ok();
    lines_with(&mut parser, literal, theme)
}

fn lines_with(parser: &mut LineParser<'_>, literal: &str, theme: &Theme) -> Vec<Vec<Span>> {
    let mut out = Vec::new();
    let mut stack = ScopeStack::new();
    let mut failed = false;
    for raw in literal.split_inclusive('\n') {
        let spans = if failed {
            None
        } else {
            highlight_line(parser, raw, &mut stack, theme)
        };
        match spans {
            Some(spans) => out.push(spans),
            None => {
                failed = true;
                out.push(vec![Span::new(without_line_ending(raw), theme.code_block)]);
            }
        }
    }
    out
}

fn highlight_line(
    parser: &mut LineParser<'_>,
    raw: &str,
    stack: &mut ScopeStack,
    theme: &Theme,
) -> Option<Vec<Span>> {
    let ops = parser(raw)?;
    let mut spans = Vec::new();
    for (text, op) in ScopeRegionIterator::new(&ops, raw) {
        stack.apply(op).ok()?;
        if text.is_empty() {
            continue;
        }
        let style = classify(stack.as_slice()).map_or(theme.code_block, |role| role.style(theme));
        spans.push(Span::new(text, style));
    }
    let mut spans = coalesce(spans);
    trim_line_ending(&mut spans);
    Some(spans)
}

/// `raw` without a trailing `\n` or `\r\n`, matching [`str::lines`].
fn without_line_ending(raw: &str) -> &str {
    match raw.strip_suffix('\n') {
        Some(line) => line.strip_suffix('\r').unwrap_or(line),
        None => raw,
    }
}

/// Remove a trailing `\n` or `\r\n` from the end of `spans`, dropping spans
/// that become empty.
fn trim_line_ending(spans: &mut Vec<Span>) {
    let mut pending = ['\n', '\r'].iter();
    let mut expected = pending.next();
    while let (Some(last), Some(&c)) = (spans.last_mut(), expected) {
        if last.text.ends_with(c) {
            last.text.pop();
            expected = pending.next();
            if last.text.is_empty() {
                spans.pop();
            }
        } else {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::layout::spans_text;

    fn stack(scopes: &str) -> ScopeStack {
        ScopeStack::from_str(scopes).unwrap()
    }

    fn highlighted(lang: &str, code: &str) -> Vec<Vec<Span>> {
        let language = language(lang).unwrap_or_else(|| panic!("{lang} should resolve"));
        lines(language, code, &Theme::default())
    }

    /// The style of the first span whose text is exactly `needle`.
    fn style_of(spans: &[Vec<Span>], needle: &str) -> Style {
        spans
            .iter()
            .flatten()
            .find(|s| s.text == needle)
            .unwrap_or_else(|| panic!("no span {needle:?} in {spans:?}"))
            .style
    }

    #[test]
    fn language_is_the_first_word_cut_at_a_comma_and_case_insensitive() {
        for info in [
            "rust,ignore",
            "js title=\"x\"",
            "Rust",
            "rs",
            "{rust}",
            "  rust  ",
        ] {
            assert!(language(info).is_some(), "{info:?}");
        }
        assert_eq!(language("rust,ignore").unwrap().name(), "Rust");
        assert_eq!(language("js title=\"x\"").unwrap().name(), "JavaScript");
        assert_eq!(language("Rust").unwrap().name(), "Rust");
        assert_eq!(language("rs").unwrap().name(), "Rust");
    }

    #[test]
    fn aliases_resolve() {
        for (alias, name) in [
            ("shell", "Bourne Again Shell (bash)"),
            ("zsh", "Bourne Again Shell (bash)"),
            ("jsonc", "JSON"),
            ("json5", "JSON"),
            ("tf", "Terraform"),
            ("hcl", "Terraform"),
            ("csharp", "C#"),
            ("golang", "Go"),
        ] {
            assert_eq!(language(alias).map(|l| l.name()), Some(name), "{alias}");
        }
    }

    #[test]
    fn plain_markers_unknown_names_and_empty_info_resolve_to_none() {
        for info in [
            "text",
            "plaintext",
            "plain",
            "txt",
            "nohighlight",
            "TEXT",
            "foobarlang",
            "console",
            "",
            "   ",
            "{}",
        ] {
            assert!(language(info).is_none(), "{info:?}");
        }
    }

    #[test]
    fn common_languages_resolve() {
        for token in [
            "bash",
            "sh",
            "c",
            "cpp",
            "csharp",
            "css",
            "dockerfile",
            "go",
            "html",
            "java",
            "javascript",
            "js",
            "json",
            "kotlin",
            "markdown",
            "python",
            "py",
            "ruby",
            "rust",
            "sql",
            "swift",
            "toml",
            "typescript",
            "ts",
            "tsx",
            "xml",
            "yaml",
        ] {
            assert!(language(token).is_some(), "{token} should resolve");
        }
    }

    #[test]
    fn classifier_picks_the_deepest_matching_role() {
        let cases = [
            (
                "source.rust storage.type.function.rust",
                Some(Role::Keyword),
            ),
            ("source.rust keyword.control.rust", Some(Role::Keyword)),
            ("source.rust storage.modifier.rust", Some(Role::Keyword)),
            ("source.rust keyword.operator.assignment.rust", None),
            (
                "string.quoted.double punctuation.definition.string.begin",
                Some(Role::String),
            ),
            (
                "comment.line punctuation.definition.comment",
                Some(Role::Comment),
            ),
            (
                "string.quoted constant.character.escape",
                Some(Role::Constant),
            ),
            ("constant.numeric.integer", Some(Role::Constant)),
            ("entity.name.function", Some(Role::Function)),
            ("support.function", Some(Role::Function)),
            ("support.type", Some(Role::Type)),
            ("entity.name.struct", Some(Role::Type)),
            ("entity.name.tag", Some(Role::Tag)),
            ("variable.other", None),
            ("punctuation.section.block.begin", None),
            ("", None),
        ];
        for (scopes, expected) in cases {
            assert_eq!(classify(stack(scopes).as_slice()), expected, "{scopes:?}");
        }
    }

    #[test]
    fn text_is_preserved_line_for_line() {
        let fixtures = [
            ("rust", "fn main() {\n    let s = \"a\\n\"; // c\n}\n"),
            ("toml", "[section]\nkey = 123\nlist = [\"a\", 'b']\n"),
            ("bash", "for f in *.md; do\n  echo \"$f\"\ndone\n"),
            ("html", "<p class=\"x\">hi &amp; bye</p>\n"),
            ("rust", "fn a() {}\r\nfn b() {}\r\n"),
            ("rust", "no trailing newline"),
            ("rust", "a\n\n\nb\n"),
            ("rust", "trailing spaces   \n\t\n"),
        ];
        for (lang, code) in fixtures {
            let lines = highlighted(lang, code);
            let texts: Vec<String> = lines.iter().map(|l| spans_text(l)).collect();
            let expected: Vec<&str> = code.lines().collect();
            assert_eq!(texts, expected, "{lang}: {code:?}");
            for line in &lines {
                assert!(line.iter().all(|s| !s.text.is_empty()), "{line:?}");
            }
        }
    }

    #[test]
    fn rust_tokens_take_their_roles() {
        let theme = Theme::default();
        let lines = highlighted("rust", "fn main() {}\nlet x = 1; // one\n");
        assert_eq!(style_of(&lines, "fn"), theme.syntax_keyword);
        assert_eq!(style_of(&lines, "main"), theme.syntax_function);
        assert_eq!(style_of(&lines, "let"), theme.syntax_keyword);
        assert_eq!(style_of(&lines, "1"), theme.syntax_constant);
        assert_eq!(style_of(&lines, "// one"), theme.syntax_comment);
        // Punctuation, operators, and identifiers coalesce into plain runs.
        for plain in ["() {}", " x = ", "; "] {
            assert_eq!(style_of(&lines, plain), theme.code_block, "{plain:?}");
        }
    }

    #[test]
    fn string_delimiters_take_the_string_role_as_one_run() {
        let theme = Theme::default();
        let lines = highlighted("python", "print(\"hi\")\n");
        assert_eq!(style_of(&lines, "\"hi\""), theme.syntax_string);
        assert_eq!(style_of(&lines, "print"), theme.syntax_function);
    }

    #[test]
    fn html_tag_names_take_the_tag_role() {
        let theme = Theme::default();
        let lines = highlighted("html", "<p>x</p>\n");
        let tags: Vec<&Span> = lines.iter().flatten().filter(|s| s.text == "p").collect();
        assert_eq!(tags.len(), 2, "{lines:?}");
        assert!(tags.iter().all(|s| s.style == theme.syntax_tag));
        assert_eq!(style_of(&lines, ">x</"), theme.code_block);
    }

    #[test]
    fn swift_and_typescript_declarations_split_keywords_from_types() {
        let theme = Theme::default();
        let swift = highlighted(
            "swift",
            "struct Point { var x: Int }\nfunc f() -> Bool { return true }\n",
        );
        assert_eq!(style_of(&swift, "struct"), theme.syntax_keyword);
        assert_eq!(style_of(&swift, "var"), theme.syntax_keyword);
        assert_eq!(style_of(&swift, "Int"), theme.syntax_type);
        assert_eq!(style_of(&swift, "Bool"), theme.syntax_type);
        assert_eq!(style_of(&swift, "func"), theme.syntax_keyword);
        assert_eq!(style_of(&swift, "f"), theme.syntax_function);
        assert_eq!(style_of(&swift, "true"), theme.syntax_constant);
        assert_eq!(style_of(&swift, " Point { "), theme.code_block);

        let ts = highlighted(
            "typescript",
            "interface Foo {}\nclass Bar extends Foo {}\nconst n: number = 1;\n",
        );
        assert_eq!(style_of(&ts, "interface"), theme.syntax_keyword);
        assert_eq!(style_of(&ts, "class"), theme.syntax_keyword);
        assert_eq!(style_of(&ts, "extends"), theme.syntax_keyword);
        assert_eq!(style_of(&ts, "const"), theme.syntax_keyword);
        assert_eq!(style_of(&ts, "Bar"), theme.syntax_type);
        assert_eq!(style_of(&ts, "number"), theme.syntax_type);
        assert_eq!(style_of(&ts, "1"), theme.syntax_constant);
        let foos: Vec<&Span> = ts.iter().flatten().filter(|s| s.text == "Foo").collect();
        assert_eq!(foos.len(), 2);
        assert!(foos.iter().all(|s| s.style == theme.syntax_type), "{ts:?}");
    }

    #[test]
    fn rust_type_names_are_types_and_declaration_words_are_keywords() {
        let theme = Theme::default();
        let lines = highlighted("rust", "pub struct Foo { v: Vec<String> }\nimpl Foo {}\n");
        assert_eq!(style_of(&lines, "pub"), theme.syntax_keyword);
        assert_eq!(style_of(&lines, "struct"), theme.syntax_keyword);
        assert_eq!(style_of(&lines, "impl"), theme.syntax_keyword);
        assert_eq!(style_of(&lines, "Vec"), theme.syntax_type);
        assert_eq!(style_of(&lines, "String"), theme.syntax_type);
        let foos: Vec<&Span> = lines.iter().flatten().filter(|s| s.text == "Foo").collect();
        assert_eq!(foos.len(), 2);
        assert!(
            foos.iter().all(|s| s.style == theme.syntax_type),
            "{lines:?}"
        );
    }

    #[test]
    fn structured_data_keys_take_the_tag_role() {
        let theme = Theme::default();
        let toml = highlighted("toml", "[section]\nkey = \"v\"\nnum = 1\n");
        assert_eq!(style_of(&toml, "section"), theme.syntax_tag);
        assert_eq!(style_of(&toml, "key"), theme.syntax_tag);
        assert_eq!(style_of(&toml, "\"v\""), theme.syntax_string);
        assert_eq!(style_of(&toml, "1"), theme.syntax_constant);

        let yaml = highlighted("yaml", "name: catmd\nn: 2\n");
        assert_eq!(style_of(&yaml, "name"), theme.syntax_tag);
        assert_eq!(style_of(&yaml, "2"), theme.syntax_constant);

        let json = highlighted("json", "{\"a\": \"b\", \"c\": true}\n");
        assert_eq!(style_of(&json, "\"a\""), theme.syntax_tag);
        assert_eq!(style_of(&json, "\"b\""), theme.syntax_string);
        assert_eq!(style_of(&json, "true"), theme.syntax_constant);
    }

    #[test]
    fn a_failing_parser_degrades_the_rest_of_the_block_to_plain_code() {
        let theme = Theme::default();
        let set = syntaxes();
        let language = language("rust").unwrap();
        let mut state = ParseState::new(language.0);
        let mut calls = 0;
        let mut parser = |line: &str| {
            calls += 1;
            if calls == 2 {
                None
            } else {
                state.parse_line(line, set).ok()
            }
        };
        let code = "fn a() {}\nfn b() {}\nfn c() {}\n";
        let lines = lines_with(&mut parser, code, &theme);
        let texts: Vec<String> = lines.iter().map(|l| spans_text(l)).collect();
        assert_eq!(texts, vec!["fn a() {}", "fn b() {}", "fn c() {}"]);
        assert_eq!(style_of(&lines[..1], "fn"), theme.syntax_keyword);
        assert_eq!(lines[1], vec![Span::new("fn b() {}", theme.code_block)]);
        assert_eq!(lines[2], vec![Span::new("fn c() {}", theme.code_block)]);
        assert_eq!(calls, 2, "parsing stops after the failure");
    }

    #[test]
    fn line_endings_are_trimmed_from_the_last_span() {
        let s = |t: &str| Span::new(t, Style::new());
        let mut spans = vec![s("a"), s("\n")];
        trim_line_ending(&mut spans);
        assert_eq!(spans, vec![s("a")]);
        let mut spans = vec![s("a\r\n")];
        trim_line_ending(&mut spans);
        assert_eq!(spans, vec![s("a")]);
        let mut spans = vec![s("a"), s("\r"), s("\n")];
        trim_line_ending(&mut spans);
        assert_eq!(spans, vec![s("a")]);
        let mut spans = vec![s("a\r")];
        trim_line_ending(&mut spans);
        assert_eq!(spans, vec![s("a\r")], "a bare CR is content");
        let mut spans = vec![s("\n")];
        trim_line_ending(&mut spans);
        assert!(spans.is_empty());
        assert_eq!(without_line_ending("a\r\n"), "a");
        assert_eq!(without_line_ending("a\r"), "a\r");
        assert_eq!(without_line_ending("a"), "a");
    }
}
