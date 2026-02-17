use ag_ast::Span;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    Const,
    Mut,
    If,
    Else,
    For,
    In,
    Of,
    While,
    Match,
    Ret,
    Yield,
    Await,
    Async,
    Import,
    Export,
    From,
    As,
    Type,
    Struct,
    Enum,
    Impl,
    Pub,
    SelfKw,
    True,
    False,
    Nil,
    Use,
    With,
    On,
    Underscore,
    Try,
    Catch,
    Extern,

    // Literals
    Ident(String),
    IntLiteral(String),
    FloatLiteral(String),
    StringLiteral(String),

    // Template strings
    TemplateNoSub(String),
    TemplateHead(String),
    TemplateMiddle(String),
    TemplateTail(String),

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    StarStar,
    EqEq,
    BangEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    AmpAmp,
    PipePipe,
    Bang,
    Pipe,
    PipeGt,
    QuestionQuestion,
    QuestionDot,
    Eq,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    FatArrow,
    ThinArrow,
    ColonColon,
    At,
    DotDot,
    DotDotDot,

    // Punctuation
    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Colon,
    Dot,
    Question,

    // Comments
    LineComment(String),
    BlockComment(String),
    DocComment(String),

    // DSL tokens
    DslBlockStart,
    DslBlockEnd,
    DslText(String),
    DslCaptureStart,
    DslCaptureEnd,

    // Special
    Eof,
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
    pub text: String,
}

pub struct Lexer<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    template_depth_stack: Vec<u32>,
    dsl_raw_mode: bool,
    dsl_capture_depth: u32,
    dsl_block_start_pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            template_depth_stack: Vec::new(),
            dsl_raw_mode: false,
            dsl_capture_depth: 0,
            dsl_block_start_pos: 0,
        }
    }

    pub fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token();
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.bytes.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.bytes.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == b' ' || ch == b'\t' || ch == b'\r' || ch == b'\n' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Called by the parser to enter DSL raw mode.
    /// Expects ``` followed by newline; emits DslBlockStart.
    pub fn enter_dsl_raw_mode(&mut self) -> Token {
        self.skip_whitespace_no_newline();
        let start = self.pos;

        // Check for ```
        if self.peek() == Some(b'`') && self.peek_at(1) == Some(b'`') && self.peek_at(2) == Some(b'`') {
            self.pos += 3;
            // Skip rest of line (allow trailing whitespace/content until newline)
            while let Some(ch) = self.peek() {
                if ch == b'\n' {
                    self.pos += 1;
                    break;
                }
                self.pos += 1;
            }
            self.dsl_raw_mode = true;
            self.dsl_block_start_pos = start;
            Token {
                kind: TokenKind::DslBlockStart,
                span: Span::new(start as u32, self.pos as u32),
                text: self.source[start..self.pos].to_string(),
            }
        } else {
            Token {
                kind: TokenKind::Error("expected ``` to open DSL block".to_string()),
                span: Span::new(start as u32, self.pos as u32),
                text: String::new(),
            }
        }
    }

    fn skip_whitespace_no_newline(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == b' ' || ch == b'\t' || ch == b'\r' {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn lex_dsl_raw(&mut self) -> Token {
        let start = self.pos;
        let mut text = String::new();

        loop {
            match self.peek() {
                None => {
                    // Unterminated DSL block
                    if !text.is_empty() {
                        // First emit the accumulated text
                        return Token {
                            kind: TokenKind::DslText(text),
                            span: Span::new(start as u32, self.pos as u32),
                            text: self.source[start..self.pos].to_string(),
                        };
                    }
                    self.dsl_raw_mode = false;
                    return Token {
                        kind: TokenKind::Error("unterminated DSL block".to_string()),
                        span: Span::new(self.dsl_block_start_pos as u32, self.pos as u32),
                        text: String::new(),
                    };
                }
                Some(b'#') if self.peek_at(1) == Some(b'{') => {
                    // Capture start: emit accumulated text first if any
                    if !text.is_empty() {
                        return Token {
                            kind: TokenKind::DslText(text),
                            span: Span::new(start as u32, self.pos as u32),
                            text: self.source[start..self.pos].to_string(),
                        };
                    }
                    let cap_start = self.pos;
                    self.pos += 2; // consume '#{'
                    self.dsl_raw_mode = false;
                    self.dsl_capture_depth = 1;
                    return Token {
                        kind: TokenKind::DslCaptureStart,
                        span: Span::new(cap_start as u32, self.pos as u32),
                        text: "#{".to_string(),
                    };
                }
                Some(b'`') if self.peek_at(1) == Some(b'`') && self.peek_at(2) == Some(b'`') => {
                    // Check if ``` is at line start (only whitespace before it on this line)
                    if self.is_backticks_at_line_start() {
                        if !text.is_empty() {
                            return Token {
                                kind: TokenKind::DslText(text),
                                span: Span::new(start as u32, self.pos as u32),
                                text: self.source[start..self.pos].to_string(),
                            };
                        }
                        let end_start = self.pos;
                        self.pos += 3;
                        self.dsl_raw_mode = false;
                        return Token {
                            kind: TokenKind::DslBlockEnd,
                            span: Span::new(end_start as u32, self.pos as u32),
                            text: "```".to_string(),
                        };
                    }
                    // Mid-line backticks: treat as text
                    text.push(self.source[self.pos..].chars().next().unwrap());
                    self.pos += 1;
                }
                Some(_) => {
                    let ch = self.source[self.pos..].chars().next().unwrap();
                    text.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn is_backticks_at_line_start(&self) -> bool {
        // Walk backwards from current pos to find the start of the current line
        let mut i = self.pos;
        while i > 0 {
            let prev = self.bytes[i - 1];
            if prev == b'\n' {
                break;
            }
            if prev != b' ' && prev != b'\t' && prev != b'\r' {
                return false;
            }
            i -= 1;
        }
        true
    }

    pub fn next_token(&mut self) -> Token {
        // DSL raw mode: scan raw text
        if self.dsl_raw_mode {
            return self.lex_dsl_raw();
        }

        // DSL capture mode: track brace nesting, emit DslCaptureEnd at outermost }
        if self.dsl_capture_depth > 0 {
            self.skip_whitespace();
            if self.peek() == Some(b'{') {
                self.dsl_capture_depth += 1;
            } else if self.peek() == Some(b'}') {
                self.dsl_capture_depth -= 1;
                if self.dsl_capture_depth == 0 {
                    let start = self.pos;
                    self.pos += 1;
                    self.dsl_raw_mode = true;
                    return Token {
                        kind: TokenKind::DslCaptureEnd,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "}".to_string(),
                    };
                }
            }
        }

        // If we're inside a template interpolation and hit '}', resume template lexing
        if let Some(depth) = self.template_depth_stack.last() {
            if *depth == 0 {
                if self.peek() == Some(b'}') {
                    self.pos += 1; // consume '}'
                    self.template_depth_stack.pop();
                    return self.lex_template_continuation();
                }
            }
        }

        self.skip_whitespace();

        let start = self.pos;

        let Some(ch) = self.peek() else {
            return Token {
                kind: TokenKind::Eof,
                span: Span::new(start as u32, start as u32),
                text: String::new(),
            };
        };

        match ch {
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident_or_keyword(start),
            b'0'..=b'9' => self.lex_number(start),
            b'"' => self.lex_string(start, b'"'),
            b'\'' => self.lex_string(start, b'\''),
            b'`' => self.lex_template_start(start),
            b'/' => self.lex_slash(start),
            _ => self.lex_punct_or_operator(start),
        }
    }

    fn lex_ident_or_keyword(&mut self, start: usize) -> Token {
        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == b'_' {
                self.pos += 1;
            } else {
                break;
            }
        }
        let text = &self.source[start..self.pos];
        let kind = match text {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "mut" => TokenKind::Mut,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "of" => TokenKind::Of,
            "while" => TokenKind::While,
            "match" => TokenKind::Match,
            "ret" => TokenKind::Ret,
            "yield" => TokenKind::Yield,
            "await" => TokenKind::Await,
            "async" => TokenKind::Async,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "from" => TokenKind::From,
            "as" => TokenKind::As,
            "type" => TokenKind::Type,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "pub" => TokenKind::Pub,
            "self" => TokenKind::SelfKw,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "use" => TokenKind::Use,
            "with" => TokenKind::With,
            "on" => TokenKind::On,
            "_" => TokenKind::Underscore,
            "try" => TokenKind::Try,
            "catch" => TokenKind::Catch,
            "extern" => TokenKind::Extern,
            _ => TokenKind::Ident(text.to_string()),
        };
        Token {
            kind,
            span: Span::new(start as u32, self.pos as u32),
            text: text.to_string(),
        }
    }

    fn lex_number(&mut self, start: usize) -> Token {
        // Consume digits
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }

        let mut is_float = false;

        // Check for decimal point (but not '..' range)
        if self.peek() == Some(b'.') && self.peek_at(1) != Some(b'.') {
            if let Some(next) = self.peek_at(1) {
                if next.is_ascii_digit() {
                    is_float = true;
                    self.pos += 1; // consume '.'
                    while let Some(ch) = self.peek() {
                        if ch.is_ascii_digit() {
                            self.pos += 1;
                        } else {
                            break;
                        }
                    }
                }
            }
        }

        // Check for exponent
        if self.peek() == Some(b'e') || self.peek() == Some(b'E') {
            is_float = true;
            self.pos += 1;
            if self.peek() == Some(b'+') || self.peek() == Some(b'-') {
                self.pos += 1;
            }
            while let Some(ch) = self.peek() {
                if ch.is_ascii_digit() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
        }

        let text = &self.source[start..self.pos];
        let kind = if is_float {
            TokenKind::FloatLiteral(text.to_string())
        } else {
            TokenKind::IntLiteral(text.to_string())
        };
        Token {
            kind,
            span: Span::new(start as u32, self.pos as u32),
            text: text.to_string(),
        }
    }

    fn lex_string(&mut self, start: usize, quote: u8) -> Token {
        self.pos += 1; // consume opening quote
        let mut value = String::new();
        loop {
            match self.peek() {
                None | Some(b'\n') => {
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::Error("unterminated string literal".to_string()),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.advance() {
                        Some(b'n') => value.push('\n'),
                        Some(b't') => value.push('\t'),
                        Some(b'r') => value.push('\r'),
                        Some(b'0') => value.push('\0'),
                        Some(b'\\') => value.push('\\'),
                        Some(b'\'') => value.push('\''),
                        Some(b'"') => value.push('"'),
                        Some(ch) => {
                            value.push('\\');
                            value.push(ch as char);
                        }
                        None => {}
                    }
                }
                Some(ch) if ch == quote => {
                    self.pos += 1; // consume closing quote
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::StringLiteral(value),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(ch) => {
                    value.push(ch as char);
                    self.pos += 1;
                }
            }
        }
    }

    fn lex_template_start(&mut self, start: usize) -> Token {
        self.pos += 1; // consume opening backtick
        let mut value = String::new();
        loop {
            match self.peek() {
                None => {
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::Error("unterminated template string".to_string()),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'`') => {
                    self.pos += 1; // consume closing backtick
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::TemplateNoSub(value),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'$') if self.peek_at(1) == Some(b'{') => {
                    self.pos += 2; // consume '${'
                    self.template_depth_stack.push(0);
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::TemplateHead(value),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.advance() {
                        Some(b'n') => value.push('\n'),
                        Some(b't') => value.push('\t'),
                        Some(b'r') => value.push('\r'),
                        Some(b'`') => value.push('`'),
                        Some(b'$') => value.push('$'),
                        Some(b'\\') => value.push('\\'),
                        Some(ch) => {
                            value.push('\\');
                            value.push(ch as char);
                        }
                        None => {}
                    }
                }
                Some(ch) => {
                    value.push(ch as char);
                    self.pos += 1;
                }
            }
        }
    }

    fn lex_template_continuation(&mut self) -> Token {
        let start = self.pos - 1; // include the '}' we already consumed
        let mut value = String::new();
        loop {
            match self.peek() {
                None => {
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::Error("unterminated template string".to_string()),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'`') => {
                    self.pos += 1;
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::TemplateTail(value),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'$') if self.peek_at(1) == Some(b'{') => {
                    self.pos += 2;
                    self.template_depth_stack.push(0);
                    let text = self.source[start..self.pos].to_string();
                    return Token {
                        kind: TokenKind::TemplateMiddle(value),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    };
                }
                Some(b'\\') => {
                    self.pos += 1;
                    match self.advance() {
                        Some(b'n') => value.push('\n'),
                        Some(b't') => value.push('\t'),
                        Some(b'r') => value.push('\r'),
                        Some(b'`') => value.push('`'),
                        Some(b'$') => value.push('$'),
                        Some(b'\\') => value.push('\\'),
                        Some(ch) => {
                            value.push('\\');
                            value.push(ch as char);
                        }
                        None => {}
                    }
                }
                Some(ch) => {
                    value.push(ch as char);
                    self.pos += 1;
                }
            }
        }
    }

    fn lex_slash(&mut self, start: usize) -> Token {
        self.pos += 1; // consume '/'
        match self.peek() {
            Some(b'/') => {
                self.pos += 1;
                // Check for doc comment '///'
                let is_doc = self.peek() == Some(b'/') && self.peek_at(1) != Some(b'/');
                let comment_start = start;
                while let Some(ch) = self.peek() {
                    if ch == b'\n' {
                        break;
                    }
                    self.pos += 1;
                }
                let text = self.source[comment_start..self.pos].to_string();
                let kind = if is_doc {
                    TokenKind::DocComment(text.clone())
                } else {
                    TokenKind::LineComment(text.clone())
                };
                Token {
                    kind,
                    span: Span::new(start as u32, self.pos as u32),
                    text,
                }
            }
            Some(b'*') => {
                self.pos += 1;
                let mut depth = 1u32;
                while depth > 0 {
                    match self.peek() {
                        None => break,
                        Some(b'/') if self.peek_at(1) == Some(b'*') => {
                            self.pos += 2;
                            depth += 1;
                        }
                        Some(b'*') if self.peek_at(1) == Some(b'/') => {
                            self.pos += 2;
                            depth -= 1;
                        }
                        _ => {
                            self.pos += 1;
                        }
                    }
                }
                let text = self.source[start..self.pos].to_string();
                Token {
                    kind: TokenKind::BlockComment(text.clone()),
                    span: Span::new(start as u32, self.pos as u32),
                    text,
                }
            }
            Some(b'=') => {
                self.pos += 1;
                Token {
                    kind: TokenKind::SlashEq,
                    span: Span::new(start as u32, self.pos as u32),
                    text: "/=".to_string(),
                }
            }
            _ => Token {
                kind: TokenKind::Slash,
                span: Span::new(start as u32, self.pos as u32),
                text: "/".to_string(),
            },
        }
    }

    fn lex_punct_or_operator(&mut self, start: usize) -> Token {
        // Handle multi-byte UTF-8 characters that aren't valid tokens
        let ch_char = self.source[self.pos..].chars().next().unwrap();
        if !ch_char.is_ascii() {
            self.pos += ch_char.len_utf8();
            let text = ch_char.to_string();
            return Token {
                kind: TokenKind::Error(text.clone()),
                span: Span::new(start as u32, self.pos as u32),
                text,
            };
        }
        let ch = self.advance().unwrap();

        // Track brace depth for template string nesting
        if ch == b'{' {
            if let Some(depth) = self.template_depth_stack.last_mut() {
                *depth += 1;
            }
            return Token {
                kind: TokenKind::LBrace,
                span: Span::new(start as u32, self.pos as u32),
                text: "{".to_string(),
            };
        }
        if ch == b'}' {
            if let Some(depth) = self.template_depth_stack.last_mut() {
                if *depth > 0 {
                    *depth -= 1;
                }
            }
            return Token {
                kind: TokenKind::RBrace,
                span: Span::new(start as u32, self.pos as u32),
                text: "}".to_string(),
            };
        }

        match ch {
            b'(' => Token {
                kind: TokenKind::LParen,
                span: Span::new(start as u32, self.pos as u32),
                text: "(".to_string(),
            },
            b')' => Token {
                kind: TokenKind::RParen,
                span: Span::new(start as u32, self.pos as u32),
                text: ")".to_string(),
            },
            b'[' => Token {
                kind: TokenKind::LBracket,
                span: Span::new(start as u32, self.pos as u32),
                text: "[".to_string(),
            },
            b']' => Token {
                kind: TokenKind::RBracket,
                span: Span::new(start as u32, self.pos as u32),
                text: "]".to_string(),
            },
            b',' => Token {
                kind: TokenKind::Comma,
                span: Span::new(start as u32, self.pos as u32),
                text: ",".to_string(),
            },
            b';' => Token {
                kind: TokenKind::Semi,
                span: Span::new(start as u32, self.pos as u32),
                text: ";".to_string(),
            },
            b':' => {
                if self.peek() == Some(b':') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::ColonColon,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "::".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Colon,
                        span: Span::new(start as u32, self.pos as u32),
                        text: ":".to_string(),
                    }
                }
            }
            b'.' => {
                if self.peek() == Some(b'.') {
                    self.pos += 1;
                    if self.peek() == Some(b'.') {
                        self.pos += 1;
                        Token {
                            kind: TokenKind::DotDotDot,
                            span: Span::new(start as u32, self.pos as u32),
                            text: "...".to_string(),
                        }
                    } else {
                        Token {
                            kind: TokenKind::DotDot,
                            span: Span::new(start as u32, self.pos as u32),
                            text: "..".to_string(),
                        }
                    }
                } else {
                    Token {
                        kind: TokenKind::Dot,
                        span: Span::new(start as u32, self.pos as u32),
                        text: ".".to_string(),
                    }
                }
            }
            b'?' => {
                if self.peek() == Some(b'.') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::QuestionDot,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "?.".to_string(),
                    }
                } else if self.peek() == Some(b'?') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::QuestionQuestion,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "??".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Question,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "?".to_string(),
                    }
                }
            }
            b'@' => Token {
                kind: TokenKind::At,
                span: Span::new(start as u32, self.pos as u32),
                text: "@".to_string(),
            },
            b'+' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::PlusEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "+=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Plus,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "+".to_string(),
                    }
                }
            }
            b'-' => {
                if self.peek() == Some(b'>') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::ThinArrow,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "->".to_string(),
                    }
                } else if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::MinusEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "-=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Minus,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "-".to_string(),
                    }
                }
            }
            b'*' => {
                if self.peek() == Some(b'*') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::StarStar,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "**".to_string(),
                    }
                } else if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::StarEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "*=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Star,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "*".to_string(),
                    }
                }
            }
            b'%' => Token {
                kind: TokenKind::Percent,
                span: Span::new(start as u32, self.pos as u32),
                text: "%".to_string(),
            },
            b'=' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::EqEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "==".to_string(),
                    }
                } else if self.peek() == Some(b'>') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::FatArrow,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "=>".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Eq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "=".to_string(),
                    }
                }
            }
            b'!' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::BangEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "!=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Bang,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "!".to_string(),
                    }
                }
            }
            b'<' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::LtEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "<=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Lt,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "<".to_string(),
                    }
                }
            }
            b'>' => {
                if self.peek() == Some(b'=') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::GtEq,
                        span: Span::new(start as u32, self.pos as u32),
                        text: ">=".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Gt,
                        span: Span::new(start as u32, self.pos as u32),
                        text: ">".to_string(),
                    }
                }
            }
            b'&' => {
                if self.peek() == Some(b'&') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::AmpAmp,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "&&".to_string(),
                    }
                } else {
                    let text = self.source[start..self.pos].to_string();
                    Token {
                        kind: TokenKind::Error(text.clone()),
                        span: Span::new(start as u32, self.pos as u32),
                        text,
                    }
                }
            }
            b'|' => {
                if self.peek() == Some(b'|') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::PipePipe,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "||".to_string(),
                    }
                } else if self.peek() == Some(b'>') {
                    self.pos += 1;
                    Token {
                        kind: TokenKind::PipeGt,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "|>".to_string(),
                    }
                } else {
                    Token {
                        kind: TokenKind::Pipe,
                        span: Span::new(start as u32, self.pos as u32),
                        text: "|".to_string(),
                    }
                }
            }
            _ => {
                // Error recovery: unknown character
                let text = self.source[start..self.pos].to_string();
                Token {
                    kind: TokenKind::Error(text.clone()),
                    span: Span::new(start as u32, self.pos as u32),
                    text,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        Lexer::tokenize(src)
            .into_iter()
            .filter(|t| {
                !matches!(
                    t.kind,
                    TokenKind::Eof
                        | TokenKind::LineComment(_)
                        | TokenKind::BlockComment(_)
                        | TokenKind::DocComment(_)
                )
            })
            .map(|t| t.kind)
            .collect()
    }

    #[test]
    fn keyword_vs_ident() {
        assert_eq!(
            kinds("let x = fn_name"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::Ident("fn_name".into()),
            ]
        );
    }

    #[test]
    fn ident_with_keyword_prefix() {
        assert_eq!(kinds("letter"), vec![TokenKind::Ident("letter".into())]);
    }

    #[test]
    fn int_literal() {
        assert_eq!(kinds("42"), vec![TokenKind::IntLiteral("42".into())]);
    }

    #[test]
    fn float_literal() {
        assert_eq!(kinds("3.14"), vec![TokenKind::FloatLiteral("3.14".into())]);
    }

    #[test]
    fn exponent_notation() {
        assert_eq!(
            kinds("2.5e-3"),
            vec![TokenKind::FloatLiteral("2.5e-3".into())]
        );
    }

    #[test]
    fn double_quoted_string() {
        assert_eq!(
            kinds(r#""hello world""#),
            vec![TokenKind::StringLiteral("hello world".into())]
        );
    }

    #[test]
    fn escape_sequences() {
        assert_eq!(
            kinds(r#""line1\nline2""#),
            vec![TokenKind::StringLiteral("line1\nline2".into())]
        );
    }

    #[test]
    fn unterminated_string() {
        let tokens = kinds(r#""hello"#);
        assert!(matches!(tokens[0], TokenKind::Error(_)));
    }

    #[test]
    fn template_no_sub() {
        assert_eq!(
            kinds("`hello world`"),
            vec![TokenKind::TemplateNoSub("hello world".into())]
        );
    }

    #[test]
    fn template_with_interpolation() {
        assert_eq!(
            kinds("`hello ${name}!`"),
            vec![
                TokenKind::TemplateHead("hello ".into()),
                TokenKind::Ident("name".into()),
                TokenKind::TemplateTail("!".into()),
            ]
        );
    }

    #[test]
    fn template_multiple_interpolations() {
        assert_eq!(
            kinds("`${a} + ${b} = ${c}`"),
            vec![
                TokenKind::TemplateHead("".into()),
                TokenKind::Ident("a".into()),
                TokenKind::TemplateMiddle(" + ".into()),
                TokenKind::Ident("b".into()),
                TokenKind::TemplateMiddle(" = ".into()),
                TokenKind::Ident("c".into()),
                TokenKind::TemplateTail("".into()),
            ]
        );
    }

    #[test]
    fn pipe_operator() {
        assert_eq!(
            kinds("a |> b"),
            vec![
                TokenKind::Ident("a".into()),
                TokenKind::PipeGt,
                TokenKind::Ident("b".into()),
            ]
        );
    }

    #[test]
    fn arrow_operators() {
        assert_eq!(kinds("=> ->"), vec![TokenKind::FatArrow, TokenKind::ThinArrow]);
    }

    #[test]
    fn optional_chaining() {
        assert_eq!(
            kinds("x?.y"),
            vec![
                TokenKind::Ident("x".into()),
                TokenKind::QuestionDot,
                TokenKind::Ident("y".into()),
            ]
        );
    }

    #[test]
    fn whitespace_skipped() {
        assert_eq!(
            kinds("let   x  =  42"),
            vec![
                TokenKind::Let,
                TokenKind::Ident("x".into()),
                TokenKind::Eq,
                TokenKind::IntLiteral("42".into()),
            ]
        );
    }

    #[test]
    fn line_comment() {
        let tokens = Lexer::tokenize("x // this is a comment\ny");
        let comment = tokens.iter().find(|t| matches!(t.kind, TokenKind::LineComment(_)));
        assert!(comment.is_some());
    }

    #[test]
    fn doc_comment() {
        let tokens = Lexer::tokenize("/// Docs for next item\nfn foo() {}");
        let doc = tokens.iter().find(|t| matches!(t.kind, TokenKind::DocComment(_)));
        assert!(doc.is_some());
    }

    #[test]
    fn block_comment() {
        let tokens = Lexer::tokenize("x /* block */ y");
        let block = tokens.iter().find(|t| matches!(t.kind, TokenKind::BlockComment(_)));
        assert!(block.is_some());
    }

    #[test]
    fn error_recovery() {
        let tokens = kinds("let x = 42 \u{00a7} y");
        assert!(tokens.iter().any(|t| matches!(t, TokenKind::Error(_))));
        assert!(tokens.iter().any(|t| *t == TokenKind::Ident("y".into())));
    }

    #[test]
    fn all_comparison_ops() {
        assert_eq!(
            kinds("== != < > <= >="),
            vec![
                TokenKind::EqEq,
                TokenKind::BangEq,
                TokenKind::Lt,
                TokenKind::Gt,
                TokenKind::LtEq,
                TokenKind::GtEq,
            ]
        );
    }

    #[test]
    fn assignment_ops() {
        assert_eq!(
            kinds("+= -= *= /="),
            vec![
                TokenKind::PlusEq,
                TokenKind::MinusEq,
                TokenKind::StarEq,
                TokenKind::SlashEq,
            ]
        );
    }

    #[test]
    fn range_and_spread() {
        assert_eq!(kinds(".. ..."), vec![TokenKind::DotDot, TokenKind::DotDotDot]);
    }

    #[test]
    fn double_colon() {
        assert_eq!(
            kinds("Enum::Variant"),
            vec![
                TokenKind::Ident("Enum".into()),
                TokenKind::ColonColon,
                TokenKind::Ident("Variant".into()),
            ]
        );
    }

    // ── DSL lexer tests ──

    #[test]
    fn dsl_at_sign_token() {
        assert_eq!(
            kinds("@prompt system"),
            vec![
                TokenKind::At,
                TokenKind::Ident("prompt".into()),
                TokenKind::Ident("system".into()),
            ]
        );
    }

    #[test]
    fn dsl_raw_mode_plain_text() {
        let mut lexer = Lexer::new("```\nYou are a helpful assistant.\n```\n");
        let start_tok = lexer.enter_dsl_raw_mode();
        assert_eq!(start_tok.kind, TokenKind::DslBlockStart);
        let text_tok = lexer.next_token();
        assert_eq!(text_tok.kind, TokenKind::DslText("You are a helpful assistant.\n".into()));
        let end_tok = lexer.next_token();
        assert_eq!(end_tok.kind, TokenKind::DslBlockEnd);
    }

    #[test]
    fn dsl_single_capture() {
        let mut lexer = Lexer::new("```\nHello #{name}!\n```\n");
        let _ = lexer.enter_dsl_raw_mode();
        let t1 = lexer.next_token();
        assert_eq!(t1.kind, TokenKind::DslText("Hello ".into()));
        let t2 = lexer.next_token();
        assert_eq!(t2.kind, TokenKind::DslCaptureStart);
        let t3 = lexer.next_token();
        assert_eq!(t3.kind, TokenKind::Ident("name".into()));
        let t4 = lexer.next_token();
        assert_eq!(t4.kind, TokenKind::DslCaptureEnd);
        let t5 = lexer.next_token();
        assert_eq!(t5.kind, TokenKind::DslText("!\n".into()));
        let t6 = lexer.next_token();
        assert_eq!(t6.kind, TokenKind::DslBlockEnd);
    }

    #[test]
    fn dsl_multiple_captures() {
        let mut lexer = Lexer::new("```\n#{a} and #{b}\n```\n");
        let _ = lexer.enter_dsl_raw_mode();
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureStart);
        assert_eq!(lexer.next_token().kind, TokenKind::Ident("a".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureEnd);
        assert_eq!(lexer.next_token().kind, TokenKind::DslText(" and ".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureStart);
        assert_eq!(lexer.next_token().kind, TokenKind::Ident("b".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureEnd);
        assert_eq!(lexer.next_token().kind, TokenKind::DslText("\n".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslBlockEnd);
    }

    #[test]
    fn dsl_hash_not_followed_by_brace() {
        let mut lexer = Lexer::new("```\n## Heading\n#{expr}\n```\n");
        let _ = lexer.enter_dsl_raw_mode();
        assert_eq!(lexer.next_token().kind, TokenKind::DslText("## Heading\n".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureStart);
        assert_eq!(lexer.next_token().kind, TokenKind::Ident("expr".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureEnd);
        assert_eq!(lexer.next_token().kind, TokenKind::DslText("\n".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslBlockEnd);
    }

    #[test]
    fn dsl_nested_braces_in_capture() {
        let mut lexer = Lexer::new("```\n#{a + { x: 1 }}\n```\n");
        let _ = lexer.enter_dsl_raw_mode();
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureStart);
        // Tokens inside capture: a + { x : 1 }
        assert_eq!(lexer.next_token().kind, TokenKind::Ident("a".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::Plus);
        assert_eq!(lexer.next_token().kind, TokenKind::LBrace);
        assert_eq!(lexer.next_token().kind, TokenKind::Ident("x".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::Colon);
        assert_eq!(lexer.next_token().kind, TokenKind::IntLiteral("1".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::RBrace);
        assert_eq!(lexer.next_token().kind, TokenKind::DslCaptureEnd);
    }

    #[test]
    fn dsl_unterminated_block() {
        let mut lexer = Lexer::new("```\n  content\n");
        let _ = lexer.enter_dsl_raw_mode();
        let t1 = lexer.next_token();
        assert_eq!(t1.kind, TokenKind::DslText("  content\n".into()));
        let t2 = lexer.next_token();
        assert!(matches!(t2.kind, TokenKind::Error(ref s) if s.contains("unterminated")));
    }

    #[test]
    fn dsl_backticks_midline_not_block_end() {
        let mut lexer = Lexer::new("```\nuse ``` in code\n```\n");
        let _ = lexer.enter_dsl_raw_mode();
        assert_eq!(lexer.next_token().kind, TokenKind::DslText("use ``` in code\n".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslBlockEnd);
    }

    #[test]
    fn dsl_indented_block_end() {
        let mut lexer = Lexer::new("```\n  content\n  ```\n");
        let _ = lexer.enter_dsl_raw_mode();
        assert_eq!(lexer.next_token().kind, TokenKind::DslText("  content\n  ".into()));
        assert_eq!(lexer.next_token().kind, TokenKind::DslBlockEnd);
    }

    // ── Extern keyword tests ──

    #[test]
    fn extern_keyword() {
        assert_eq!(kinds("extern fn"), vec![TokenKind::Extern, TokenKind::Fn]);
    }

    #[test]
    fn extern_not_ident() {
        assert_eq!(kinds("extern"), vec![TokenKind::Extern]);
    }

    #[test]
    fn extern_prefix_is_ident() {
        assert_eq!(kinds("external"), vec![TokenKind::Ident("external".into())]);
    }
}
