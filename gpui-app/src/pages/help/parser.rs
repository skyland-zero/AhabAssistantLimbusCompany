#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum HelpBlock {
    Heading { level: u8, text: String },
    Paragraph(String),
    Bullet(String),
    Ordered(String),
    Code(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum InlinePart {
    Text(String),
    Strong(String),
    Code(String),
    Link { label: String, url: String },
}

pub(super) fn parse_help(source: &str) -> Vec<HelpBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();
    let mut code = false;
    let mut code_lines = Vec::new();

    let flush_paragraph = |blocks: &mut Vec<HelpBlock>, paragraph: &mut Vec<String>| {
        if !paragraph.is_empty() {
            blocks.push(HelpBlock::Paragraph(paragraph.join(" ")));
            paragraph.clear();
        }
    };
    let flush_code = |blocks: &mut Vec<HelpBlock>, code_lines: &mut Vec<String>| {
        if !code_lines.is_empty() {
            blocks.push(HelpBlock::Code(code_lines.join("\n")));
            code_lines.clear();
        }
    };

    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if code {
                flush_code(&mut blocks, &mut code_lines);
            } else {
                flush_paragraph(&mut blocks, &mut paragraph);
            }
            code = !code;
            continue;
        }
        if code {
            code_lines.push(line.to_owned());
            continue;
        }
        if trimmed.is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph);
        } else if let Some((level, title)) = heading(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Heading {
                level,
                text: inline_text(title),
            });
        } else if let Some(text) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Bullet(text.to_owned()));
        } else if let Some(text) = ordered_item(trimmed) {
            flush_paragraph(&mut blocks, &mut paragraph);
            blocks.push(HelpBlock::Ordered(text));
        } else {
            paragraph.push(trimmed.to_owned());
        }
    }
    if code {
        flush_code(&mut blocks, &mut code_lines);
    } else {
        flush_paragraph(&mut blocks, &mut paragraph);
    }
    blocks
}

pub(super) fn heading(line: &str) -> Option<(u8, &str)> {
    let hashes = line
        .chars()
        .take_while(|character| *character == '#')
        .count();
    if (1..=3).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ') {
        Some((hashes as u8, line[hashes + 1..].trim()))
    } else {
        None
    }
}

pub(super) fn ordered_item(line: &str) -> Option<String> {
    let split = line.find(". ")?;
    if split > 0
        && line[..split]
            .chars()
            .all(|character| character.is_ascii_digit())
    {
        Some(format!("{}.  {}", &line[..split], &line[split + 2..]))
    } else {
        None
    }
}

pub(super) fn parse_inline(text: &str) -> Vec<InlinePart> {
    let mut parts = Vec::new();
    let mut remaining = text;
    while !remaining.is_empty() {
        let next = [
            remaining.find("**"),
            remaining.find('`'),
            remaining.find('['),
        ]
        .into_iter()
        .flatten()
        .min();
        let Some(index) = next else {
            parts.push(InlinePart::Text(remaining.to_owned()));
            break;
        };
        if index > 0 {
            parts.push(InlinePart::Text(remaining[..index].to_owned()));
            remaining = &remaining[index..];
            continue;
        }

        if remaining.starts_with("**") {
            if let Some(end) = remaining[2..].find("**") {
                let value_end = 2 + end;
                parts.push(InlinePart::Strong(remaining[2..value_end].to_owned()));
                remaining = &remaining[value_end + 2..];
                continue;
            }
            parts.push(InlinePart::Text("**".to_owned()));
            remaining = &remaining[2..];
            continue;
        }

        if remaining.starts_with('`') {
            if let Some(end) = remaining[1..].find('`') {
                let value_end = 1 + end;
                parts.push(InlinePart::Code(remaining[1..value_end].to_owned()));
                remaining = &remaining[value_end + 1..];
                continue;
            }
            parts.push(InlinePart::Text("`".to_owned()));
            remaining = &remaining[1..];
            continue;
        }

        if remaining.starts_with('[') {
            if let Some(label_end) = remaining[1..].find("](") {
                let label_end = label_end + 1;
                let url_start = label_end + 2;
                if let Some(url_end) = remaining[url_start..].find(')') {
                    let url_end = url_start + url_end;
                    parts.push(InlinePart::Link {
                        label: remaining[1..label_end].to_owned(),
                        url: remaining[url_start..url_end].to_owned(),
                    });
                    remaining = &remaining[url_end + 1..];
                    continue;
                }
            }
            parts.push(InlinePart::Text("[".to_owned()));
            remaining = &remaining[1..];
            continue;
        }
    }
    parts
}

pub(super) fn inline_text(text: impl AsRef<str>) -> String {
    parse_inline(text.as_ref())
        .into_iter()
        .map(|part| match part {
            InlinePart::Text(value) | InlinePart::Strong(value) | InlinePart::Code(value) => value,
            InlinePart::Link { label, .. } => label,
        })
        .collect()
}
