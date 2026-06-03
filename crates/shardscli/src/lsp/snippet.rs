use std::borrow::Cow;
use anyhow::{anyhow, Result};

#[derive(Debug, PartialEq, Eq)]
pub enum CaseChange {
    Upcase,
    Downcase,
    Capitalize,
}

#[derive(Debug, PartialEq, Eq)]
pub enum FormatItem {
    Text(String),
    Capture(usize),
    CaseChange(usize, CaseChange),
    Conditional(usize, Option<String>, Option<String>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Regex {
    value: String,
    replacement: Vec<FormatItem>,
    options: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SnippetElement<'a> {
    Tabstop {
        tabstop: usize,
    },
    Placeholder {
        tabstop: usize,
        value: Vec<SnippetElement<'a>>,
    },
    Choice {
        tabstop: usize,
        choices: Vec<String>,
    },
    Variable {
        name: &'a str,
        default: Option<Vec<SnippetElement<'a>>>,
        regex: Option<Regex>,
    },
    Text(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Snippet<'a> {
    elements: Vec<SnippetElement<'a>>,
}

pub fn parse(s: &str) -> Result<Snippet<'_>> {
    parser::parse(s).map_err(|rest| anyhow!("Failed to parse snippet. Remaining input: {}", rest))
}

fn render_elements(
    snippet_elements: &[SnippetElement<'_>],
    insert: &mut String,
    offset: &mut usize,
    tabstops: &mut Vec<(usize, (usize, usize))>,
    newline_with_offset: &str,
    include_placeholder: bool,
) {
    use SnippetElement::*;

    for element in snippet_elements {
        match element {
            Text(text) => {
                // small optimization to avoid calling replace when it's unnecessary
                let text = if text.contains('\n') {
                    Cow::Owned(text.replace('\n', newline_with_offset))
                } else {
                    Cow::Borrowed(text.as_str())
                };
                *offset += text.chars().count();
                insert.push_str(&text);
            }
            Variable {
                name: _,
                regex: _,
                r#default,
            } => {
                // TODO: variables. For now, fall back to the default, which defaults to "".
                render_elements(
                    r#default.as_deref().unwrap_or_default(),
                    insert,
                    offset,
                    tabstops,
                    newline_with_offset,
                    include_placeholder,
                );
            }
            &Tabstop { tabstop } => {
                tabstops.push((tabstop, (*offset, *offset)));
            }
            Placeholder {
                tabstop,
                value: inner_snippet_elements,
            } => {
                let start_offset = *offset;
                if include_placeholder {
                    render_elements(
                        inner_snippet_elements,
                        insert,
                        offset,
                        tabstops,
                        newline_with_offset,
                        include_placeholder,
                    );
                }
                tabstops.push((*tabstop, (start_offset, *offset)));
            }
            &Choice {
                tabstop,
                choices: _,
            } => {
                // TODO: choices
                tabstops.push((tabstop, (*offset, *offset)));
            }
        }
    }
}

pub fn render(
    snippet: &Snippet<'_>,
    newline_with_offset: &str,
    include_placeholder: bool,
) -> (String, (usize, usize)) {
    let mut insert = String::new();
    let mut tabstops = Vec::new();
    let mut offset = 0;

    render_elements(
        &snippet.elements,
        &mut insert,
        &mut offset,
        &mut tabstops,
        newline_with_offset,
        include_placeholder,
    );

    // sort in ascending order (except for 0, which should always be the last one (per lsp doc))
    tabstops.sort_unstable_by_key(|(n, _)| if *n == 0 { usize::MAX } else { *n });

    // merge tabstops with the same index (we take advantage of the fact that we just sorted them
    // above to simply look backwards)
    let mut ntabstops = Vec::<SmallVec<[(usize, usize); 1]>>::new();
    {
        let mut prev = None;
        for (tabstop, r) in tabstops {
            if prev == Some(tabstop) {
                let len_1 = ntabstops.len() - 1;
                ntabstops[len_1].push(r);
            } else {
                prev = Some(tabstop);
                ntabstops.push(smallvec![r]);
            }
        }
    }

    (insert, ntabstops)
}

mod parser {
    use super::{CaseChange, FormatItem, Regex, Snippet, SnippetElement};

    /*
    https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#snippet_syntax

        any         ::= tabstop | placeholder | choice | variable | text
        tabstop     ::= '$' int | '${' int '}'
        placeholder ::= '${' int ':' any '}'
        choice      ::= '${' int '|' text (',' text)* '|}'
        variable    ::= '$' var | '${' var }'
                        | '${' var ':' any '}'
                        | '${' var '/' regex '/' (format | text)+ '/' options '}'
        format      ::= '$' int | '${' int '}'
                        | '${' int ':' '/upcase' | '/downcase' | '/capitalize' '}'
                        | '${' int ':+' if '}'
                        | '${' int ':?' if ':' else '}'
                        | '${' int ':-' else '}' | '${' int ':' else '}'
        regex       ::= Regular Expression value (ctor-string)
        options     ::= Regular Expression option (ctor-options)
        var         ::= [_a-zA-Z] [_a-zA-Z0-9]*
        int         ::= [0-9]+
        text        ::= .*
        if          ::= text
        else        ::= text
    */

    fn var<'a>() -> impl Parser<'a, Output = &'a str> {
        // var = [_a-zA-Z][_a-zA-Z0-9]*
        move |input: &'a str| {
            input
                .char_indices()
                .take_while(|(p, c)| {
                    *c == '_'
                        || if *p == 0 {
                            c.is_ascii_alphabetic()
                        } else {
                            c.is_ascii_alphanumeric()
                        }
                })
                .last()
                .map(|(index, c)| {
                    let index = index + c.len_utf8();
                    (&input[index..], &input[0..index])
                })
                .ok_or(input)
        }
    }

    const TEXT_ESCAPE_CHARS: &[char] = &['\\', '}', '$'];
    const CHOICE_TEXT_ESCAPE_CHARS: &[char] = &['\\', '|', ','];

    fn text<'a>(
        escape_chars: &'static [char],
        term_chars: &'static [char],
    ) -> impl Parser<'a, Output = String> {
        move |input: &'a str| {
            let mut chars = input.char_indices().peekable();
            let mut res = String::new();
            while let Some((i, c)) = chars.next() {
                match c {
                    '\\' => {
                        if let Some(&(_, c)) = chars.peek() {
                            if escape_chars.contains(&c) {
                                chars.next();
                                res.push(c);
                                continue;
                            }
                        }
                        res.push('\\');
                    }
                    c if term_chars.contains(&c) => return Ok((&input[i..], res)),
                    c => res.push(c),
                }
            }

            Ok(("", res))
        }
    }

    fn digit<'a>() -> impl Parser<'a, Output = usize> {
        filter_map(take_while(|c| c.is_ascii_digit()), |s| s.parse().ok())
    }

    fn case_change<'a>() -> impl Parser<'a, Output = CaseChange> {
        use CaseChange::*;

        choice!(
            map("upcase", |_| Upcase),
            map("downcase", |_| Downcase),
            map("capitalize", |_| Capitalize),
        )
    }

    fn format<'a>() -> impl Parser<'a, Output = FormatItem> {
        use FormatItem::*;

        choice!(
            // '$' int
            map(right("$", digit()), Capture),
            // '${' int '}'
            map(seq!("${", digit(), "}"), |seq| Capture(seq.1)),
            // '${' int ':' '/upcase' | '/downcase' | '/capitalize' '}'
            map(seq!("${", digit(), ":/", case_change(), "}"), |seq| {
                CaseChange(seq.1, seq.3)
            }),
            // '${' int ':+' if '}'
            map(
                seq!("${", digit(), ":+", text(TEXT_ESCAPE_CHARS, &['}']), "}"),
                |seq| { Conditional(seq.1, Some(seq.3), None) }
            ),
            // '${' int ':?' if ':' else '}'
            map(
                seq!(
                    "${",
                    digit(),
                    ":?",
                    text(TEXT_ESCAPE_CHARS, &[':']),
                    ":",
                    text(TEXT_ESCAPE_CHARS, &['}']),
                    "}"
                ),
                |seq| { Conditional(seq.1, Some(seq.3), Some(seq.5)) }
            ),
            // '${' int ':-' else '}' | '${' int ':' else '}'
            map(
                seq!(
                    "${",
                    digit(),
                    ":",
                    optional("-"),
                    text(TEXT_ESCAPE_CHARS, &['}']),
                    "}"
                ),
                |seq| { Conditional(seq.1, None, Some(seq.4)) }
            ),
        )
    }

    fn regex<'a>() -> impl Parser<'a, Output = Regex> {
        map(
            seq!(
                "/",
                // TODO parse as ECMAScript and convert to rust regex
                text(&['/'], &['/']),
                "/",
                zero_or_more(choice!(
                    format(),
                    // text doesn't parse $, if format fails we just accept the $ as text
                    map("$", |_| FormatItem::Text("$".into())),
                    map(text(&['\\', '/'], &['/', '$']), FormatItem::Text),
                )),
                "/",
                // vscode really doesn't allow escaping } here
                // so it's impossible to write a regex escape containing a }
                // we can consider deviating here and allowing the escape
                text(&[], &['}']),
            ),
            |(_, value, _, replacement, _, options)| Regex {
                value,
                replacement,
                options,
            },
        )
    }

    fn tabstop<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            or(
                right("$", digit()),
                map(seq!("${", digit(), "}"), |values| values.1),
            ),
            |digit| SnippetElement::Tabstop { tabstop: digit },
        )
    }

    fn placeholder<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            seq!(
                "${",
                digit(),
                ":",
                // according to the grammar there is just a single anything here.
                // However in the prose it is explained that placeholders can be nested.
                // The example there contains both a placeholder text and a nested placeholder
                // which indicates a list. Looking at the VSCode sourcecode, the placeholder
                // is indeed parsed as zero_or_more so the grammar is simply incorrect here
                zero_or_more(anything(TEXT_ESCAPE_CHARS, true)),
                "}"
            ),
            |seq| SnippetElement::Placeholder {
                tabstop: seq.1,
                value: seq.3,
            },
        )
    }

    fn choice<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        map(
            seq!(
                "${",
                digit(),
                "|",
                sep(text(CHOICE_TEXT_ESCAPE_CHARS, &['|', ',']), ","),
                "|}",
            ),
            |seq| SnippetElement::Choice {
                tabstop: seq.1,
                choices: seq.3,
            },
        )
    }

    fn variable<'a>() -> impl Parser<'a, Output = SnippetElement<'a>> {
        choice!(
            // $var
            map(right("$", var()), |name| SnippetElement::Variable {
                name,
                default: None,
                regex: None,
            }),
            // ${var}
            map(seq!("${", var(), "}",), |values| SnippetElement::Variable {
                name: values.1,
                default: None,
                regex: None,
            }),
            // ${var:default}
            map(
                seq!(
                    "${",
                    var(),
                    ":",
                    zero_or_more(anything(TEXT_ESCAPE_CHARS, true)),
                    "}",
                ),
                |values| SnippetElement::Variable {
                    name: values.1,
                    default: Some(values.3),
                    regex: None,
                }
            ),
            // ${var/value/format/options}
            map(seq!("${", var(), regex(), "}"), |values| {
                SnippetElement::Variable {
                    name: values.1,
                    default: None,
                    regex: Some(values.2),
                }
            }),
        )
    }

    fn anything<'a>(
        escape_chars: &'static [char],
        end_at_brace: bool,
    ) -> impl Parser<'a, Output = SnippetElement<'a>> {
        let term_chars: &[_] = if end_at_brace { &['$', '}'] } else { &['$'] };
        move |input: &'a str| {
            let parser = choice!(
                tabstop(),
                placeholder(),
                choice(),
                variable(),
                map("$", |_| SnippetElement::Text("$".into())),
                map(text(escape_chars, term_chars), SnippetElement::Text),
            );
            parser.parse(input)
        }
    }

    fn snippet<'a>() -> impl Parser<'a, Output = Snippet<'a>> {
        map(one_or_more(anything(TEXT_ESCAPE_CHARS, false)), |parts| {
            Snippet { elements: parts }
        })
    }

    pub fn parse(s: &str) -> Result<Snippet, &str> {
        snippet().parse(s).and_then(|(remainder, snippet)| {
            if remainder.is_empty() {
                Ok(snippet)
            } else {
                Err(remainder)
            }
        })
    }
}
