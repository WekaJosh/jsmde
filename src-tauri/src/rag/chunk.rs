const TARGET_CHARS: usize = 1200;
const MIN_CHARS: usize = 200;
const OVERLAP_CHARS: usize = 150;

pub struct Chunk {
    pub index: usize,
    pub content: String,
}

pub fn chunk_markdown(text: &str) -> Vec<Chunk> {
    let paragraphs = split_paragraphs(text);
    let mut chunks = Vec::new();
    let mut buf = String::new();

    for para in paragraphs {
        if buf.len() + para.len() + 2 <= TARGET_CHARS || buf.is_empty() {
            if !buf.is_empty() {
                buf.push_str("\n\n");
            }
            buf.push_str(&para);
        } else {
            push_chunk(&mut chunks, std::mem::take(&mut buf));
            buf.push_str(&tail(chunks.last().map(|c| c.content.as_str()).unwrap_or(""), OVERLAP_CHARS));
            if !buf.is_empty() {
                buf.push_str("\n\n");
            }
            buf.push_str(&para);
        }
    }
    push_chunk(&mut chunks, buf);
    chunks
}

fn push_chunk(out: &mut Vec<Chunk>, content: String) {
    let trimmed = content.trim().to_string();
    if trimmed.len() < MIN_CHARS && !out.is_empty() {
        let last = out.last_mut().unwrap();
        last.content.push_str("\n\n");
        last.content.push_str(&trimmed);
        return;
    }
    if trimmed.is_empty() {
        return;
    }
    let index = out.len();
    out.push(Chunk {
        index,
        content: trimmed,
    });
}

fn tail(s: &str, n: usize) -> String {
    if s.len() <= n {
        return s.to_string();
    }
    let start = s.len().saturating_sub(n);
    let boundary = s[start..]
        .char_indices()
        .next()
        .map(|(i, _)| start + i)
        .unwrap_or(start);
    s[boundary..].to_string()
}

fn split_paragraphs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_fence = false;

    for line in text.lines() {
        let fence = line.trim_start().starts_with("```");
        if fence {
            in_fence = !in_fence;
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            continue;
        }
        if in_fence {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
            continue;
        }
        if line.trim().is_empty() {
            if !current.trim().is_empty() {
                out.push(std::mem::take(&mut current));
            } else {
                current.clear();
            }
        } else {
            if !current.is_empty() {
                current.push('\n');
            }
            current.push_str(line);
        }
    }
    if !current.trim().is_empty() {
        out.push(current);
    }
    out
}
