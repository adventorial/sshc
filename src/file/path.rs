use std::{fmt, path::PathBuf};

/// [ssh_config(5)](https://linux.die.net/man/5/ssh_config) file is a sequence of entries, an entry is either a *keyword argument* line, or a *comment* line.
///
/// In contrary to man, we will be explicitly distinguishing comments and empty lines, though originally
/// empty lines are considered comments.
///
/// This is an example of ssh_config file.
///
/// ```ssh-config
/// # [% sshc 0.0.1 %]
///
/// Include dir/folder/file
///
/// # a simple entry
/// Host example.com ssh.example.com
///     Port 22
///     User root
///     Ciphers aes256-cbc,arcfour
/// ```
#[derive(PartialEq, Debug)]
pub struct File {
    /// Vector of ssh_config lines.
    ///
    /// Please, note that '\n'-terminated ssh_config file always contains empty line in the end.
    pub lines: Vec<Line>,
    /// Original path of ssh_config file.
    ///
    /// It is used to be able to restore the source of entries provided by this file.
    pub path: Option<PathBuf>,
}

impl fmt::Display for File {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.lines
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<String>>()
                .join("")
        )
    }
}

/// Line is a sequence of characters in ssh_config file followed by /n or /r/n.
#[derive(PartialEq, Debug)]
pub struct Line {
    /// Indent prefix is the longest possible line prefix consisting of whitespace symbols.
    ///
    /// It is convenient to keep it to be able to restore original line formatting.
    pub indent_prefix: WhitespaceString,
    /// Expression is an optional sequence of characters between [`indent_prefix`] and [`indent_suffix`].
    ///
    /// See [`Expression`] for details.
    pub expression: Expression,
    /// Indent suffix is the longest possible line suffix consisting of whitespace symbols, not intersecting with [`indent_prefix`].
    ///
    /// Though keeping [`indent_prefix`] may seem useless, it is stored for the purpose of idempotent
    /// reading and writing ssh_config files.
    pub indent_suffix: WhitespaceString,
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}\n",
            self.indent_prefix, self.expression, self.indent_suffix
        )
    }
}

#[derive(PartialEq, Debug)]
pub enum Expression {
    /// Configuration options is an entry of a keyword-argument format.
    ConfigurationOptions {
        /// Keyword is a case-insensitive token consisting of `[A-Za-z]` and corresponding to the field being set.
        keyword: String,
        /// Separator is a string splitting keyword and arguments.
        ///
        /// Separator can be represented exactly in two different ways:
        /// - an arbitrary sequence of whitespace symbols (e.g., `Keyword argument`);
        /// - equals (`=`) sign preceded or/and followed by an arbitrary sequences of whitespace symbols (e.g. `Keyword=argument`, `Keyword = argument`).
        separator: String,
        /// Arguments expression is a sequence of argument tokens.
        ///
        /// See [`ArgumentTokenSequence`] for details.
        arguments_expression: ArgumentTokenSequence,
    },
    /// Comment is a string starting with hash (`#`) symbol.
    ///
    /// Please, note that genuine understanding of comment lines in [ssh_config(5)](https://linux.die.net/man/5/ssh_config)
    /// is different from what we call a comment, because we distinguish [`Empty`] expression as a separate case, not as a comment.
    Comment(String),
    /// Empty expression is an empty string.
    Empty,
    /// Any string not being a valid [`ConfigurationOptions`], [`Comment`] or [`Empty`] expression.
    Malformed(String),
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfigurationOptions {
                keyword,
                separator,
                arguments_expression,
            } => write!(
                f,
                "{}{}{}",
                keyword,
                separator,
                arguments_expression
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec::<String>>()
                    .join("")
            ),
            Expression::Comment(comment) => write!(f, "{}", comment),
            Expression::Empty => write!(f, ""),
            Expression::Malformed(malformed) => write!(f, "{}", malformed),
        }
    }
}

/// Arguments expression is a sequence of argument tokens.
///
/// It must contain at least one non-[`Whitespace`] token.
/// It may not contain several [`Pure`] / [`Quoted`] or [`Whitespace`] tokens in a row.
type ArgumentTokenSequence = Vec<ArgumentToken>;

/// Argument token is a part of arguments expression string.
///
/// More specifically, it is either an argument, or a whitespace sequence splitting one argument from another.
#[derive(PartialEq, Debug, Clone)]
pub enum ArgumentToken {
    /// Pure token is a value that may not contain whitespace characters and quoting characters.
    Pure(String),
    /// Quoted token is a value quoted with double-quote (`"`) symbol.
    ///
    /// It may contain whitespace characters inside, but double-quote characters must be escaped (`\"`).
    Quoted(String),
    /// Whitespace token is a separator for [`Pure`] and [`Quoted`] tokens.
    Whitespace(WhitespaceString),
}

impl fmt::Display for ArgumentToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArgumentToken::Pure(value) => write!(f, "{}", value),
            ArgumentToken::Quoted(value) => write!(f, "\"{}\"", value),
            ArgumentToken::Whitespace(value) => write!(f, "{}", value),
        }
    }
}

/// Whitespace string is a string consisting only of space (`' '`) or tabular (`'\t'`) symbols
type WhitespaceString = String;
