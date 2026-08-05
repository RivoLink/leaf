use unicode_width::UnicodeWidthStr;

pub(crate) fn to_unicode(text: &str) -> String {
    let preprocessed = strip_command_spaces(text);
    let converted = unicodeit::replace(&preprocessed);
    postprocess(&converted)
}

fn strip_command_spaces(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == '\\' && i + 1 < len && chars[i + 1].is_ascii_alphabetic() {
            let cmd_start = i + 1;
            let mut cmd_end = cmd_start;
            while cmd_end < len && chars[cmd_end].is_ascii_alphabetic() {
                cmd_end += 1;
            }
            let cmd = &chars[cmd_start..cmd_end];
            let is_left = cmd == ['l', 'e', 'f', 't'];

            if is_left || cmd == ['r', 'i', 'g', 'h', 't'] {
                if !is_left && result.ends_with(' ') {
                    result.pop();
                }
                i = cmd_end;
                if i < len && chars[i] == '.' {
                    i += 1;
                } else if is_left && i < len {
                    result.push(chars[i]);
                    i += 1;
                    if i < len && chars[i - 1] == '\\' && !chars[i].is_ascii_alphabetic() {
                        result.push(chars[i]);
                        i += 1;
                    }
                    if i < len && chars[i] == ' ' {
                        i += 1;
                    }
                }
                continue;
            }

            result.push('\\');
            for c in &chars[cmd_start..cmd_end] {
                result.push(*c);
            }
            i = cmd_end;
            if i < len && chars[i] == ' ' {
                let next = chars.get(i + 1).copied().unwrap_or(' ');
                let is_binop = cmd == ['c', 'd', 'o', 't']
                    || cmd == ['t', 'i', 'm', 'e', 's']
                    || cmd == ['d', 'i', 'v']
                    || cmd == ['p', 'm']
                    || cmd == ['m', 'p']
                    || cmd == ['i', 'n']
                    || cmd == ['c', 'a', 'p']
                    || cmd == ['c', 'u', 'p'];
                if !is_binop && (next.is_ascii_alphabetic() || next == '\\' || next == '{') {
                    i += 1;
                }
            }
            continue;
        }
        result.push(chars[i]);
        i += 1;
    }

    result
}

fn postprocess(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let mut i = 0;

    while i < input.len() {
        if input[i..].starts_with("\\text{") {
            let brace_start = i + 6;
            if let Some((content, end)) = read_brace_group(input, brace_start) {
                result.push_str(content.trim());
                i = end;
                continue;
            }
        }

        if input[i..].starts_with("\\begin{cases}") {
            let after = i + 13;
            if let Some(rel) = input[after..].find("\\end{cases}") {
                let body = &input[after..after + rel];
                let last_line = result.rsplit('\n').next().unwrap_or(&result);
                let pad = UnicodeWidthStr::width(last_line);
                result.push_str(&render_cases(body, pad));
                i = after + rel + 11;
                continue;
            }
        }

        if input[i..].starts_with("\\frac{") {
            if let Some((output, end)) = parse_frac(input, i) {
                result.push_str(&output);
                i = end;
                continue;
            }
            result.push_str("\\frac{");
            i += 6;
            continue;
        }

        if input[i..].starts_with("\\binom{") {
            if let Some((output, end)) = parse_binom(input, i) {
                result.push_str(&output);
                i = end;
                continue;
            }
        }

        if input[i..].starts_with("√{") {
            let brace_start = i + '√'.len_utf8() + 1;
            if let Some((group, end)) = read_brace_group(input, brace_start) {
                result.push('√');
                result.push('(');
                result.push_str(&postprocess(group));
                result.push(')');
                i = end;
                continue;
            }
        }

        if input[i..].starts_with("^{") {
            if let Some((output, end)) = convert_script(input, i + 2, to_superscript) {
                result.push_str(&output);
                i = end;
                continue;
            }
            result.push_str("^{");
            i += 2;
            continue;
        }

        if input[i..].starts_with("_{") {
            if let Some((output, end)) = convert_script(input, i + 2, to_subscript) {
                result.push_str(&output);
                i = end;
                continue;
            }
            result.push_str("_{");
            i += 2;
            continue;
        }

        if input[i..].starts_with('^') && i + 1 < input.len() {
            let next = input[i + 1..].chars().next().unwrap();
            if next != '{' {
                i += 1;
                continue;
            }
        }

        let ch = input[i..].chars().next().unwrap();
        result.push(ch);
        i += ch.len_utf8();
    }

    result
}

fn parse_two_groups(
    input: &str,
    start: usize,
    prefix_len: usize,
) -> Option<(String, String, usize)> {
    let after = start + prefix_len;
    let (a, after_a) = read_brace_group(input, after)?;
    if after_a >= input.len() || input.as_bytes()[after_a] != b'{' {
        return None;
    }
    let (b, after_b) = read_brace_group(input, after_a + 1)?;
    Some((postprocess(a), postprocess(b), after_b))
}

fn parse_frac(input: &str, start: usize) -> Option<(String, usize)> {
    let (num, den, end) = parse_two_groups(input, start, 6)?;
    let mut out = String::new();
    wrap_if_multi(&mut out, &num);
    out.push('/');
    wrap_if_multi(&mut out, &den);
    Some((out, end))
}

fn parse_binom(input: &str, start: usize) -> Option<(String, usize)> {
    let (n, k, end) = parse_two_groups(input, start, 7)?;
    Some((format!("C({n},{k})"), end))
}

fn wrap_if_multi(out: &mut String, s: &str) {
    if s.chars().count() > 1 && s.contains(['+', '-', '−', '=', ' ', '<', '>', '/']) {
        out.push('(');
        out.push_str(s);
        out.push(')');
    } else {
        out.push_str(s);
    }
}

fn convert_script(
    input: &str,
    brace_start: usize,
    mapper: fn(char) -> char,
) -> Option<(String, usize)> {
    let (group, end) = read_brace_group(input, brace_start)?;
    let group = postprocess(group);
    let mapped: String = group.chars().map(mapper).collect();
    let all_converted = mapped
        .chars()
        .zip(group.chars())
        .all(|(m, g)| m != g || g.is_ascii_digit());
    if all_converted {
        Some((mapped, end))
    } else {
        Some((format!("({group})"), end))
    }
}

fn read_brace_group(input: &str, start: usize) -> Option<(&str, usize)> {
    let bytes = input.as_bytes();
    let mut depth: u32 = 1;
    let mut i = start;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'{' => depth += 1,
            b'}' => depth -= 1,
            _ => {}
        }
        if depth > 0 {
            i += 1;
        }
    }
    if depth == 0 {
        Some((&input[start..i], i + 1))
    } else {
        None
    }
}

fn render_cases(body: &str, prefix_width: usize) -> String {
    let rows: Vec<&str> = body
        .split("\\\\")
        .map(|r| r.trim())
        .filter(|r| !r.is_empty())
        .collect();

    if rows.is_empty() {
        return "{ }".to_string();
    }

    let parsed: Vec<(String, Option<String>)> = rows
        .iter()
        .map(|row| {
            let parts: Vec<&str> = row.splitn(2, '&').collect();
            let value = postprocess(parts[0].trim());
            let condition = parts.get(1).map(|p| postprocess(p.trim()));
            (value, condition)
        })
        .collect();

    let max_first_col = parsed
        .iter()
        .map(|(v, _)| UnicodeWidthStr::width(v.as_str()))
        .max()
        .unwrap_or(0);

    let padding = " ".repeat(prefix_width);
    let mut out = String::new();

    for (idx, (value, condition)) in parsed.iter().enumerate() {
        let brace = if parsed.len() == 1 {
            "{"
        } else if idx == 0 {
            "\u{23A7}"
        } else if idx == parsed.len() - 1 {
            "\u{23A9}"
        } else {
            "\u{23AA}"
        };

        if idx > 0 {
            out.push('\n');
            out.push_str(&padding);
        }
        out.push_str(brace);
        out.push(' ');
        out.push_str(value);

        if let Some(cond) = condition {
            let val_width = UnicodeWidthStr::width(value.as_str());
            let col_pad = max_first_col - val_width + 2;
            out.push_str(&" ".repeat(col_pad));
            out.push_str(cond);
        }
    }

    out
}

pub(super) fn to_superscript(ch: char) -> char {
    match ch {
        '0' => '⁰',
        '1' => '¹',
        '2' => '²',
        '3' => '³',
        '4' => '⁴',
        '5' => '⁵',
        '6' => '⁶',
        '7' => '⁷',
        '8' => '⁸',
        '9' => '⁹',
        '+' => '⁺',
        '-' | '−' => '⁻',
        '=' => '⁼',
        '(' => '⁽',
        ')' => '⁾',
        'n' => 'ⁿ',
        'i' => 'ⁱ',
        _ => ch,
    }
}

fn to_subscript(ch: char) -> char {
    match ch {
        '0' => '₀',
        '1' => '₁',
        '2' => '₂',
        '3' => '₃',
        '4' => '₄',
        '5' => '₅',
        '6' => '₆',
        '7' => '₇',
        '8' => '₈',
        '9' => '₉',
        '+' => '₊',
        '-' | '−' => '₋',
        '=' => '₌',
        '(' => '₍',
        ')' => '₎',
        'a' => 'ₐ',
        'e' => 'ₑ',
        'i' => 'ᵢ',
        'j' => 'ⱼ',
        'k' => 'ₖ',
        'n' => 'ₙ',
        'o' => 'ₒ',
        'p' => 'ₚ',
        'r' => 'ᵣ',
        's' => 'ₛ',
        't' => 'ₜ',
        'u' => 'ᵤ',
        'x' => 'ₓ',
        _ => ch,
    }
}
