//! Document chunking service — code-aware and paragraph-based text splitting.

use regex::Regex;

/// Estimate token count (~4 chars per token).
fn estimate_tokens(text: &str) -> usize {
    (text.len() / 4).max(1)
}

/// Split text into chunks with overlap. Returns Vec<(content, token_count)>.
pub fn chunk_text(text: &str, chunk_size: usize, overlap: usize) -> Vec<(String, usize)> {
    let text = text.trim();
    if text.is_empty() {
        return Vec::new();
    }

    let separators = ["\n\n\n", "\n\n", "\n", ". ", " "];
    let paragraphs = split_by_separators(text, &separators, chunk_size);

    let mut chunks: Vec<(String, usize)> = Vec::new();
    let mut buffer = String::new();

    for para in &paragraphs {
        let para = para.trim();
        if para.is_empty() {
            continue;
        }
        let candidate = if buffer.is_empty() {
            para.to_string()
        } else {
            format!("{}\n\n{}", buffer, para)
        };
        let tokens = estimate_tokens(&candidate);

        if tokens <= chunk_size {
            buffer = candidate;
        } else {
            if !buffer.is_empty() {
                let t = estimate_tokens(&buffer);
                chunks.push((buffer.clone(), t));
            }
            if estimate_tokens(para) > chunk_size {
                chunks.extend(force_split(para, chunk_size));
                buffer.clear();
            } else {
                buffer = para.to_string();
            }
        }
    }

    if !buffer.is_empty() {
        let t = estimate_tokens(&buffer);
        chunks.push((buffer, t));
    }

    // Apply overlap
    if overlap > 0 && chunks.len() > 1 {
        chunks = apply_overlap(chunks, overlap);
    }

    chunks
}

/// Code-aware chunking: splits on function/class definitions.
pub fn chunk_code(content: &str, chunk_size: usize) -> Vec<(String, usize)> {
    // Rust regex doesn't support lookahead — split on lines starting with definitions
    let re = Regex::new(r"\n\s*(?:def |class |fn |function |func |async def |pub fn |impl )").unwrap();

    // Manual split preserving the delimiter at the start of each block
    let mut blocks: Vec<String> = Vec::new();
    let mut last = 0;
    for m in re.find_iter(content) {
        if m.start() > last {
            blocks.push(content[last..m.start()].to_string());
        }
        last = m.start() + 1; // keep \n with the next block
    }
    if last < content.len() {
        blocks.push(content[last..].to_string());
    }
    if blocks.is_empty() {
        blocks.push(content.to_string());
    }

    let mut chunks: Vec<(String, usize)> = Vec::new();
    let mut buffer = String::new();

    for block in blocks {
        let candidate = if buffer.is_empty() {
            block.trim().to_string()
        } else {
            format!("{}\n\n{}", buffer, block.trim())
        };
        let tokens = estimate_tokens(&candidate);

        if tokens <= chunk_size {
            buffer = candidate;
        } else {
            if !buffer.is_empty() {
                let t = estimate_tokens(&buffer);
                chunks.push((buffer.clone(), t));
            }
            if estimate_tokens(&block) > chunk_size {
                chunks.extend(force_split(&block, chunk_size));
                buffer.clear();
            } else {
                buffer = block.trim().to_string();
            }
        }
    }

    if !buffer.is_empty() {
        let t = estimate_tokens(&buffer);
        chunks.push((buffer, t));
    }

    if chunks.is_empty() {
        chunk_text(content, chunk_size, 0)
    } else {
        chunks
    }
}

/// Entry point: detect file type and dispatch to appropriate chunker.
pub fn process_file(path: &str, content: &str, chunk_size: usize, overlap: usize) -> Vec<(String, usize)> {
    let code_extensions = [".py", ".rs", ".js", ".ts", ".go", ".java", ".c", ".cpp", ".sh"];
    let ext = path.rfind('.').map(|i| &path[i..]).unwrap_or("");
    let ext_lower = ext.to_lowercase();

    if code_extensions.iter().any(|e| ext_lower == *e) {
        let mut chunks = chunk_code(content, chunk_size);
        if overlap > 0 && chunks.len() > 1 {
            chunks = apply_overlap(chunks, overlap);
        }
        chunks
    } else {
        chunk_text(content, chunk_size, overlap)
    }
}

// ── Internal helpers ──

fn split_by_separators(text: &str, separators: &[&str], max_tokens: usize) -> Vec<String> {
    for sep in separators {
        let parts: Vec<&str> = text.split(sep).collect();
        if parts.iter().all(|p| estimate_tokens(p) <= max_tokens) {
            return parts.into_iter().map(String::from).collect();
        }
    }
    vec![text.to_string()]
}

fn force_split(text: &str, chunk_size: usize) -> Vec<(String, usize)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut chunks = Vec::new();
    let mut buf_words: Vec<&str> = Vec::new();
    let mut current_tokens = 0usize;

    for word in words {
        let wt = (word.len() / 4).max(1);
        if current_tokens + wt > chunk_size && !buf_words.is_empty() {
            let chunk = buf_words.join(" ");
            let t = estimate_tokens(&chunk);
            chunks.push((chunk, t));
            buf_words.clear();
            current_tokens = 0;
        }
        buf_words.push(word);
        current_tokens += wt;
    }

    if !buf_words.is_empty() {
        let chunk = buf_words.join(" ");
        let t = estimate_tokens(&chunk);
        chunks.push((chunk, t));
    }

    chunks
}

fn apply_overlap(chunks: Vec<(String, usize)>, overlap: usize) -> Vec<(String, usize)> {
    let mut result: Vec<(String, usize)> = vec![chunks[0].clone()];

    for i in 1..chunks.len() {
        let prev_text = &chunks[i - 1].0;
        let curr_text = &chunks[i].0;
        let prev_words: Vec<&str> = prev_text.split_whitespace().collect();
        let start = if prev_words.len() > overlap {
            prev_words.len() - overlap
        } else {
            0
        };
        let overlap_words = &prev_words[start..];
        let prefix = overlap_words.join(" ");
        let merged = format!("{}\n{}", prefix, curr_text).trim().to_string();
        let t = estimate_tokens(&merged);
        result.push((merged, t));
    }

    result
}
