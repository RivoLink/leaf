const FRONTMATTER_VERTICAL_THRESHOLD: usize = 5;

pub(super) fn extract_frontmatter(src: &str) -> (&str, Option<Vec<(String, String)>>) {
    let Some(rest) = src
        .strip_prefix("---\n")
        .or_else(|| src.strip_prefix("---\r\n"))
    else {
        return (src, None);
    };

    let open_len = src.len() - rest.len();
    let mut offset = open_len;
    for line in rest.split_inclusive('\n') {
        // A lone CR is a line ending (CommonMark/YAML), so "---\r\r\n" still closes.
        let fence = line.trim_end_matches(['\r', '\n']);
        if fence == "---" || fence == "..." {
            let fm_block = &src[open_len..offset];
            let content = &src[offset + line.len()..];
            let pairs = parse_pairs(fm_block);
            if pairs.is_empty() {
                return (content, None);
            }
            return (content, Some(pairs));
        }
        offset += line.len();
    }

    (src, None)
}

pub(super) fn is_vertical(pairs: &[(String, String)]) -> bool {
    pairs.len() >= FRONTMATTER_VERTICAL_THRESHOLD
}

fn parse_pairs(block: &str) -> Vec<(String, String)> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    let lines: Vec<&str> = block.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        let Some(colon_pos) = trimmed.find(':') else {
            i += 1;
            continue;
        };

        let key = trimmed[..colon_pos].trim().to_string();
        let raw_value = trimmed[colon_pos + 1..].trim();

        if key.is_empty() {
            i += 1;
            continue;
        }

        if raw_value == ">-" || raw_value == ">" || raw_value == "|" || raw_value == "|-" {
            let mut parts: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && is_indented(lines[i]) {
                let part = lines[i].trim();
                if !part.is_empty() {
                    parts.push(part);
                }
                i += 1;
            }
            pairs.push((key, parts.join(" ")));
        } else if raw_value.is_empty() {
            let mut items: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() && is_indented(lines[i]) {
                let part = lines[i].trim();
                if let Some(item) = part.strip_prefix("- ") {
                    items.push(item.trim());
                } else if !part.is_empty() {
                    items.push(part);
                }
                i += 1;
            }
            if items.is_empty() {
                pairs.push((key, String::new()));
            } else {
                pairs.push((key, items.join(", ")));
            }
        } else {
            pairs.push((key, strip_quotes(raw_value).to_string()));
            i += 1;
        }
    }

    pairs
}

fn is_indented(line: &str) -> bool {
    line.starts_with(' ') || line.starts_with('\t')
}

fn strip_quotes(s: &str) -> &str {
    if s.len() >= 2
        && ((s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')))
    {
        return &s[1..s.len() - 1];
    }
    s
}
