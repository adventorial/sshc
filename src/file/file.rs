use std::{fs, path::PathBuf};

use super::{ArgumentToken, Expression, File, Line};

pub fn read_ssh_config(path: &PathBuf) -> std::io::Result<File> {
    fs::read_to_string(path).map(|content| parse_ssh_config(content.as_str(), Some(path.clone())))
}

pub fn parse_ssh_config(content: &str, path: Option<PathBuf>) -> File {
    let mut lines = Vec::<Line>::new();
    for line in content.lines() {
        lines.push(parse_line(line));
    }
    File { lines, path }
}

pub fn write_ssh_config(file: &File, path: PathBuf) -> std::io::Result<()> {
    fs::write(path, file.to_string())
}

fn parse_line(line: &str) -> Line {
    let content = line.trim_end_matches(['\n', '\r']);
    let mut it = content.chars();
    if it.clone().any(|c| c == '\n') {
        panic!("multiline string can not be parsed as a single line");
    }
    let indent_prefix: String = it.by_ref().take_while(|s| s.is_whitespace()).collect();
    let indent_suffix_len = it.by_ref().rev().take_while(|s| s.is_whitespace()).count();
    let indent_suffix: String = content[(content.len() - indent_suffix_len)..].to_string();

    Line {
        indent_prefix,
        expression: parse_expression(line.trim()),
        indent_suffix,
    }
}

fn parse_expression(content: &str) -> Expression {
    if content.is_empty() {
        return Expression::Empty;
    } else if content.starts_with('#') {
        return Expression::Comment(content.to_string());
    }

    let keyword: String = content.chars().take_while(|c| c.is_alphabetic()).collect();

    if keyword.is_empty() {
        return Expression::Malformed(content.to_string());
    }

    let separator: String = content
        .chars()
        .skip(keyword.len())
        .take_while(|c| c.is_whitespace() || c == &'=')
        .collect();

    if !separator.is_empty()
        && (separator.as_str().trim().is_empty() || separator.as_str().trim() == "=")
    {
        if let Some(arguments_expression) =
            parse_arguments_expression(content[(keyword.len() + separator.len())..].trim())
        {
            return Expression::ConfigurationOptions {
                keyword,
                separator,
                arguments_expression,
            };
        } else {
            return Expression::Malformed(content.to_string());
        }
    }

    Expression::Malformed(content.to_string())
}

fn parse_arguments_expression(content: &str) -> Option<Vec<ArgumentToken>> {
    let mut arguments_expression = Vec::<ArgumentToken>::new();
    let mut remaining = content;

    while !remaining.is_empty() {
        if remaining.starts_with(|c: char| c.is_whitespace()) {
            let argument: String = remaining
                .chars()
                .take_while(|c| c.is_whitespace())
                .collect();
            remaining = &remaining[argument.len()..];
            arguments_expression.push(ArgumentToken::Whitespace(argument));
        } else if remaining.starts_with('"') {
            let mut prev = '\0';
            let mut token_size = 0;
            let mut found_end_quote = false;
            for c in remaining.chars().skip(1) {
                if prev != '\\' && c == '"' {
                    found_end_quote = true;
                    break;
                }
                token_size += 1;
                prev = c;
            }

            if found_end_quote {
                let argument: String = remaining.chars().skip(1).take(token_size).collect();
                remaining = &remaining[(1 + argument.len() + 1)..];
                arguments_expression.push(ArgumentToken::Quoted(argument));
            } else {
                return None;
            }
        } else {
            let argument: String = remaining
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();

            if argument.contains('#') {
                return None;
            }

            remaining = &remaining[argument.len()..];
            arguments_expression.push(ArgumentToken::Pure(argument));
        }
    }

    if arguments_expression.is_empty() {
        None
    } else {
        Some(arguments_expression)
    }
}

#[cfg(test)]
mod tests {
    use tempfile::{tempfile, NamedTempFile};

    use super::*;

    const VALID_INDENTS: &[&str] = &["", " ", "\t", "\t ", " \t", "\t\t", "  "];
    const VALID_SEPARATORS: &[&str] = &[
        " ",
        "\t",
        "\t ",
        " \t",
        "\t\t",
        "=",
        " =",
        "= ",
        " = ",
        "  =  ",
        "\t=",
        "=\t",
        "\t \t= \t\t",
    ];

    fn test_correct_line(
        keyword: &str,
        arguments_expression: &str,
        arguments_expected: Vec<ArgumentToken>,
    ) {
        for &indent_prefix in VALID_INDENTS {
            for &separator in VALID_SEPARATORS {
                for &indent_suffix in VALID_INDENTS {
                    let expected = format!(
                        "{}{}{}{}{}\n",
                        indent_prefix, keyword, separator, arguments_expression, indent_suffix
                    );
                    let actual = parse_line(expected.as_str());
                    assert_eq!(
                        actual,
                        Line {
                            indent_prefix: indent_prefix.to_string(),
                            expression: Expression::ConfigurationOptions {
                                keyword: keyword.to_string(),
                                separator: separator.to_string(),
                                arguments_expression: arguments_expected.clone()
                            },
                            indent_suffix: indent_suffix.to_string(),
                        }
                    );
                    assert_eq!(expected, actual.to_string());
                }
            }
        }
    }

    fn test_malformed_line(line: &str) {
        for &indent_prefix in VALID_INDENTS {
            for &indent_suffix in VALID_INDENTS {
                let expected = format!("{}{}{}\n", indent_prefix, line, indent_suffix);
                let actual = parse_line(expected.as_str());
                assert_eq!(
                    actual,
                    Line {
                        indent_prefix: indent_prefix.to_string(),
                        expression: Expression::Malformed(line.to_string()),
                        indent_suffix: indent_suffix.to_string(),
                    }
                );
                assert_eq!(expected, actual.to_string());
            }
        }
    }

    #[test]
    fn smoke_parse_correct_line_test() {
        test_correct_line(
            "Host",
            "example.com",
            vec![ArgumentToken::Pure("example.com".to_string())],
        );
        test_correct_line("Host", "*", vec![ArgumentToken::Pure("*".to_string())]);
        test_correct_line(
            "Host",
            "lol!*.com example.com \t !kek!",
            vec![
                ArgumentToken::Pure("lol!*.com".to_string()),
                ArgumentToken::Whitespace(" ".to_string()),
                ArgumentToken::Pure("example.com".to_string()),
                ArgumentToken::Whitespace(" \t ".to_string()),
                ArgumentToken::Pure("!kek!".to_string()),
            ],
        );
        test_correct_line(
            "Host",
            "\"*\"",
            vec![ArgumentToken::Quoted("*".to_string())],
        );
        test_correct_line(
            "Host",
            "\"hello # \\\" lol \"",
            vec![ArgumentToken::Quoted("hello # \\\" lol ".to_string())],
        );
        test_correct_line(
            "Host",
            "lol!*.com \"example.com\" \t !kek!",
            vec![
                ArgumentToken::Pure("lol!*.com".to_string()),
                ArgumentToken::Whitespace(" ".to_string()),
                ArgumentToken::Quoted("example.com".to_string()),
                ArgumentToken::Whitespace(" \t ".to_string()),
                ArgumentToken::Pure("!kek!".to_string()),
            ],
        );
    }

    #[test]
    fn malformed_lines_test() {
        const INVALID_EXPRESSIONS: &[&str] = &[
            "Host",
            "Host0 example.com",
            "Host # kek",
            "123",
            "Ho#st kek",
            "Host #kek",
            "Host k#k",
            "Host kek #",
            "Host ke\"k # lol",
            "Host \"lol",
        ];
        for &invalid_expression in INVALID_EXPRESSIONS {
            test_malformed_line(invalid_expression);
        }
    }

    #[test]
    fn deserialization_test() {
        let config = "# a comment\n\
                            \t # one more comment \t\r\n\
                            \t Host example.com \n\
                            User root\n\
                            \n"
        .to_string();
        let mut tmp_file = NamedTempFile::new().unwrap();

        let ssh_config_file1 =
            parse_ssh_config(config.as_str(), Some(tmp_file.path().to_path_buf()));

        use std::io::Write;
        write!(tmp_file, "{}", config).unwrap();

        let ssh_config_file2 = read_ssh_config(&tmp_file.path().to_path_buf()).unwrap();
        assert_eq!(ssh_config_file1, ssh_config_file2);
    }
}
