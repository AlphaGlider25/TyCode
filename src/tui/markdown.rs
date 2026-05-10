use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

// ── Theme colors (kept in sync with ui.rs palette) ───────────────────────────
const TEXT_PRIMARY:   Color = Color::Rgb(220, 215, 235);
const TEXT_SECONDARY: Color = Color::Rgb(140, 130, 170);
const TEXT_MUTED:     Color = Color::Rgb(80, 75, 105);
const ACCENT_DIM:     Color = Color::Rgb(60, 40, 120);
const CODE_FG:        Color = Color::Rgb(200, 190, 230);
const CODE_BG:        Color = Color::Rgb(18, 16, 28);
const SUCCESS:        Color = Color::Rgb(80, 200, 120);

// Syntax highlight token colors
const SYN_KEYWORD: Color  = Color::Rgb(180, 100, 240);
const SYN_STRING:  Color  = Color::Rgb(130, 200, 100);
const SYN_NUMBER:  Color  = Color::Rgb(240, 180, 80);
const SYN_COMMENT: Color  = Color::Rgb(100, 95, 130);
const SYN_TYPE:    Color  = Color::Rgb(100, 190, 240);
const SYN_SYMBOL:  Color  = Color::Rgb(200, 160, 255);

/// Convert markdown text to styled ratatui Lines.
/// `width` controls the inner width for code-block borders.
pub fn markdown_to_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let inner_w = (width as usize).saturating_sub(6).max(20);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut in_code_block = false;
    let mut code_block_lines: Vec<String> = Vec::new();
    let mut code_lang = String::new();

    for raw_line in text.lines() {
        if raw_line.starts_with("```") {
            if in_code_block {
                in_code_block = false;
                render_code_block(&mut lines, &code_block_lines, &code_lang, inner_w);
                code_block_lines.clear();
                code_lang.clear();
            } else {
                in_code_block = true;
                code_lang = raw_line.trim_start_matches('`').trim().to_string();
            }
            continue;
        }

        if in_code_block {
            code_block_lines.push(raw_line.to_string());
            continue;
        }

        // Blockquote
        if raw_line.starts_with("> ") {
            let content = &raw_line[2..];
            let mut spans = vec![
                Span::styled("  │ ", Style::default().fg(ACCENT_DIM)),
            ];
            let mut inner = parse_inline(content);
            for s in inner.iter_mut() {
                s.style = s.style.fg(TEXT_SECONDARY).add_modifier(Modifier::ITALIC);
            }
            spans.extend(inner);
            lines.push(Line::from(spans));
            continue;
        }

        // Headers — H3 first (most specific)
        if raw_line.starts_with("### ") {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", &raw_line[4..]),
                Style::default().fg(SYN_SYMBOL).add_modifier(Modifier::BOLD),
            )]));
            continue;
        }
        if raw_line.starts_with("## ") {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", &raw_line[3..]),
                Style::default().fg(SYN_TYPE).add_modifier(Modifier::BOLD),
            )]));
            continue;
        }
        if raw_line.starts_with("# ") {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    raw_line[2..].to_string(),
                    Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                ),
            ]));
            continue;
        }

        // Horizontal rule
        if raw_line.trim() == "---" || raw_line.trim() == "***" || raw_line.trim() == "___" {
            lines.push(Line::from(vec![Span::styled(
                format!("  {}", "─".repeat(inner_w)),
                Style::default().fg(TEXT_MUTED),
            )]));
            continue;
        }

        // Task list: - [ ] or - [x]
        if let Some(rest) = raw_line.strip_prefix("- [ ] ").or_else(|| raw_line.strip_prefix("- [ ]")) {
            let mut spans = vec![Span::styled("  ○ ", Style::default().fg(TEXT_MUTED))];
            spans.extend(parse_inline(rest));
            lines.push(Line::from(spans));
            continue;
        }
        if let Some(rest) = raw_line.strip_prefix("- [x] ").or_else(|| raw_line.strip_prefix("- [x]"))
            .or_else(|| raw_line.strip_prefix("- [X] ")) {
            let mut spans = vec![Span::styled("  ✓ ", Style::default().fg(SUCCESS))];
            let mut inner = parse_inline(rest);
            for s in inner.iter_mut() {
                s.style = s.style.add_modifier(Modifier::DIM);
            }
            spans.extend(inner);
            lines.push(Line::from(spans));
            continue;
        }

        // Bullet list
        if raw_line.starts_with("- ") || raw_line.starts_with("* ") {
            let content = &raw_line[2..];
            let mut spans = vec![Span::styled(
                "  • ".to_string(),
                Style::default().fg(TEXT_SECONDARY),
            )];
            spans.extend(parse_inline(content));
            lines.push(Line::from(spans));
            continue;
        }

        // Numbered list
        if let Some(rest) = try_strip_numbered_list(raw_line) {
            let prefix_len = raw_line.len() - rest.len();
            let prefix = &raw_line[..prefix_len];
            let mut spans = vec![Span::styled(
                format!("  {prefix}"),
                Style::default().fg(TEXT_SECONDARY),
            )];
            spans.extend(parse_inline(rest));
            lines.push(Line::from(spans));
            continue;
        }

        // Normal line
        if raw_line.is_empty() {
            lines.push(Line::from(""));
        } else {
            lines.push(Line::from(parse_inline(raw_line)));
        }
    }

    // Handle unterminated code block
    if in_code_block && !code_block_lines.is_empty() {
        render_code_block(&mut lines, &code_block_lines, &code_lang, inner_w);
    }

    lines
}

fn render_code_block(lines: &mut Vec<Line<'static>>, code_lines: &[String], lang: &str, inner_w: usize) {
    let label = if lang.is_empty() { " code ".to_string() } else { format!(" {lang} ") };
    let fill = inner_w.saturating_sub(label.len() + 2);
    let top = format!("  ╭─{label}{}", "─".repeat(fill));
    let bot = format!("  ╰{}", "─".repeat(inner_w));

    lines.push(Line::from(vec![
        Span::styled(top, Style::default().fg(TEXT_MUTED)),
    ]));
    for cl in code_lines {
        let mut spans = vec![
            Span::styled("  │ ", Style::default().fg(TEXT_MUTED)),
        ];
        spans.extend(highlight_code(cl, lang));
        lines.push(Line::from(spans));
    }
    lines.push(Line::from(vec![
        Span::styled(bot, Style::default().fg(TEXT_MUTED)),
    ]));
}

/// Simple keyword-based syntax highlighting. Returns a list of styled spans for one code line.
fn highlight_code(line: &str, lang: &str) -> Vec<Span<'static>> {
    let lang = lang.to_lowercase();
    match lang.as_str() {
        "rust" => highlight_rust(line),
        "python" | "py" => highlight_python(line),
        "javascript" | "js" | "typescript" | "ts" | "jsx" | "tsx" => highlight_js(line),
        "bash" | "sh" | "shell" | "zsh" => highlight_bash(line),
        "toml" => highlight_toml(line),
        "json" => highlight_json(line),
        "yaml" | "yml" => highlight_yaml(line),
        "go" => highlight_go(line),
        "html" | "xml" => highlight_html(line),
        "sql" => highlight_sql(line),
        _ => vec![Span::styled(line.to_string(), Style::default().fg(CODE_FG))],
    }
}

fn default_fg() -> Style { Style::default().fg(CODE_FG) }
fn kw() -> Style { Style::default().fg(SYN_KEYWORD) }
fn str_style() -> Style { Style::default().fg(SYN_STRING) }
fn num() -> Style { Style::default().fg(SYN_NUMBER) }
fn comment() -> Style { Style::default().fg(SYN_COMMENT).add_modifier(Modifier::ITALIC) }
fn ty() -> Style { Style::default().fg(SYN_TYPE) }
fn sym() -> Style { Style::default().fg(SYN_SYMBOL) }

fn tokenize_with_strings(line: &str, keywords: &[&str], types: &[&str], comment_prefix: &str) -> Vec<Span<'static>> {
    // Check for line comments first
    if !comment_prefix.is_empty() {
        if let Some(idx) = line.find(comment_prefix) {
            let before = &line[..idx];
            let after = &line[idx..];
            if !before.contains('"') && !before.contains('\'') {
                let mut spans = tokenize_words(before, keywords, types);
                spans.push(Span::styled(after.to_string(), comment()));
                return spans;
            }
        }
    }
    tokenize_words(line, keywords, types)
}

fn tokenize_words(line: &str, keywords: &[&str], types: &[&str]) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut in_string = false;
    let mut string_char = '"';
    let mut current = String::new();
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_string {
            current.push(ch);
            if ch == '\\' {
                if let Some(next) = chars.next() {
                    current.push(next);
                }
            } else if ch == string_char {
                spans.push(Span::styled(current.clone(), str_style()));
                current.clear();
                in_string = false;
            }
        } else if ch == '"' || ch == '\'' {
            // flush current word
            if !current.is_empty() {
                spans.push(classify_word(current.clone(), keywords, types));
                current.clear();
            }
            in_string = true;
            string_char = ch;
            current.push(ch);
        } else if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
        } else {
            if !current.is_empty() {
                spans.push(classify_word(current.clone(), keywords, types));
                current.clear();
            }
            // Check if it's a number starting with digit followed by non-alpha
            spans.push(Span::styled(ch.to_string(), default_fg()));
        }
    }
    if !current.is_empty() {
        if in_string {
            spans.push(Span::styled(current, str_style()));
        } else {
            spans.push(classify_word(current, keywords, types));
        }
    }
    if spans.is_empty() {
        spans.push(Span::raw(""));
    }
    spans
}

fn classify_word(word: String, keywords: &[&str], types: &[&str]) -> Span<'static> {
    if keywords.contains(&word.as_str()) {
        Span::styled(word, kw())
    } else if types.contains(&word.as_str()) {
        Span::styled(word, ty())
    } else if word.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        Span::styled(word, num())
    } else if word.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
        Span::styled(word, sym())
    } else {
        Span::styled(word, default_fg())
    }
}

fn highlight_rust(line: &str) -> Vec<Span<'static>> {
    const KW: &[&str] = &["fn", "let", "mut", "pub", "use", "mod", "struct", "enum", "impl",
        "trait", "type", "const", "static", "return", "if", "else", "for", "while", "loop",
        "match", "in", "where", "async", "await", "move", "ref", "self", "Self", "super",
        "crate", "true", "false", "Some", "None", "Ok", "Err", "break", "continue", "unsafe"];
    const TY: &[&str] = &["String", "str", "i8","i16","i32","i64","i128","u8","u16","u32","u64",
        "u128","usize","isize","f32","f64","bool","char","Vec","Option","Result","Box","Arc",
        "Rc","Mutex","HashMap","HashSet","BTreeMap"];
    tokenize_with_strings(line, KW, TY, "//")
}

fn highlight_python(line: &str) -> Vec<Span<'static>> {
    const KW: &[&str] = &["def", "class", "import", "from", "return", "if", "elif", "else",
        "for", "while", "in", "not", "and", "or", "is", "lambda", "yield", "with", "as",
        "try", "except", "finally", "raise", "pass", "break", "continue", "async", "await",
        "True", "False", "None", "global", "nonlocal", "del", "assert"];
    const TY: &[&str] = &["int", "str", "float", "bool", "list", "dict", "set", "tuple",
        "bytes", "bytearray", "type", "object", "super", "self", "cls", "print", "len",
        "range", "enumerate", "zip", "map", "filter", "sorted", "reversed", "open"];
    tokenize_with_strings(line, KW, TY, "#")
}

fn highlight_js(line: &str) -> Vec<Span<'static>> {
    const KW: &[&str] = &["const", "let", "var", "function", "return", "if", "else", "for",
        "while", "do", "in", "of", "class", "extends", "new", "this", "super", "import",
        "export", "default", "from", "async", "await", "try", "catch", "finally", "throw",
        "typeof", "instanceof", "void", "delete", "true", "false", "null", "undefined",
        "break", "continue", "switch", "case", "yield", "static", "get", "set", "type",
        "interface", "enum", "implements", "abstract", "readonly"];
    const TY: &[&str] = &["string", "number", "boolean", "object", "Array", "Promise",
        "Map", "Set", "Symbol", "Error", "Date", "RegExp", "JSON", "Math", "Object",
        "console", "window", "document", "module", "require", "exports"];
    tokenize_with_strings(line, KW, TY, "//")
}

fn highlight_bash(line: &str) -> Vec<Span<'static>> {
    // Check for comment
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return vec![Span::styled(line.to_string(), comment())];
    }
    const KW: &[&str] = &["if", "then", "else", "elif", "fi", "for", "do", "done", "while",
        "until", "case", "esac", "in", "function", "return", "local", "export", "readonly",
        "echo", "printf", "exit", "source", "cd", "pwd", "ls", "mkdir", "rm", "cp", "mv",
        "cat", "grep", "sed", "awk", "find", "curl", "wget", "sudo", "chmod", "chown",
        "true", "false", "test", "read", "shift", "set", "unset"];
    tokenize_with_strings(line, KW, &[], "#")
}

fn highlight_toml(line: &str) -> Vec<Span<'static>> {
    let trimmed = line.trim();
    if trimmed.starts_with('#') {
        return vec![Span::styled(line.to_string(), comment())];
    }
    if trimmed.starts_with('[') {
        return vec![Span::styled(line.to_string(), Style::default().fg(SYN_TYPE).add_modifier(Modifier::BOLD))];
    }
    // key = value: highlight key in keyword color
    if let Some(eq_pos) = line.find('=') {
        let key = &line[..eq_pos];
        let rest = &line[eq_pos..];
        let mut spans = vec![Span::styled(key.to_string(), kw())];
        spans.push(Span::styled(rest.to_string(), str_style()));
        return spans;
    }
    vec![Span::styled(line.to_string(), default_fg())]
}

fn highlight_json(line: &str) -> Vec<Span<'static>> {
    tokenize_with_strings(line, &["true", "false", "null"], &[], "")
}

fn highlight_yaml(line: &str) -> Vec<Span<'static>> {
    let trimmed = line.trim_start();
    if trimmed.starts_with('#') {
        return vec![Span::styled(line.to_string(), comment())];
    }
    if let Some(colon_pos) = line.find(": ") {
        let key = &line[..colon_pos];
        let rest = &line[colon_pos..];
        let key_style = if key.trim_start().starts_with('-') { default_fg() } else { kw() };
        return vec![
            Span::styled(key.to_string(), key_style),
            Span::styled(rest.to_string(), default_fg()),
        ];
    }
    vec![Span::styled(line.to_string(), default_fg())]
}

fn highlight_go(line: &str) -> Vec<Span<'static>> {
    const KW: &[&str] = &["func", "var", "const", "type", "struct", "interface", "map", "chan",
        "if", "else", "for", "range", "return", "import", "package", "go", "defer", "select",
        "switch", "case", "default", "break", "continue", "fallthrough", "goto", "nil", "true", "false"];
    const TY: &[&str] = &["string", "int", "int8", "int16", "int32", "int64", "uint", "uint8",
        "uint16", "uint32", "uint64", "float32", "float64", "bool", "byte", "rune", "error",
        "any", "comparable", "make", "new", "len", "cap", "append", "copy", "delete", "close",
        "panic", "recover", "print", "println"];
    tokenize_with_strings(line, KW, TY, "//")
}

fn highlight_html(line: &str) -> Vec<Span<'static>> {
    // Very basic: color tags in type color, attributes in keyword, values in string
    if let Some(start) = line.find('<') {
        let before = &line[..start];
        let after = &line[start..];
        let tag_end = after.find('>').map(|i| i + 1).unwrap_or(after.len());
        let tag = &after[..tag_end];
        let rest = &after[tag_end..];
        let mut spans = vec![];
        if !before.is_empty() { spans.push(Span::styled(before.to_string(), default_fg())); }
        spans.push(Span::styled(tag.to_string(), ty()));
        if !rest.is_empty() { spans.push(Span::styled(rest.to_string(), default_fg())); }
        return spans;
    }
    vec![Span::styled(line.to_string(), default_fg())]
}

fn highlight_sql(line: &str) -> Vec<Span<'static>> {
    const KW: &[&str] = &["SELECT", "FROM", "WHERE", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER",
        "ON", "AND", "OR", "NOT", "IN", "IS", "NULL", "ORDER", "BY", "GROUP", "HAVING",
        "INSERT", "INTO", "VALUES", "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "DROP",
        "ALTER", "ADD", "COLUMN", "INDEX", "PRIMARY", "KEY", "FOREIGN", "REFERENCES",
        "AS", "DISTINCT", "LIMIT", "OFFSET", "COUNT", "SUM", "AVG", "MIN", "MAX",
        "WITH", "UNION", "ALL", "EXISTS", "CASE", "WHEN", "THEN", "ELSE", "END",
        "select", "from", "where", "join", "left", "right", "inner", "outer",
        "on", "and", "or", "not", "in", "is", "null", "order", "by", "group",
        "insert", "into", "values", "update", "set", "delete", "create", "table"];
    tokenize_with_strings(line, KW, &[], "--")
}

/// Parse inline markdown: **bold**, *italic*, `code`, ~~strikethrough~~.
fn parse_inline(text: &str) -> Vec<Span<'static>> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut chars = text.char_indices().peekable();
    let mut current = String::new();

    let default_style = Style::default().fg(TEXT_PRIMARY);
    let bold_style    = Style::default().fg(TEXT_PRIMARY).add_modifier(Modifier::BOLD);
    let italic_style  = Style::default().fg(TEXT_SECONDARY).add_modifier(Modifier::ITALIC);
    let code_style    = Style::default().fg(SYN_SYMBOL).bg(CODE_BG);
    let strike_style  = Style::default().fg(TEXT_MUTED).add_modifier(Modifier::CROSSED_OUT);

    while let Some((_i, ch)) = chars.next() {
        match ch {
            '`' => {
                if !current.is_empty() { spans.push(Span::styled(std::mem::take(&mut current), default_style)); }
                let mut code = String::new();
                let mut found = false;
                for (_, c) in chars.by_ref() {
                    if c == '`' { found = true; break; }
                    code.push(c);
                }
                if found { spans.push(Span::styled(code, code_style)); }
                else { current.push('`'); current.push_str(&code); }
            }
            '~' if chars.peek().map(|(_, c)| *c) == Some('~') => {
                chars.next();
                if !current.is_empty() { spans.push(Span::styled(std::mem::take(&mut current), default_style)); }
                let mut strike = String::new();
                let mut found = false;
                while let Some((_, c)) = chars.next() {
                    if c == '~' && chars.peek().map(|(_, c)| *c) == Some('~') {
                        chars.next(); found = true; break;
                    }
                    strike.push(c);
                }
                if found { spans.push(Span::styled(strike, strike_style)); }
                else { current.push_str("~~"); current.push_str(&strike); }
            }
            '*' => {
                if chars.peek().map(|(_, c)| *c) == Some('*') {
                    chars.next();
                    if !current.is_empty() { spans.push(Span::styled(std::mem::take(&mut current), default_style)); }
                    let mut bold = String::new();
                    let mut found = false;
                    while let Some((_, c)) = chars.next() {
                        if c == '*' && chars.peek().map(|(_, c)| *c) == Some('*') {
                            chars.next(); found = true; break;
                        }
                        bold.push(c);
                    }
                    if found { spans.push(Span::styled(bold, bold_style)); }
                    else { current.push_str("**"); current.push_str(&bold); }
                } else {
                    if !current.is_empty() { spans.push(Span::styled(std::mem::take(&mut current), default_style)); }
                    let mut italic = String::new();
                    let mut found = false;
                    for (_, c) in chars.by_ref() {
                        if c == '*' { found = true; break; }
                        italic.push(c);
                    }
                    if found { spans.push(Span::styled(italic, italic_style)); }
                    else { current.push('*'); current.push_str(&italic); }
                }
            }
            _ => { current.push(ch); }
        }
    }
    if !current.is_empty() { spans.push(Span::styled(current, default_style)); }
    if spans.is_empty() { spans.push(Span::raw("")); }
    spans
}

fn try_strip_numbered_list(line: &str) -> Option<&str> {
    let digit_end = line.chars().take_while(|c| c.is_ascii_digit()).count();
    if digit_end == 0 { return None; }
    line[digit_end..].strip_prefix(". ")
}
