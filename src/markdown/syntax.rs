use ratatui::{
    style::{Modifier, Style},
    text::Span,
};
use syntect::{
    easy::HighlightLines, highlighting::Theme, parsing::SyntaxSet, util::LinesWithEndings,
};
use unicode_width::UnicodeWidthStr;

use super::width::{display_width, expand_tabs};
use ratatui::style::Color;

pub(super) fn syntect_to_color(c: syntect::highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// Language key for a file: its extension, or for well-known extensionless
/// DevOps files (`Dockerfile`, `Makefile`, `Jenkinsfile`, ...) the lowercased name.
pub(crate) fn syntax_key(path: &std::path::Path) -> String {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if name.starts_with("dockerfile.") {
        return "dockerfile".into();
    }
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        return ext.to_string();
    }
    match name.as_str() {
        "dockerfile" | "containerfile" => "dockerfile".into(),
        "makefile" | "gnumakefile" => "makefile".into(),
        "jenkinsfile" => "jenkinsfile".into(),
        ".env" | ".envrc" => "env".into(),
        "tiltfile" | "vagrantfile" | "inventory" => name,
        _ => String::new(),
    }
}

pub(crate) fn resolve_syntax<'a>(
    lang: &str,
    ss: &'a SyntaxSet,
) -> &'a syntect::parsing::SyntaxReference {
    let raw = lang.trim();
    let normalized = raw
        .split(|c: char| c.is_whitespace() || c == ',' || c == '{')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();

    let aliases: &[&str] = match normalized.as_str() {
        "ts" | "typescript" => &[
            "JavaScript",
            "js",
            "javascript",
            "TypeScript",
            "ts",
            "typescript",
        ],
        "tsx" => &["JSX", "jsx", "JavaScript", "js", "typescriptreact", "tsx"],
        "js" | "javascript" => &["JavaScript", "js", "javascript"],
        "jsx" => &["JSX", "jsx", "JavaScript React", "JavaScript", "js"],
        "shell" | "bash" | "sh" | "zsh" => &["Bourne Again Shell (bash)", "bash", "sh"],
        "py" | "python" => &["Python", "py", "python"],
        "c" => &["C", "c"],
        "cpp" | "cxx" | "cc" | "c++" => &["C++", "cpp", "cxx", "cc"],
        "json" | "json5" => &["JSON", "json"],
        "toml" => &["TOML", "toml", "YAML", "yml", "yaml"],
        "java" => &["Java", "java"],
        "kt" | "kotlin" => &["Kotlin", "kt", "kotlin", "Java", "java"],
        "ps1" | "powershell" | "pwsh" => &["PowerShell", "ps1", "powershell", "bash", "sh"],
        "docker" | "dockerfile" => &["Dockerfile", "dockerfile", "bash", "sh"],
        "tf" | "tfvars" | "hcl" | "terraform" | "nomad" | "tfstate" => {
            &["Terraform", "HCL", "tf", "hcl"]
        }
        "make" | "makefile" | "mk" => &["Makefile", "make", "mk"],
        "env" | "dotenv" | "ini" | "conf" | "cfg" | "properties" => &["INI", "Properties", "bash"],
        "jenkinsfile" | "groovy" | "gradle" => &["Groovy", "groovy"],
        // No Go-template grammar ships with two-face; Jinja2 is the closest for Helm.
        "tmpl" | "gotmpl" | "tpl" | "j2" | "jinja" | "jinja2" => &["Jinja2", "Go Template", "yaml"],
        "service" | "timer" | "socket" | "mount" | "target" | "inventory" => &["INI", "ini"],
        "rego" | "opa" => &["Rego", "rego"],
        "proto" | "protobuf" => &["Protocol Buffer", "proto"],
        "tiltfile" | "starlark" | "bzl" => &["Python", "py"],
        "vagrantfile" => &["Ruby", "rb"],
        "nginx" => &["nginx", "conf"],
        "yml" | "yaml" => &["YAML", "yml", "yaml"],
        "rs" | "rust" => &["Rust", "rs", "rust"],
        _ if normalized.is_empty() => &[],
        _ => &[],
    };

    // "PHP Source" highlights php even without "<?php".
    if matches!(
        normalized.as_str(),
        "php" | "php3" | "php4" | "php5" | "php7" | "phtml"
    ) {
        if let Some(s) = ss.find_syntax_by_name("PHP Source") {
            return s;
        }
    }

    ss.find_syntax_by_token(raw)
        .or_else(|| ss.find_syntax_by_extension(raw))
        .or_else(|| ss.find_syntax_by_token(&normalized))
        .or_else(|| ss.find_syntax_by_extension(&normalized))
        .or_else(|| {
            aliases.iter().find_map(|alias| {
                ss.find_syntax_by_token(alias)
                    .or_else(|| ss.find_syntax_by_extension(alias))
                    .or_else(|| ss.find_syntax_by_name(alias))
            })
        })
        .unwrap_or_else(|| ss.find_syntax_plain_text())
}

pub(super) struct CodeLine {
    pub(super) content_spans: Vec<Span<'static>>,
}

pub(super) fn highlight_code(
    code: &str,
    lang: &str,
    ss: &SyntaxSet,
    theme: &Theme,
    render_width: usize,
    full_width: bool,
    show_line_numbers: bool,
) -> (Vec<CodeLine>, usize, usize) {
    let syntax = resolve_syntax(lang, ss);
    let mut hl = HighlightLines::new(syntax, theme);

    let mut raw: Vec<(Vec<Span<'static>>, usize)> = Vec::new();
    for line_str in LinesWithEndings::from(code) {
        let regions = hl.highlight_line(line_str, ss).unwrap_or_default();
        let mut spans = Vec::new();
        let mut text_width: usize = 0;
        for (st, text) in &regions {
            let t = expand_tabs(text.trim_end_matches('\n'), text_width);
            if t.is_empty() {
                continue;
            }
            text_width += display_width(&t);
            let mut rs = Style::default().fg(syntect_to_color(st.foreground));
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::BOLD)
            {
                rs = rs.add_modifier(Modifier::BOLD);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::ITALIC)
            {
                rs = rs.add_modifier(Modifier::ITALIC);
            }
            if st
                .font_style
                .contains(syntect::highlighting::FontStyle::UNDERLINE)
            {
                rs = rs.add_modifier(Modifier::UNDERLINED);
            }
            spans.push(Span::styled(t, rs));
        }
        raw.push((spans, text_width));
    }

    let label = if lang.is_empty() { "text" } else { lang };
    let total_lines = raw.len();
    let digit_width = if show_line_numbers {
        total_lines.max(1).to_string().len()
    } else {
        0
    };
    let gutter_width = if show_line_numbers {
        digit_width + 2
    } else {
        1
    };
    let max_inner_width = render_width
        .saturating_sub(2)
        .max(UnicodeWidthStr::width(label) + 3);
    let inner_width = if full_width {
        max_inner_width
    } else {
        let max_text = raw.iter().map(|(_, w)| *w).max().unwrap_or(0);
        let min_inner = (UnicodeWidthStr::width(label) + 3)
            .max(44)
            .min(max_inner_width);
        (max_text + 2 + gutter_width)
            .max(min_inner)
            .min(max_inner_width)
    };

    let mut out = Vec::new();
    for (spans, _text_width) in raw {
        out.push(CodeLine {
            content_spans: spans,
        });
    }
    (out, inner_width, digit_width)
}
