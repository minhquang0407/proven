use crate::graph_rag::CodeGraphRAG;
use crate::memory_cls::GraphMemoryManager;
use crate::models::{FunctionSpan, GraphEdge, GraphNode, RepoGraph, TargetContext};
use crate::oracle::{
    audit_changed_files_env_keys, collect_unverified_literals, detect_concurrency_risk,
    detect_integration_slice, detect_missing_sibling_guards_in_file, discover_target_specs,
    infer_signature_invariants,
};
use rayon::prelude::*;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

pub const MAX_SOURCE_FILE_BYTES: u64 = 1_048_576; // 1 MB DoS guard

pub fn read_bounded_source_file(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_SOURCE_FILE_BYTES {
        return None;
    }
    fs::read_to_string(path).ok()
}

/// Masks out string literals (`"""..."""`, `'''...'''`, `"..."`, `'...'`) and Python `#` comments
/// with spaces of identical byte length, while preserving Python's `//` floor-division operator.
pub fn mask_python_line(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_quote: Option<&[u8]> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(delim) = in_quote {
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(b' ');
                out.push(b' ');
                i += 2;
                continue;
            }
            if i + delim.len() <= bytes.len() && &bytes[i..i + delim.len()] == delim {
                out.extend(std::iter::repeat_n(b' ', delim.len()));
                in_quote = None;
                i += delim.len();
                continue;
            }
            out.push(b' ');
            i += 1;
        } else {
            if b == b'#' {
                while i < bytes.len() {
                    out.push(b' ');
                    i += 1;
                }
                break;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"\"\"\"" {
                out.extend_from_slice(b"   ");
                in_quote = Some(b"\"\"\"");
                i += 3;
                continue;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"'''" {
                out.extend_from_slice(b"   ");
                in_quote = Some(b"'''");
                i += 3;
                continue;
            }
            if b == b'"' {
                in_quote = Some(b"\"");
                out.push(b' ');
                i += 1;
                continue;
            }
            if b == b'\'' {
                in_quote = Some(b"'");
                out.push(b' ');
                i += 1;
                continue;
            }
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

/// Detects whether an apostrophe `'` at `bytes[i]` in a brace-delimited language line is a
/// Rust lifetime/loop-label (`'static`, `'a`, `'_`, `'outer:`) or C++14 numeric digit separator
/// (`1'000`, `0xFF'FF`) rather than a character/string quote delimiter. Returns `Some(token_len)`
/// of bytes starting at `i` that should be emitted outside quote mode, or `None` for real quotes.
pub fn non_quote_apostrophe_len(bytes: &[u8], i: usize) -> Option<usize> {
    if bytes.get(i) != Some(&b'\'') {
        return None;
    }
    // C++14 numeric digit separator: digit immediately before `'`, hex digit immediately after
    if i > 0
        && bytes[i - 1].is_ascii_digit()
        && !(i >= 2 && matches!(bytes[i - 2], b'u' | b'U') && bytes[i - 1] == b'8')
        && bytes.get(i + 1).is_some_and(u8::is_ascii_hexdigit)
    {
        return Some(1);
    }
    // Do not treat prefixed char/raw literals (`b'x'`, `r'...'`, `L'x'`, `u'x'`, `U'x'`) as lifetimes
    if i > 0 && (bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_') {
        return None;
    }
    let next = *bytes.get(i + 1)?;
    if !(next.is_ascii_alphabetic() || next == b'_') {
        return None;
    }
    let mut end = i + 2;
    while end < bytes.len() && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
        end += 1;
    }
    // Closed `'x'` char literal or `'ident'` single-word string literal
    if bytes.get(end) == Some(&b'\'') {
        return None;
    }
    let prev = if i > 0 { Some(bytes[i - 1]) } else { None };
    if prev == Some(b'&') {
        return Some(end - i);
    }
    if prev == Some(b'<') && matches!(bytes.get(end), Some(b'>' | b',' | b':')) {
        return Some(end - i);
    }
    if bytes.get(end) == Some(&b':')
        && prev.is_none_or(|p| p.is_ascii_whitespace() || matches!(p, b'{' | b';' | b','))
    {
        return Some(end - i);
    }
    let rest = &bytes[end..];
    let mut k = 0;
    let mut next_unescaped_quote = None;
    while k < rest.len() {
        if rest[k] == b'\\' {
            k += 2;
            continue;
        }
        if rest[k] == b'\'' {
            next_unescaped_quote = Some(k);
            break;
        }
        k += 1;
    }
    match next_unescaped_quote {
        None if bytes.last() != Some(&b'\\') => Some(end - i),
        Some(qk)
            if rest
                .get(qk + 1)
                .is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') =>
        {
            Some(end - i)
        }
        _ => None,
    }
}

/// Masks out string/template literals (`"..."`, `'...'`, `` `...` ``) and C-family comments
/// (`//...`, single-line and multi-line `/*...*/`) with spaces of identical byte length,
/// while preserving `#include`, `#[attribute]`, JS `#private` identifiers, and Rust lifetimes.
pub fn mask_brace_line(line: &str, in_block_comment: &mut bool) -> String {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_quote: Option<u8> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if *in_block_comment {
            if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                out.push(b' ');
                out.push(b' ');
                *in_block_comment = false;
                i += 2;
                continue;
            }
            out.push(b' ');
            i += 1;
            continue;
        }

        if let Some(q) = in_quote {
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(b' ');
                out.push(b' ');
                i += 2;
                continue;
            }
            if b == q {
                out.push(b' ');
                in_quote = None;
                i += 1;
                continue;
            }
            out.push(b' ');
            i += 1;
        } else {
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                while i < bytes.len() {
                    out.push(b' ');
                    i += 1;
                }
                break;
            }
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                out.push(b' ');
                out.push(b' ');
                *in_block_comment = true;
                i += 2;
                continue;
            }
            if b == b'\''
                && let Some(tok_len) = non_quote_apostrophe_len(bytes, i)
            {
                out.extend_from_slice(&bytes[i..i + tok_len]);
                i += tok_len;
                continue;
            }
            if b == b'"' || b == b'\'' || b == b'`' {
                in_quote = Some(b);
                out.push(b' ');
                i += 1;
                continue;
            }
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

/// Heuristic detector distinguishing brace-delimited languages (Rust, TS/JS, Go, Java, C/C++)
/// from Python source snippets. Scans for definitive language keywords first so Python files
/// with module-level dicts (`CONFIG = {`) or semicolons before `def` are never misclassified.
pub fn is_brace_language_source(source_code: &str) -> bool {
    let mut saw_brace_or_semi = false;
    for raw_line in source_code.lines() {
        let t = raw_line.trim();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        if t.starts_with("def ")
            || t.starts_with("async def ")
            || t.starts_with("from ")
            || t.starts_with("elif ")
            || t.starts_with("except ")
            || t.starts_with("except:")
            || (t.starts_with("class ") && t.ends_with(':'))
        {
            return false;
        }
        if t.starts_with("fn ")
            || t.starts_with("pub fn ")
            || t.starts_with("pub async fn ")
            || t.starts_with("pub struct ")
            || t.starts_with("pub(")
            || t.starts_with("func ")
            || t.starts_with("function ")
            || t.starts_with("export function ")
            || t.starts_with("export async function ")
            || t.starts_with("export const ")
            || t.starts_with("public ")
            || t.starts_with("private ")
            || t.starts_with("protected ")
            || t.starts_with("#include ")
            || t.starts_with("package ")
            || t.starts_with("use ")
        {
            return true;
        }
        if t.ends_with('{') || t.ends_with(';') {
            saw_brace_or_semi = true;
        }
    }
    saw_brace_or_semi || source_code.contains("/*")
}

pub fn mask_line_with_block_comment(line: &str, in_block_comment: &mut bool) -> String {
    if *in_block_comment || line.contains("/*") {
        mask_brace_line(line, in_block_comment)
    } else {
        mask_python_line(line)
    }
}

pub fn mask_strings_and_comments(line: &str) -> String {
    mask_python_line(line)
}

/// Strips optional Python string literal prefixes (`r`, `u`, `b`, `f`, `fr`, `rf`, `br`, `rb`)
/// when immediately followed by a quote character.
pub fn strip_python_string_prefix(s: &str) -> &str {
    let bytes = s.as_bytes();
    let mut prefix_len = 0;
    while prefix_len < bytes.len() && prefix_len < 2 {
        let b = bytes[prefix_len].to_ascii_lowercase();
        if matches!(b, b'r' | b'u' | b'b' | b'f') {
            prefix_len += 1;
        } else {
            break;
        }
    }
    if prefix_len > 0
        && (s[prefix_len..].starts_with("\"\"\"")
            || s[prefix_len..].starts_with("'''")
            || s[prefix_len..].starts_with('"')
            || s[prefix_len..].starts_with('\''))
    {
        &s[prefix_len..]
    } else {
        s
    }
}

/// Scans a masked line of a signature header, tracking parenthesis/bracket depth (`()`, `[]`),
/// and returns the byte index of the header-terminating `:` at depth 0 after the opening `(`.
pub fn scan_python_signature_colon(
    masked_line: &str,
    paren_depth: &mut i32,
    seen_open_paren: &mut bool,
) -> Option<usize> {
    for (byte_idx, ch) in masked_line.char_indices() {
        match ch {
            '(' | '[' => {
                *paren_depth += 1;
                if ch == '(' {
                    *seen_open_paren = true;
                }
            }
            ')' | ']' => {
                *paren_depth -= 1;
            }
            ':' if *seen_open_paren && *paren_depth <= 0 => {
                return Some(byte_idx);
            }
            _ => {}
        }
    }
    None
}

/// Tracks Python multi-line triple-quoted string state (`"""` or `'''`) across lines.
/// Uses byte slices (`&bytes[i..i + 3]`) rather than `&str` slices so multi-byte UTF-8 characters
/// inside or outside docstrings never cause a char-boundary panic.
pub fn update_triple_quote_state<'a>(line: &str, mut state: Option<&'a str>) -> Option<&'a str> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if let Some(delim) = state {
            if bytes[i] == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == delim.as_bytes() {
                state = None;
                i += 3;
                continue;
            }
            i += 1;
        } else {
            if bytes[i] == b'#' {
                break;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"\"\"\"" {
                state = Some("\"\"\"");
                i += 3;
                continue;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"'''" {
                state = Some("'''");
                i += 3;
                continue;
            }
            if bytes[i] == b'"' || bytes[i] == b'\'' {
                let q = bytes[i];
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == q {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            i += 1;
        }
    }
    state
}

pub fn is_declaration_file(rel_path: &str) -> bool {
    let lower = rel_path.to_lowercase();
    lower.ends_with(".d.ts") || lower.ends_with(".d.mts") || lower.ends_with(".d.cts")
}

/// Finds a matching function span by preferring exact ID/name matches first, and only falling
/// back to dot-qualified method matches (`Class.method`) or bare symbol matches (`method`),
/// avoiding false suffix collisions and preferring real implementation bodies over `@overload` stubs.
pub fn find_matching_function<'a>(
    functions: &'a [FunctionSpan],
    target_id: &str,
) -> Option<&'a FunctionSpan> {
    let raw_trim = target_id.trim();
    if raw_trim.is_empty() {
        return None;
    }
    let after_colons = raw_trim.rsplit("::").next().unwrap_or(raw_trim);
    let clean_sym = after_colons
        .strip_prefix("FUNC:")
        .or_else(|| after_colons.strip_prefix("CLASS:"))
        .or_else(|| after_colons.strip_prefix("FILE:"))
        .unwrap_or(after_colons)
        .trim();
    let short_sym = clean_sym.split('.').next_back().unwrap_or(clean_sym).trim();
    if clean_sym.is_empty() || short_sym.is_empty() {
        return None;
    }

    let prefer_impl = |a: &&'a FunctionSpan, b: &&'a FunctionSpan| {
        let len_a = a.end.saturating_sub(a.start);
        let len_b = b.end.saturating_sub(b.start);
        len_a.cmp(&len_b).then_with(|| b.start.cmp(&a.start))
    };

    // Pass 1: Exact full symbol match (e.g. `Range.__call__` must never match `URL.__call__`)
    let full_id = format!("FUNC:{}", clean_sym);
    if let Some(exact) = functions
        .iter()
        .filter(|f| f.function_id == raw_trim || f.function_id == full_id || f.name == clean_sym)
        .max_by(prefer_impl)
    {
        return Some(exact);
    }

    // Pass 2: Dot-qualified suffix match (`.short_sym`) or bare `short_sym` (for brace languages)
    let dot_suffix = format!(".{}", short_sym);
    if let Some(dot_match) = functions
        .iter()
        .filter(|f| f.name.ends_with(&dot_suffix))
        .max_by(prefer_impl)
    {
        return Some(dot_match);
    }

    functions
        .iter()
        .filter(|f| f.name == short_sym)
        .max_by(prefer_impl)
}

/// Security boundary: ensures `candidate` is a strictly relative path inside `repo_path`
/// with zero `..` traversal, zero drive/root prefixes, and no symlink escape outside `repo_path`.
pub fn sanitize_repo_relative_path(repo_path: &Path, candidate: &str) -> Option<String> {
    let trimmed = candidate.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return None;
    }
    let norm = trimmed.replace('\\', "/");
    if norm.starts_with('/') || norm.contains(':') {
        return None;
    }
    let rel_path = Path::new(&norm);
    for comp in rel_path.components() {
        match comp {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if let Ok(canon_repo) = repo_path.canonicalize() {
        let mut current = repo_path.to_path_buf();
        for comp in rel_path.components() {
            if let Component::Normal(part) = comp {
                current.push(part);
                if let Ok(sym_meta) = fs::symlink_metadata(&current)
                    && sym_meta.file_type().is_symlink() {
                        let Ok(canon_cur) = current.canonicalize() else {
                            return None;
                        };
                        if !canon_cur.starts_with(&canon_repo) {
                            return None;
                        }
                    }
            }
        }
        let joined = repo_path.join(rel_path);
        let mut probe = joined.as_path();
        while !probe.exists() {
            let Some(parent) = probe.parent() else {
                break;
            };
            if parent == probe {
                break;
            }
            probe = parent;
        }
        if probe.exists() {
            let Ok(canon_probe) = probe.canonicalize() else {
                return None;
            };
            if !canon_probe.starts_with(&canon_repo) {
                return None;
            }
        }
    }
    let cleaned = norm.trim_start_matches("./").to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Security guard against Git argument/flag injection (e.g. `--output=...` or `--ext-diff`).
pub fn is_safe_git_ref(git_ref: &str) -> bool {
    let t = git_ref.trim();
    if t.is_empty() || t.starts_with('-') || t.contains("..") || t.len() > 128 {
        return false;
    }
    t.chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '~' | '^' | '-'))
}

const NOISY_DIRS: &[&str] = &[
    ".git",
    ".hg",
    ".svn",
    ".venv",
    "venv",
    "env",
    ".env",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".tox",
    ".nox",
    "node_modules",
    "bower_components",
    "target",
    "build",
    "dist",
    "out",
    ".next",
    ".nuxt",
    ".svelte-kit",
    "vendor",
    "third_party",
    "site-packages",
    ".proven",
    ".softgnn",
    "data_output",
    "temp_dirty",
    ".idea",
    ".vscode",
    "coverage",
    "htmlcov",
];

pub fn is_noisy_dir(dir_name: &str, rel_path: &str) -> bool {
    let lower = dir_name.to_lowercase();
    if NOISY_DIRS.contains(&lower.as_str())
        || lower.ends_with(".egg-info")
        || lower.starts_with("venv_")
        || lower.starts_with(".venv")
        || lower.starts_with("temp_dirty")
    {
        return true;
    }
    let norm_rel = rel_path.replace('\\', "/").to_lowercase();
    for part in norm_rel.split('/') {
        if NOISY_DIRS.contains(&part)
            || part.ends_with(".egg-info")
            || part.starts_with("venv_")
            || part.starts_with(".venv")
            || part.starts_with("temp_dirty")
        {
            return true;
        }
    }
    false
}

pub fn detect_language_for_ext(ext: &str) -> Option<&'static str> {
    match ext.to_lowercase().as_str() {
        "py" => Some("python"),
        "rs" => Some("rust"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" => Some("javascript"),
        "go" => Some("go"),
        "java" => Some("java"),
        "c" | "cpp" | "cc" | "h" | "hpp" => Some("cpp"),
        "cs" => Some("csharp"),
        _ => None,
    }
}

pub fn detect_repo_language(repo_path: &Path) -> String {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    let walker = WalkDir::new(repo_path).into_iter().filter_entry(|e| {
        if e.file_type().is_dir() {
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            let rel = e
                .path()
                .strip_prefix(repo_path)
                .unwrap_or(e.path())
                .to_string_lossy();
            return !is_noisy_dir(&name, &rel);
        }
        true
    });

    for entry in walker.flatten() {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
                && let Some(lang) = detect_language_for_ext(ext) {
                    *counts.entry(lang).or_insert(0) += 1;
                }
    }
    counts
        .into_iter()
        .max_by_key(|&(_, count)| count)
        .map(|(lang, _)| lang.to_string())
        .unwrap_or_else(|| "python".to_string())
}

#[derive(Debug, Clone)]
pub struct ParsedFileSummary {
    pub rel_path: String,
    pub language: String,
    pub imports: Vec<String>,
    pub classes: Vec<String>,
    pub functions: Vec<FunctionSpan>,
}

pub fn parse_source_content(content: &str, rel_path: &str, lang: &str) -> ParsedFileSummary {
    if lang == "python" {
        parse_python_source(content, rel_path)
    } else {
        parse_brace_source(content, rel_path, lang)
    }
}

fn strip_python_inline_comment(line: &str) -> &str {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut in_quote: Option<&[u8]> = None;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(delim) = in_quote {
            if b == b'\\' && i + 1 < bytes.len() {
                i += 2;
                continue;
            }
            if i + delim.len() <= bytes.len() && &bytes[i..i + delim.len()] == delim {
                in_quote = None;
                i += delim.len();
                continue;
            }
            i += 1;
        } else {
            if b == b'#' {
                return &line[..i];
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"\"\"\"" {
                in_quote = Some(b"\"\"\"");
                i += 3;
                continue;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"'''" {
                in_quote = Some(b"'''");
                i += 3;
                continue;
            }
            if b == b'"' {
                in_quote = Some(b"\"");
                i += 1;
                continue;
            }
            if b == b'\'' {
                in_quote = Some(b"'");
                i += 1;
                continue;
            }
            i += 1;
        }
    }
    line
}

fn extract_calls_from_masked(
    masked_body: &str,
    call_re: &Regex,
    calls: &mut Vec<String>,
    uses: &mut Vec<String>,
) {
    for cap in call_re.captures_iter(masked_body) {
        let Some(sym_match) = cap.get(1) else {
            continue;
        };
        let sym = sym_match.as_str();
        if !matches!(
            sym,
            "if" | "for"
                | "while"
                | "return"
                | "print"
                | "len"
                | "range"
                | "isinstance"
                | "super"
        ) {
            let clean_sym = sym.strip_prefix("self.").unwrap_or(sym).to_string();
            if clean_sym
                .chars()
                .next()
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
                && !uses.contains(&clean_sym)
            {
                uses.push(clean_sym.clone());
            }
            if !calls.contains(&clean_sym) {
                calls.push(clean_sym);
            }
        }
    }
}

fn update_delim_depth(masked_line: &str, depth: &mut i32) {
    for ch in masked_line.chars() {
        if matches!(ch, '(' | '[' | '{') {
            *depth += 1;
        } else if matches!(ch, ')' | ']' | '}') {
            *depth = (*depth - 1).max(0);
        }
    }
}

fn parse_python_source(content: &str, rel_path: &str) -> ParsedFileSummary {
    let lines: Vec<&str> = content.lines().collect();
    let mut imports = Vec::new();
    let mut classes = Vec::new();
    let mut functions = Vec::new();

    let (Ok(class_re), Ok(func_start_re), Ok(call_re)) = (
        Regex::new(r"^(\s*)class\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:\((.*?)\))?\s*:"),
        Regex::new(r"^(\s*)(?:async\s+)?def\s+([A-Za-z_][A-Za-z0-9_]*)\s*\((.*)$"),
        Regex::new(r"\b([A-Za-z_][A-Za-z0-9_.]*)\s*\("),
    ) else {
        return ParsedFileSummary {
            rel_path: rel_path.to_string(),
            language: "python".to_string(),
            imports,
            classes,
            functions,
        };
    };

    let mut class_stack: Vec<(String, usize)> = Vec::new();
    let mut in_triple: Option<&str> = None;
    let mut outer_delim_depth: i32 = 0;
    let mut idx = 0usize;

    while idx < lines.len() {
        let line = lines[idx];
        let prev_triple = in_triple;
        in_triple = update_triple_quote_state(line, in_triple);
        if prev_triple.is_some() {
            idx += 1;
            continue;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            idx += 1;
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let was_in_outer_delim = outer_delim_depth > 0;
        let masked_current = mask_strings_and_comments(line);
        update_delim_depth(&masked_current, &mut outer_delim_depth);

        if was_in_outer_delim {
            idx += 1;
            continue;
        }

        while let Some((_, cls_indent)) = class_stack.last() {
            if indent <= *cls_indent {
                class_stack.pop();
            } else {
                break;
            }
        }

        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            imports.push(trimmed.to_string());
            idx += 1;
            continue;
        }

        if let Some(caps) = class_re.captures(line) {
            let cls_indent = caps.get(1).map(|m| m.as_str().len()).unwrap_or(0);
            if let Some(cls_m) = caps.get(2) {
                let cls_name = cls_m.as_str().to_string();
                classes.push(cls_name.clone());
                class_stack.push((cls_name, cls_indent));
            }
            idx += 1;
            continue;
        }

        if let Some(caps) = func_start_re.captures(line) {
            let fn_indent = caps.get(1).map(|m| m.as_str().len()).unwrap_or(0);
            let Some(name_m) = caps.get(2) else {
                idx += 1;
                continue;
            };
            let raw_name = name_m.as_str();
            let start_line = idx + 1;

            // Handle both single-line, multi-line, and one-liner Python def signatures
            let mut sig_end_idx = idx;
            let mut paren_depth: i32 = 0;
            let mut seen_open_paren = false;
            let mut sig_accum = Vec::new();
            let mut inline_body_masked = String::new();
            for (s_idx, &s_line) in lines.iter().enumerate().skip(idx) {
                let code_part = mask_strings_and_comments(s_line);
                sig_end_idx = s_idx;
                if let Some(colon_idx) =
                    scan_python_signature_colon(&code_part, &mut paren_depth, &mut seen_open_paren)
                {
                    sig_accum.push(s_line[..colon_idx].trim());
                    let rem = code_part[colon_idx + 1..].trim();
                    if !rem.is_empty() {
                        inline_body_masked = rem.to_string();
                    }
                    break;
                } else {
                    let raw_no_comment = strip_python_inline_comment(s_line);
                    sig_accum.push(raw_no_comment.trim());
                }
                if s_idx - idx > 15 {
                    break;
                }
            }

            let joined_sig = sig_accum.join(" ");
            let masked_sig = mask_strings_and_comments(&joined_sig);
            let args = masked_sig
                .find('(')
                .and_then(|open| {
                    let mut depth = 0i32;
                    let mut close_idx = None;
                    for (rel_i, ch) in masked_sig[open..].char_indices() {
                        if ch == '(' {
                            depth += 1;
                        } else if ch == ')' {
                            depth -= 1;
                            if depth == 0 {
                                close_idx = Some(open + rel_i);
                                break;
                            }
                        }
                    }
                    close_idx.map(|close| joined_sig[open + 1..close].trim().to_string())
                })
                .unwrap_or_default();

            let mut end_line = sig_end_idx + 1;
            if inline_body_masked.is_empty() {
                let mut sub_triple: Option<&str> = None;
                let mut body_delim_depth: i32 = 0;
                for (sub_idx, &sub_line) in lines.iter().enumerate().skip(sig_end_idx + 1) {
                    let was_in_triple = sub_triple.is_some();
                    if was_in_triple {
                        sub_triple = update_triple_quote_state(sub_line, sub_triple);
                        end_line = sub_idx + 1;
                        continue;
                    }
                    let sub_trim = sub_line.trim();
                    if sub_trim.is_empty() || sub_trim.starts_with('#') {
                        continue;
                    }
                    if body_delim_depth > 0 {
                        sub_triple = update_triple_quote_state(sub_line, sub_triple);
                        if sub_triple.is_none() {
                            update_delim_depth(&mask_strings_and_comments(sub_line), &mut body_delim_depth);
                        }
                        end_line = sub_idx + 1;
                        continue;
                    }
                    let sub_indent = sub_line.len() - sub_line.trim_start().len();
                    if sub_indent <= fn_indent {
                        break;
                    }
                    sub_triple = update_triple_quote_state(sub_line, sub_triple);
                    if sub_triple.is_none() {
                        update_delim_depth(&mask_strings_and_comments(sub_line), &mut body_delim_depth);
                    }
                    end_line = sub_idx + 1;
                }
            }

            let qual_name = if let Some((cls_name, cls_indent)) = class_stack.last() {
                if fn_indent > *cls_indent {
                    format!("{}.{}", cls_name, raw_name)
                } else {
                    raw_name.to_string()
                }
            } else {
                raw_name.to_string()
            };

            let fn_slice = &lines[start_line - 1..end_line];
            let fn_source = fn_slice.join("\n");
            let mut docstring = String::new();
            let body_start_offset = (sig_end_idx + 1).saturating_sub(start_line - 1);
            let mut code_start_offset = body_start_offset;

            if fn_slice.len() > body_start_offset {
                let first_body = strip_python_string_prefix(fn_slice[body_start_offset].trim());
                let delim = if first_body.starts_with("\"\"\"") {
                    Some("\"\"\"")
                } else if first_body.starts_with("'''") {
                    Some("'''")
                } else {
                    None
                };
                if let Some(d) = delim {
                    let after_open = &first_body[3..];
                    if let Some(close_pos) = after_open.find(d) {
                        docstring = after_open[..close_pos].trim().to_string();
                        code_start_offset = body_start_offset + 1;
                    } else {
                        let mut doc_lines = Vec::new();
                        if !after_open.trim().is_empty() {
                            doc_lines.push(after_open.trim());
                        }
                        code_start_offset = body_start_offset + 1;
                        for (d_idx, &d_line) in fn_slice.iter().enumerate().skip(body_start_offset + 1)
                        {
                            code_start_offset = d_idx + 1;
                            if let Some(c_pos) = d_line.find(d) {
                                let before = d_line[..c_pos].trim();
                                if !before.is_empty() {
                                    doc_lines.push(before);
                                }
                                break;
                            } else {
                                let t = d_line.trim();
                                if !t.is_empty() {
                                    doc_lines.push(t);
                                }
                            }
                        }
                        docstring = doc_lines.join(" ");
                    }
                }
            }

            let mut calls = Vec::new();
            let mut uses = Vec::new();
            if !inline_body_masked.is_empty() {
                extract_calls_from_masked(&inline_body_masked, &call_re, &mut calls, &mut uses);
            }
            let mut body_triple: Option<&str> = None;
            for body_line in fn_slice.iter().skip(code_start_offset) {
                let was_triple = body_triple.is_some();
                body_triple = update_triple_quote_state(body_line, body_triple);
                if was_triple || body_triple.is_some() {
                    continue;
                }
                let masked_body = mask_strings_and_comments(body_line);
                extract_calls_from_masked(&masked_body, &call_re, &mut calls, &mut uses);
            }

            functions.push(FunctionSpan {
                function_id: format!("FUNC:{}", qual_name),
                name: qual_name,
                start: start_line,
                end: end_line,
                signature: format!("def {}({})", raw_name, args),
                docstring,
                source: fn_source,
                language: "python".to_string(),
                calls,
                uses,
            });
        }
        idx += 1;
    }

    ParsedFileSummary {
        rel_path: rel_path.to_string(),
        language: "python".to_string(),
        imports,
        classes,
        functions,
    }
}

fn parse_brace_source(content: &str, rel_path: &str, lang: &str) -> ParsedFileSummary {
    let lines: Vec<&str> = content.lines().collect();
    let mut imports = Vec::new();
    let mut classes = Vec::new();
    let mut functions = Vec::new();

    let (Ok(class_re), Ok(fn_re), Ok(arrow_fn_re), Ok(ts_method_re), Ok(typed_method_re), Ok(call_re)) = (
        Regex::new(
            r"^(?:pub(?:\([^)]*\))?\s+|public\s+|private\s+|protected\s+|internal\s+|export\s+|default\s+|abstract\s+|final\s+|static\s+|sealed\s+|partial\s+)*(?:class|struct)\s+([A-Za-z_][A-Za-z0-9_]*)",
        ),
        Regex::new(
            r"(?:pub(?:\([^)]*\))?\s+)?(?:export\s+)?(?:default\s+)?(?:async\s+)?(?:fn|function|func)\s+(?:\([^)]*\)\s*)?([A-Za-z_][A-Za-z0-9_]*)\s*(?:<.*>)?\s*\(",
        ),
        Regex::new(
            r"^(?:export\s+)?(?:default\s+)?(?:const|let|var)\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?::[^=]+)?=\s*\(*\s*(?:async\s+)?(?:\([^)]*\)|[A-Za-z_][A-Za-z0-9_]*)\s*(?::[^=]+)?=>",
        ),
        Regex::new(
            r"^(?:public\s+|private\s+|protected\s+|readonly\s+|static\s+|async\s+)*([A-Za-z_][A-Za-z0-9_]*)\s*(?:<.*>)?\s*\([^;]*\{\s*$",
        ),
        Regex::new(
            r"^(?:(?:public|private|protected|internal|static|final|virtual|override|inline|constexpr|extern|unsigned|signed|const|struct)\s+)*(?:[A-Z_]+_PUBLIC\([^)]*\)\s*|[A-Za-z_][A-Za-z0-9_<>:,\[\]\*\?&\s]*?\s+|\*+)(?:[A-Za-z_][A-Za-z0-9_]*::)?([A-Za-z_][A-Za-z0-9_]*)\s*\([^;]*$",
        ),
        Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\("),
    ) else {
        return ParsedFileSummary {
            rel_path: rel_path.to_string(),
            language: lang.to_string(),
            imports,
            classes,
            functions,
        };
    };

    let mut in_block_comment = false;
    for (idx, &line) in lines.iter().enumerate() {
        let masked_line = mask_brace_line(line, &mut in_block_comment);
        let masked_trimmed = masked_line.trim();
        if masked_trimmed.is_empty() {
            continue;
        }
        let trimmed = line.trim();
        if masked_trimmed.starts_with("use ")
            || masked_trimmed.starts_with("import ")
            || masked_trimmed.starts_with("#include ")
        {
            imports.push(trimmed.to_string());
            continue;
        }
        if let Some(cls_cap) = class_re.captures(masked_trimmed)
            && let Some(cls_m) = cls_cap.get(1) {
                let cls_name = cls_m.as_str().to_string();
                if !classes.contains(&cls_name) {
                    classes.push(cls_name);
                }
            }
        let mut sig_start_idx = idx;
        let mut combined_buf = String::new();
        let matched_name = fn_re
            .captures(masked_trimmed)
            .and_then(|c| c.get(1).map(|m| m.as_str()))
            .or_else(|| {
                if matches!(lang, "typescript" | "javascript") {
                    arrow_fn_re
                        .captures(masked_trimmed)
                        .and_then(|c| c.get(1).map(|m| m.as_str()))
                        .or_else(|| {
                            ts_method_re
                                .captures(masked_trimmed)
                                .and_then(|c| c.get(1).map(|m| m.as_str()))
                        })
                } else if matches!(lang, "java" | "cpp" | "c" | "csharp") {
                    typed_method_re
                        .captures(masked_trimmed)
                        .and_then(|c| c.get(1).map(|m| m.as_str()))
                        .or_else(|| {
                            if idx > 0
                                && !line.starts_with(' ')
                                && !line.starts_with('\t')
                                && masked_trimmed.contains('(')
                                && !masked_trimmed.starts_with('#')
                            {
                                let mut dummy_bc = false;
                                let prev_masked = mask_brace_line(lines[idx - 1], &mut dummy_bc);
                                let prev_t = prev_masked.trim();
                                if !prev_t.is_empty()
                                    && !prev_t.starts_with('#')
                                    && !prev_t.starts_with('*')
                                    && !prev_t.ends_with(';')
                                    && !prev_t.ends_with('{')
                                    && !prev_t.ends_with('}')
                                    && !prev_t.ends_with(')')
                                    && !matches!(prev_t, "else" | "do")
                                {
                                    combined_buf = format!("{} {}", prev_t, masked_trimmed);
                                    if let Some(m) = typed_method_re
                                        .captures(&combined_buf)
                                        .and_then(|c| c.get(1).map(|m| m.as_str()))
                                    {
                                        sig_start_idx = idx - 1;
                                        return Some(m);
                                    }
                                }
                            }
                            None
                        })
                } else {
                    None
                }
            });

        if let Some(fn_name) = matched_name {
            if matches!(
                fn_name,
                "if" | "else"
                    | "for"
                    | "while"
                    | "switch"
                    | "catch"
                    | "match"
                    | "return"
                    | "sizeof"
                    | "constructor"
                    | "function"
                    | "fn"
                    | "func"
                    | "delete"
                    | "new"
                    | "throw"
                    | "typedef"
                    | "defined"
            ) {
                continue;
            }
            let start_line = sig_start_idx + 1;
            let is_arrow = matches!(lang, "typescript" | "javascript")
                && arrow_fn_re.is_match(masked_trimmed);
            let Some(end_line) = find_matching_brace_end(&lines, idx)
                .or(if is_arrow { Some(start_line) } else { None })
            else {
                continue;
            };
            let fn_slice = &lines[start_line - 1..end_line];
            let (sig, header_lines) = extract_brace_full_signature(
                fn_slice,
                is_arrow,
                (idx - sig_start_idx) + 1,
            );
            let mut calls = Vec::new();
            let mut body_block_comment = false;
            for h_line in fn_slice.iter().take(header_lines) {
                let _ = mask_brace_line(h_line, &mut body_block_comment);
            }
            for body_line in fn_slice.iter().skip(header_lines) {
                let masked_body = mask_brace_line(body_line, &mut body_block_comment);
                for cap in call_re.captures_iter(&masked_body) {
                    let Some(sym_m) = cap.get(1) else {
                        continue;
                    };
                    let sym = sym_m.as_str();
                    if !matches!(sym, "if" | "for" | "while" | "match" | "Some" | "Ok" | "Err")
                        && !calls.contains(&sym.to_string())
                    {
                        calls.push(sym.to_string());
                    }
                }
            }
            let docstring = extract_brace_docstring(&lines, sig_start_idx);
            functions.push(FunctionSpan {
                function_id: format!("FUNC:{}", fn_name),
                name: fn_name.to_string(),
                start: start_line,
                end: end_line,
                signature: sig,
                docstring,
                source: fn_slice.join("\n"),
                language: lang.to_string(),
                calls,
                uses: Vec::new(),
            });
        }
    }

    ParsedFileSummary {
        rel_path: rel_path.to_string(),
        language: lang.to_string(),
        imports,
        classes,
        functions,
    }
}

fn extract_brace_full_signature(
    fn_slice: &[&str],
    is_arrow: bool,
    min_header_lines: usize,
) -> (String, usize) {
    let mut parts: Vec<String> = Vec::new();
    let mut paren_depth: i32 = 0;
    let mut in_block_comment = false;
    let mut last_header_lines = min_header_lines.max(1);
    let max_scan = fn_slice.len().min(28);

    for (rel_idx, &raw_line) in fn_slice.iter().enumerate().take(max_scan) {
        let mut line_buf = String::with_capacity(raw_line.len());
        let raw_bytes = raw_line.as_bytes();
        let mut chars = raw_line.char_indices().peekable();
        let mut in_quote: Option<char> = None;
        let mut hit_body_start = false;

        while let Some((byte_idx, ch)) = chars.next() {
            if in_block_comment {
                if ch == '*' && chars.peek().map(|&(_, c)| c) == Some('/') {
                    let _ = chars.next();
                    in_block_comment = false;
                }
                continue;
            }
            if let Some(q) = in_quote {
                line_buf.push(ch);
                if ch == '\\' {
                    if let Some((_, esc)) = chars.next() {
                        line_buf.push(esc);
                    }
                    continue;
                }
                if ch == q {
                    in_quote = None;
                }
                continue;
            }
            if ch == '/' && chars.peek().map(|&(_, c)| c) == Some('/') {
                break;
            }
            if ch == '/' && chars.peek().map(|&(_, c)| c) == Some('*') {
                let _ = chars.next();
                in_block_comment = true;
                continue;
            }
            if ch == '\''
                && let Some(tok_len) = non_quote_apostrophe_len(raw_bytes, byte_idx)
            {
                line_buf.push(ch);
                while let Some(&(next_idx, _)) = chars.peek() {
                    if next_idx >= byte_idx + tok_len {
                        break;
                    }
                    if let Some((_, tc)) = chars.next() {
                        line_buf.push(tc);
                    }
                }
                continue;
            }
            if matches!(ch, '"' | '\'' | '`') {
                in_quote = Some(ch);
                line_buf.push(ch);
                continue;
            }
            if ch == '(' {
                paren_depth += 1;
                line_buf.push(ch);
                continue;
            }
            if ch == ')' {
                paren_depth = (paren_depth - 1).max(0);
                line_buf.push(ch);
                continue;
            }
            if paren_depth == 0 && (rel_idx + 1) >= min_header_lines {
                if ch == '{' {
                    hit_body_start = true;
                    break;
                }
                if is_arrow && ch == '=' && chars.peek().map(|&(_, c)| c) == Some('>') {
                    let _ = chars.next();
                    line_buf.push_str("=>");
                    hit_body_start = true;
                    break;
                }
            }
            line_buf.push(ch);
        }

        let piece = line_buf.trim();
        if !piece.is_empty() {
            parts.push(piece.to_string());
        }
        last_header_lines = rel_idx + 1;
        if hit_body_start {
            break;
        }
    }

    let joined = parts
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (joined, last_header_lines)
}

/// Extracts contiguous doc comments (`///`, `//!`, `//`, `/** ... */`, `/* ... */`) immediately
/// preceding `sig_start_idx` in brace-delimited languages, skipping intermediate attributes/annotations.
pub fn extract_brace_docstring(lines: &[&str], sig_start_idx: usize) -> String {
    let mut cursor = sig_start_idx;

    // Skip single-line or multi-line attributes/decorators (`#[...]`, `@...`, `[[...]]`, `[...]`)
    while cursor > 0 {
        let prev = lines[cursor - 1].trim();
        if prev.starts_with("#[")
            || prev.starts_with('@')
            || prev.starts_with("[[")
            || (prev.starts_with('[') && prev.ends_with(']') && !prev.starts_with("/*"))
        {
            cursor -= 1;
            continue;
        }
        if (prev.ends_with(")]") || prev.ends_with(']'))
            && !prev.starts_with("//")
            && !prev.ends_with("*/")
        {
            let min_attr = cursor.saturating_sub(10);
            let mut probe = cursor - 1;
            let mut attr_start = None;
            while probe >= min_attr {
                let pt = lines[probe].trim();
                if pt.starts_with("#[") || pt.starts_with("[[") || pt.starts_with('[') {
                    attr_start = Some(probe);
                    break;
                }
                if pt.ends_with(';') || pt.ends_with('{') || pt.ends_with('}') || probe == 0 {
                    break;
                }
                probe -= 1;
            }
            if let Some(a_start) = attr_start {
                cursor = a_start;
                continue;
            }
        }
        break;
    }

    if cursor == 0 {
        return String::new();
    }

    let top_prev = lines[cursor - 1].trim();
    if top_prev.starts_with("//") {
        let mut rev_lines = Vec::new();
        let mut i = cursor;
        while i > 0 {
            let t = lines[i - 1].trim();
            if let Some(rest) = t
                .strip_prefix("///")
                .or_else(|| t.strip_prefix("//!"))
                .or_else(|| t.strip_prefix("//"))
            {
                let cleaned = rest
                    .trim()
                    .replace("<summary>", "")
                    .replace("</summary>", "");
                let cleaned_trim = cleaned.trim();
                if !cleaned_trim.is_empty() {
                    rev_lines.push(cleaned_trim.to_string());
                }
                i -= 1;
            } else {
                break;
            }
        }
        rev_lines.reverse();
        return rev_lines.join(" ");
    }

    if top_prev.ends_with("*/") {
        let min_i = cursor.saturating_sub(60);
        let mut i = cursor;
        let mut block_start = None;
        while i > min_i {
            i -= 1;
            if lines[i].contains("/*") {
                block_start = Some(i);
                break;
            }
        }
        if let Some(b_start) = block_start
            && lines[b_start].trim().starts_with("/*")
        {
            let mut parts = Vec::new();
            for &b_line in &lines[b_start..cursor] {
                let mut s = b_line.trim();
                if let Some(pos) = s.find("/*") {
                    s = &s[pos + 2..];
                    s = s.trim_start_matches('*');
                }
                if let Some(end_pos) = s.rfind("*/") {
                    s = &s[..end_pos];
                }
                s = s.trim().trim_start_matches('*').trim();
                if !s.is_empty() {
                    parts.push(s.to_string());
                }
            }
            return parts.join(" ");
        }
    }

    String::new()
}

fn find_matching_brace_end(lines: &[&str], start_idx: usize) -> Option<usize> {
    let mut open_braces: i32 = 0;
    let mut paren_depth: i32 = 0;
    let mut found_first = false;
    let mut in_block_comment = false;

    for (idx, &line) in lines.iter().enumerate().skip(start_idx) {
        let masked = mask_brace_line(line, &mut in_block_comment);
        for &b in masked.as_bytes() {
            if !found_first {
                if b == b'(' {
                    paren_depth += 1;
                } else if b == b')' {
                    paren_depth = (paren_depth - 1).max(0);
                } else if (b == b';' || b == b'}') && paren_depth == 0 {
                    // Trait method prototype, extern declaration, or TypeScript overload without body
                    return None;
                }
                if paren_depth > 0 {
                    continue;
                }
            }
            if b == b'{' {
                open_braces += 1;
                found_first = true;
            } else if b == b'}' {
                open_braces -= 1;
                if found_first && open_braces <= 0 {
                    return Some(idx + 1);
                }
            }
        }
    }
    if found_first {
        Some((start_idx + 10).min(lines.len().max(1)))
    } else {
        None
    }
}

pub fn build_repo_graph_parallel(repo_path: &Path) -> RepoGraph {
    let mut source_files: Vec<(PathBuf, String, String)> = Vec::new();

    let walker = WalkDir::new(repo_path)
        .max_depth(16)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                let rel = e
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(e.path())
                    .to_string_lossy();
                return !is_noisy_dir(&name, &rel);
            }
            true
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
                && let Some(lang) = detect_language_for_ext(ext) {
                    let rel = entry
                        .path()
                        .strip_prefix(repo_path)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    if !is_declaration_file(&rel) {
                        source_files.push((entry.path().to_path_buf(), rel, lang.to_string()));
                    }
                }
    }

    source_files.sort_by(|a, b| a.1.cmp(&b.1));

    let summaries: Vec<ParsedFileSummary> = source_files
        .par_iter()
        .filter_map(|(abs_p, rel_p, lang)| {
            let content = read_bounded_source_file(abs_p)?;
            Some(parse_source_content(&content, rel_p, lang))
        })
        .collect();

    let mut graph = RepoGraph::default();
    let mut symbol_to_node_id: HashMap<String, String> = HashMap::new();
    let mut seen_edges: HashSet<(String, String, String)> = HashSet::new();

    for summary in &summaries {
        let file_id = format!("FILE:{}", summary.rel_path);
        let mod_name = module_path_from_rel(&summary.rel_path);
        graph.nodes.insert(
            file_id.clone(),
            GraphNode {
                id: file_id.clone(),
                node_type: "File".to_string(),
                name: summary.rel_path.clone(),
                kind: "project".to_string(),
                is_project_defined: true,
                defined_in: Some(summary.rel_path.clone()),
                module: Some(mod_name.clone()),
                start_line: Some(1),
                end_line: None,
                signature: None,
            },
        );

        for cls in &summary.classes {
            let cls_id = format!("CLASS:{}", cls);
            symbol_to_node_id.insert(cls.clone(), cls_id.clone());
            symbol_to_node_id.insert(format!("{}.{}", mod_name, cls), cls_id.clone());
            graph.nodes.insert(
                cls_id.clone(),
                GraphNode {
                    id: cls_id.clone(),
                    node_type: "Class".to_string(),
                    name: cls.clone(),
                    kind: "project".to_string(),
                    is_project_defined: true,
                    defined_in: Some(summary.rel_path.clone()),
                    module: Some(mod_name.clone()),
                    start_line: None,
                    end_line: None,
                    signature: None,
                },
            );
            if seen_edges.insert((file_id.clone(), cls_id.clone(), "defines".to_string())) {
                graph.edges.push(GraphEdge {
                    src: file_id.clone(),
                    dst: cls_id,
                    relation: "defines".to_string(),
                });
            }
        }

        for fn_span in &summary.functions {
            let fn_id = fn_span.function_id.clone();
            let short_name = fn_span.name.split('.').next_back().unwrap_or(&fn_span.name);
            symbol_to_node_id.insert(fn_span.name.clone(), fn_id.clone());
            symbol_to_node_id.insert(short_name.to_string(), fn_id.clone());
            symbol_to_node_id.insert(format!("{}.{}", mod_name, fn_span.name), fn_id.clone());

            graph.nodes.insert(
                fn_id.clone(),
                GraphNode {
                    id: fn_id.clone(),
                    node_type: "Function".to_string(),
                    name: fn_span.name.clone(),
                    kind: "project".to_string(),
                    is_project_defined: true,
                    defined_in: Some(summary.rel_path.clone()),
                    module: Some(mod_name.clone()),
                    start_line: Some(fn_span.start),
                    end_line: Some(fn_span.end),
                    signature: Some(fn_span.signature.clone()),
                },
            );
            if seen_edges.insert((file_id.clone(), fn_id.clone(), "defines".to_string())) {
                graph.edges.push(GraphEdge {
                    src: file_id.clone(),
                    dst: fn_id,
                    relation: "defines".to_string(),
                });
            }
        }
    }

    for summary in &summaries {
        for fn_span in &summary.functions {
            let src_id = &fn_span.function_id;
            for callee in &fn_span.calls {
                let short = callee.split('.').next_back().unwrap_or(callee);
                if let Some(dst_id) = symbol_to_node_id
                    .get(callee)
                    .or_else(|| symbol_to_node_id.get(short))
                    && dst_id != src_id {
                        let rel = if dst_id.starts_with("CLASS:") {
                            "instantiates"
                        } else {
                            "calls"
                        };
                        if seen_edges.insert((src_id.clone(), dst_id.clone(), rel.to_string())) {
                            graph.edges.push(GraphEdge {
                                src: src_id.clone(),
                                dst: dst_id.clone(),
                                relation: rel.to_string(),
                            });
                        }
                    }
            }
            for used in &fn_span.uses {
                if let Some(dst_id) = symbol_to_node_id.get(used)
                    && dst_id != src_id
                        && !seen_edges.contains(&(
                            src_id.clone(),
                            dst_id.clone(),
                            "instantiates".to_string(),
                        ))
                        && seen_edges.insert((src_id.clone(), dst_id.clone(), "uses".to_string()))
                    {
                        graph.edges.push(GraphEdge {
                            src: src_id.clone(),
                            dst: dst_id.clone(),
                            relation: "uses".to_string(),
                        });
                    }
            }
        }
    }

    enrich_graph_with_git_history(repo_path, &mut graph, &mut seen_edges);
    graph
}

fn enrich_graph_with_git_history(
    repo_path: &Path,
    graph: &mut RepoGraph,
    seen_edges: &mut HashSet<(String, String, String)>,
) {
    let output = Command::new("git")
        .args([
            "log",
            "-n",
            "50",
            "--pretty=format:COMMIT:%h|%an",
            "--name-only",
        ])
        .current_dir(repo_path)
        .output();

    let Ok(out) = output else {
        return;
    };
    if !out.status.success() {
        return;
    }

    let text = String::from_utf8_lossy(&out.stdout);
    let mut current_commit: Option<String> = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(rest) = line.strip_prefix("COMMIT:") {
            let mut parts = rest.splitn(2, '|');
            let hash = parts.next().unwrap_or("unknown").trim();
            let author = parts.next().unwrap_or("unknown").trim();
            let commit_id = format!("COMMIT:{}", hash);
            let dev_id = format!("DEV:{}", author);

            graph.nodes.entry(dev_id.clone()).or_insert(GraphNode {
                id: dev_id.clone(),
                node_type: "Developer".to_string(),
                name: author.to_string(),
                kind: "git".to_string(),
                is_project_defined: true,
                defined_in: None,
                module: None,
                start_line: None,
                end_line: None,
                signature: None,
            });
            graph.nodes.insert(
                commit_id.clone(),
                GraphNode {
                    id: commit_id.clone(),
                    node_type: "Commit".to_string(),
                    name: hash.to_string(),
                    kind: "git".to_string(),
                    is_project_defined: true,
                    defined_in: None,
                    module: None,
                    start_line: None,
                    end_line: None,
                    signature: None,
                },
            );
            if seen_edges.insert((dev_id.clone(), commit_id.clone(), "authored_by".to_string())) {
                graph.edges.push(GraphEdge {
                    src: dev_id,
                    dst: commit_id.clone(),
                    relation: "authored_by".to_string(),
                });
            }
            current_commit = Some(commit_id);
        } else if let Some(commit_id) = &current_commit {
            let norm_file = line.replace('\\', "/");
            let file_id = format!("FILE:{}", norm_file);
            if graph.nodes.contains_key(&file_id)
                && seen_edges.insert((commit_id.clone(), file_id.clone(), "modifies".to_string()))
            {
                graph.edges.push(GraphEdge {
                    src: commit_id.clone(),
                    dst: file_id,
                    relation: "modifies".to_string(),
                });
            }
        }
    }
}

pub fn module_path_from_rel(rel_path: &str) -> String {
    let mut s = rel_path.replace('\\', "/");
    for prefix in ["src/", "lib/"] {
        if let Some(stripped) = s.strip_prefix(prefix) {
            s = stripped.to_string();
        }
    }
    for suffix in [
        "/__init__.py",
        ".py",
        ".rs",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        ".mjs",
        ".go",
        ".java",
        ".cpp",
        ".cc",
        ".c",
        ".hpp",
        ".h",
    ] {
        if let Some(stripped) = s.strip_suffix(suffix) {
            s = stripped.to_string();
            break;
        }
    }
    s.replace('/', ".")
}

pub fn suggest_test_file(source_file: &str) -> String {
    let norm = source_file.replace('\\', "/");
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty()).collect();
    let Some(&filename) = parts.last() else {
        return "tests/test_generated.py".to_string();
    };
    let (stem, ext) = filename.rsplit_once('.').unwrap_or((filename, "py"));
    match ext {
        "ts" | "tsx" => format!("tests/{}.test.ts", stem),
        "js" | "jsx" | "mjs" => format!("tests/{}.test.js", stem),
        "go" => format!("{}_test.go", stem),
        _ => {
            let test_name = if stem.starts_with("test_") {
                format!("{}.py", stem)
            } else {
                format!("test_{}.py", stem)
            };
            if parts.len() > 1 && matches!(parts[0], "src" | "proven" | "lib" | "app") {
                let sub = parts[1..parts.len() - 1].join("/");
                if !sub.is_empty() {
                    return format!("tests/{}/{}", sub, test_name);
                }
            }
            format!("tests/{}", test_name)
        }
    }
}

pub fn resolve_source_file(repo_path: &Path, target_id: &str, explicit_file: Option<&str>) -> Option<String> {
    if let Some(f) = explicit_file
        && !f.trim().is_empty() {
            return sanitize_repo_relative_path(repo_path, f);
        }
    let clean = target_id
        .trim()
        .strip_prefix("FILE:")
        .unwrap_or(target_id.trim())
        .replace('\\', "/");
    if clean.is_empty() {
        return None;
    }
    let candidate = clean.split("::").next().unwrap_or(&clean);
    let has_source_ext = Path::new(candidate)
        .extension()
        .and_then(|s| s.to_str())
        .and_then(detect_language_for_ext)
        .is_some();
    if clean.contains('/') || has_source_ext {
        if let Some(safe_rel) = sanitize_repo_relative_path(repo_path, candidate)
            && repo_path.join(&safe_rel).exists() {
                return Some(safe_rel);
            }
        return None;
    }

    let sym = target_id
        .trim()
        .strip_prefix("FUNC:")
        .or_else(|| target_id.trim().strip_prefix("CLASS:"))
        .unwrap_or(target_id.trim())
        .trim();
    let short_sym = sym.split('.').next_back().unwrap_or(sym).trim();
    if sym.is_empty() || short_sym.is_empty() {
        return None;
    }

    let index_path = repo_path.join(".proven").join("ast_index.json");
    if let Ok(raw_idx) = fs::read_to_string(&index_path)
        && let Ok(idx_json) = serde_json::from_str::<Value>(&raw_idx)
            && let Some(map) = idx_json.get("symbol_to_file").and_then(|v| v.as_object()) {
                for key in [target_id.trim(), sym, short_sym] {
                    if let Some(rel) = map.get(key).and_then(|v| v.as_str())
                        && let Some(safe_rel) = sanitize_repo_relative_path(repo_path, rel) {
                            let abs_p = repo_path.join(&safe_rel);
                            if let Some(ext) = abs_p.extension().and_then(|s| s.to_str())
                                && let Some(lang) = detect_language_for_ext(ext)
                                    && let Some(content) = read_bounded_source_file(&abs_p) {
                                        let parsed = parse_source_content(&content, &safe_rel, lang);
                                        if find_matching_function(&parsed.functions, target_id).is_some()
                                            || parsed.classes.iter().any(|c| c == sym || c == short_sym)
                                        {
                                            return Some(safe_rel);
                                        }
                                    }
                        }
                }
            }

    let mut source_files: Vec<(PathBuf, String, String)> = Vec::new();
    let walker = WalkDir::new(repo_path)
        .max_depth(16)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                let rel = e
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(e.path())
                    .to_string_lossy();
                return !is_noisy_dir(&name, &rel);
            }
            true
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
                && let Some(lang) = detect_language_for_ext(ext) {
                    let rel = entry
                        .path()
                        .strip_prefix(repo_path)
                        .unwrap_or(entry.path())
                        .to_string_lossy()
                        .replace('\\', "/");
                    if !is_declaration_file(&rel) {
                        source_files.push((entry.path().to_path_buf(), rel, lang.to_string()));
                    }
                }
    }

    source_files.sort_by(|a, b| a.1.cmp(&b.1));

    let summaries: Vec<ParsedFileSummary> = source_files
        .par_iter()
        .filter_map(|(abs_p, rel_p, lang)| {
            let content = read_bounded_source_file(abs_p)?;
            Some(parse_source_content(&content, rel_p, lang))
        })
        .collect();

    let mut symbol_to_file: HashMap<String, String> = HashMap::new();
    let mut symbol_priority: HashMap<String, (bool, usize)> = HashMap::new();
    let mut functions_indexed: usize = 0;
    let mut classes_indexed: usize = 0;
    let mut edges_indexed: usize = 0;

    let mut best_match: Option<(String, (bool, bool, usize))> = None;

    for summary in &summaries {
        let is_non_test = !is_test_file_path(&summary.rel_path);
        classes_indexed += summary.classes.len();
        functions_indexed += summary.functions.len();
        edges_indexed += summary.classes.len() + summary.functions.len();

        for cls in &summary.classes {
            for key in [format!("CLASS:{}", cls), cls.clone()] {
                let prio = (is_non_test, 1usize);
                let should_update = symbol_priority
                    .get(&key)
                    .map(|&old| prio > old)
                    .unwrap_or(true);
                if should_update {
                    symbol_priority.insert(key.clone(), prio);
                    symbol_to_file.insert(key, summary.rel_path.clone());
                }
            }
        }

        for fn_span in &summary.functions {
            edges_indexed += fn_span.calls.len() + fn_span.uses.len();
            let span_len = fn_span.end.saturating_sub(fn_span.start) + 1;
            let short_name = fn_span
                .name
                .split('.')
                .next_back()
                .unwrap_or(&fn_span.name)
                .to_string();
            for key in [
                fn_span.function_id.clone(),
                fn_span.name.clone(),
                format!("FUNC:{}", short_name),
                short_name,
            ] {
                let prio = (is_non_test, span_len);
                let should_update = symbol_priority
                    .get(&key)
                    .map(|&old| prio > old)
                    .unwrap_or(true);
                if should_update {
                    symbol_priority.insert(key.clone(), prio);
                    symbol_to_file.insert(key, summary.rel_path.clone());
                }
            }
        }

        if let Some(matched_fn) = find_matching_function(&summary.functions, target_id) {
            let is_exact = matched_fn.name == sym || matched_fn.function_id == target_id.trim();
            let span_len = matched_fn.end.saturating_sub(matched_fn.start) + 1;
            let rank = (is_non_test, is_exact, span_len);
            if best_match.as_ref().map(|(_, r)| rank > *r).unwrap_or(true) {
                best_match = Some((summary.rel_path.clone(), rank));
            }
        } else if summary.classes.iter().any(|c| c == sym || c == short_sym) {
            let is_exact = summary.classes.iter().any(|c| c == sym);
            let rank = (is_non_test, is_exact, 1usize);
            if best_match.as_ref().map(|(_, r)| rank > *r).unwrap_or(true) {
                best_match = Some((summary.rel_path.clone(), rank));
            }
        }
    }

    let ast_nodes = summaries.len() + classes_indexed + functions_indexed;
    let index_payload = json!({
        "schema_version": "1.0",
        "files_indexed": summaries.len(),
        "functions_indexed": functions_indexed,
        "classes_indexed": classes_indexed,
        "ast_nodes": ast_nodes,
        "edges_indexed": edges_indexed,
        "symbol_to_file": symbol_to_file,
    });
    if let Some(parent) = index_path.parent() {
        let _ = fs::create_dir_all(parent);
        if let Ok(serialized) = serde_json::to_string(&index_payload) {
            let _ = fs::write(&index_path, serialized);
        }
    }

    best_match.map(|(rel, _)| rel)
}

fn error_target_context(
    effective_tid: String,
    language: &str,
    resolved_rel: String,
    blind: bool,
) -> TargetContext {
    TargetContext {
        status: "error".to_string(),
        track: "rust_native".to_string(),
        target_id: effective_tid,
        language: language.to_string(),
        source_file: resolved_rel,
        module_path: String::new(),
        signature: String::new(),
        docstring: String::new(),
        source_code: String::new(),
        imports: Vec::new(),
        functions: Vec::new(),
        callers: Vec::new(),
        callees: Vec::new(),
        suggested_test_file: String::new(),
        existing_test_file_exists: false,
        existing_test_preview: None,
        repo_memory: None,
        graph_rag_prompt: None,
        pinned_lessons: Vec::new(),
        blind_mode: blind,
        discovered_specs: Vec::new(),
        inferred_invariants: Vec::new(),
        missing_sibling_guards: Vec::new(),
        unverified_literals: Vec::new(),
        integration_slice: None,
        concurrency_risk: None,
        config_audit: None,
    }
}

fn contains_ident_at(haystack: &str, ident: &str) -> Option<usize> {
    if ident.is_empty() {
        return None;
    }
    let bytes = haystack.as_bytes();
    let mut offset = 0usize;
    while let Some(rel) = haystack[offset..].find(ident) {
        let start = offset + rel;
        let end = start + ident.len();
        let prev_ok = start == 0
            || !bytes[start - 1].is_ascii_alphanumeric() && bytes[start - 1] != b'_';
        let next_ok =
            end >= bytes.len() || !bytes[end].is_ascii_alphanumeric() && bytes[end] != b'_';
        if prev_ok && next_ok {
            return Some(start);
        }
        offset = start + 1;
    }
    None
}

fn find_call_excerpt_in_fn(fn_span: &FunctionSpan, short_sym: &str) -> String {
    if short_sym.is_empty() {
        return fn_span.signature.clone();
    }
    let raw_lines: Vec<&str> = fn_span.source.lines().collect();
    if raw_lines.is_empty() {
        return fn_span.signature.clone();
    }
    let is_brace = fn_span.language != "python" && is_brace_language_source(&fn_span.source);
    let mut masked_lines: Vec<String> = Vec::with_capacity(raw_lines.len());
    let mut in_triple: Option<&str> = None;
    let mut in_bc = false;

    for &line in &raw_lines {
        if !is_brace {
            let was_triple = in_triple.is_some();
            in_triple = update_triple_quote_state(line, in_triple);
            if was_triple || in_triple.is_some() {
                masked_lines.push(String::new());
            } else {
                masked_lines.push(mask_python_line(line));
            }
        } else {
            masked_lines.push(mask_brace_line(line, &mut in_bc));
        }
    }

    let start_idx = if raw_lines.len() > 1 { 1 } else { 0 };
    for idx in start_idx..raw_lines.len() {
        let Some(sym_pos) = contains_ident_at(&masked_lines[idx], short_sym) else {
            continue;
        };
        let mut paren_depth = 0i32;
        let mut seen_open = false;
        let mut parts = Vec::new();
        let max_end = (idx + 12).min(raw_lines.len());

        for (cur_idx, &cur_raw) in raw_lines
            .iter()
            .enumerate()
            .take(max_end)
            .skip(idx)
        {
            let scan_slice = if cur_idx == idx {
                &masked_lines[cur_idx][sym_pos..]
            } else {
                masked_lines[cur_idx].as_str()
            };
            for ch in scan_slice.chars() {
                if ch == '(' {
                    paren_depth += 1;
                    seen_open = true;
                } else if ch == ')' && seen_open {
                    paren_depth -= 1;
                }
            }
            let cleaned = if !is_brace {
                strip_python_inline_comment(cur_raw).trim()
            } else {
                cur_raw.split("//").next().unwrap_or(cur_raw).trim()
            };
            if !cleaned.is_empty() {
                parts.push(cleaned);
            }
            if !seen_open || paren_depth <= 0 {
                break;
            }
        }

        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    fn_span.signature.clone()
}

struct ScopedCallerLookup {
    callers: Vec<String>,
    caller_snippets: Vec<(String, String, String)>,
    project_symbols: HashMap<String, String>,
}

fn collect_callers_and_project_symbols(
    repo_path: &Path,
    resolved_rel: &str,
    effective_tid: &str,
    clean_sym: &str,
    short_sym: &str,
    matched_fn: Option<&FunctionSpan>,
    parsed: &ParsedFileSummary,
) -> ScopedCallerLookup {
    let mut callers = Vec::new();
    let mut caller_snippets = Vec::new();
    let mut project_symbols: HashMap<String, String> = HashMap::new();
    let dot_suffix = format!(".{}", short_sym);
    let matched_fn_id = matched_fn.map(|f| f.function_id.as_str());
    let raw_callees: &[String] = matched_fn.map_or(&[], |f| f.calls.as_slice());

    for cls in &parsed.classes {
        project_symbols.insert(cls.clone(), format!("CLASS:{}", cls));
    }
    for f in &parsed.functions {
        let f_short = f.name.split('.').next_back().unwrap_or(&f.name);
        project_symbols.insert(f.name.clone(), f.function_id.clone());
        project_symbols.insert(f_short.to_string(), f.function_id.clone());

        let is_self = f.function_id == effective_tid
            || matched_fn_id == Some(f.function_id.as_str())
            || f.name == clean_sym;
        if !is_self
            && !short_sym.is_empty()
            && f.calls
                .iter()
                .any(|c| c == clean_sym || c == short_sym || c.ends_with(&dot_suffix))
        {
            callers.push(f.function_id.clone());
            caller_snippets.push((
                f.function_id.clone(),
                resolved_rel.to_string(),
                find_call_excerpt_in_fn(f, short_sym),
            ));
        }
    }

    let callee_shorts: Vec<String> = raw_callees
        .iter()
        .map(|c| c.split('.').next_back().unwrap_or(c).to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if short_sym.is_empty() && callee_shorts.is_empty() {
        return ScopedCallerLookup {
            callers,
            caller_snippets,
            project_symbols,
        };
    }

    let mut other_files: Vec<(PathBuf, String, String)> = Vec::new();
    let walker = WalkDir::new(repo_path)
        .max_depth(12)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            if e.file_type().is_dir() {
                if e.depth() == 0 {
                    return true;
                }
                let name = e.file_name().to_string_lossy();
                let rel = e
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(e.path())
                    .to_string_lossy();
                return !is_noisy_dir(&name, &rel);
            }
            true
        });

    for entry in walker.flatten() {
        if entry.file_type().is_file()
            && let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
            && let Some(lang) = detect_language_for_ext(ext)
        {
            let rel = entry
                .path()
                .strip_prefix(repo_path)
                .unwrap_or(entry.path())
                .to_string_lossy()
                .replace('\\', "/");
            if rel != resolved_rel && !is_declaration_file(&rel) && !is_test_file_path(&rel) {
                other_files.push((entry.path().to_path_buf(), rel, lang.to_string()));
                if other_files.len() >= 400 {
                    break;
                }
            }
        }
    }
    other_files.sort_by(|a, b| a.1.cmp(&b.1));

    let matched_summaries: Vec<ParsedFileSummary> = other_files
        .par_iter()
        .filter_map(|(abs_p, rel_p, lang)| {
            let txt = read_bounded_source_file(abs_p)?;
            let mentions_target = !short_sym.is_empty() && txt.contains(short_sym);
            let mentions_callee = callee_shorts.iter().any(|cs| txt.contains(cs.as_str()));
            if !mentions_target && !mentions_callee {
                return None;
            }
            Some(parse_source_content(&txt, rel_p, lang))
        })
        .collect();

    for summary in &matched_summaries {
        for cls in &summary.classes {
            project_symbols
                .entry(cls.clone())
                .or_insert_with(|| format!("CLASS:{}", cls));
        }
        for f in &summary.functions {
            let f_short = f.name.split('.').next_back().unwrap_or(&f.name);
            project_symbols
                .entry(f.name.clone())
                .or_insert_with(|| f.function_id.clone());
            project_symbols
                .entry(f_short.to_string())
                .or_insert_with(|| f.function_id.clone());

            if !short_sym.is_empty()
                && f.calls
                    .iter()
                    .any(|c| c == clean_sym || c == short_sym || c.ends_with(&dot_suffix))
                && !callers.contains(&f.function_id)
            {
                callers.push(f.function_id.clone());
                caller_snippets.push((
                    f.function_id.clone(),
                    summary.rel_path.clone(),
                    find_call_excerpt_in_fn(f, short_sym),
                ));
            }
        }
    }

    ScopedCallerLookup {
        callers,
        caller_snippets,
        project_symbols,
    }
}

pub fn extract_scoped_context(
    repo_path: &Path,
    target_id: Option<&str>,
    source_file: Option<&str>,
    include_graph_rag: bool,
) -> TargetContext {
    extract_scoped_context_with_options(
        repo_path,
        target_id,
        source_file,
        include_graph_rag,
        false,
        None,
    )
}

pub fn extract_scoped_context_with_options(
    repo_path: &Path,
    target_id: Option<&str>,
    source_file: Option<&str>,
    include_graph_rag: bool,
    blind: bool,
    explicit_spec: Option<&str>,
) -> TargetContext {
    let tid = target_id.unwrap_or("").trim();
    let is_empty_prefixed_target = matches!(tid, "FUNC:" | "CLASS:" | "FILE:");
    let resolved_rel = if is_empty_prefixed_target {
        String::new()
    } else {
        resolve_source_file(repo_path, tid, source_file).unwrap_or_default()
    };
    let effective_tid = if tid.is_empty() {
        format!("FILE:{}", resolved_rel)
    } else {
        tid.to_string()
    };

    let abs_source = repo_path.join(&resolved_rel);
    let Some(content) = (if resolved_rel.is_empty() {
        None
    } else {
        read_bounded_source_file(&abs_source)
    }) else {
        return error_target_context(effective_tid, "unknown", resolved_rel, blind);
    };

    let ext = abs_source
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("py");
    let lang = detect_language_for_ext(ext).unwrap_or("python");
    let parsed = parse_source_content(&content, &resolved_rel, lang);

    let after_colons = effective_tid.rsplit("::").next().unwrap_or(&effective_tid);
    let clean_sym = after_colons
        .strip_prefix("FUNC:")
        .or_else(|| after_colons.strip_prefix("CLASS:"))
        .unwrap_or(after_colons);
    let short_sym = clean_sym.split('.').next_back().unwrap_or(clean_sym);

    let matched_fn = find_matching_function(&parsed.functions, &effective_tid);
    let has_class = parsed
        .classes
        .iter()
        .any(|c| c == clean_sym || c == short_sym);
    if (effective_tid.starts_with("FUNC:") && matched_fn.is_none())
        || (effective_tid.starts_with("CLASS:") && matched_fn.is_none() && !has_class)
    {
        return error_target_context(effective_tid, lang, resolved_rel, blind);
    }

    let (signature, docstring, raw_source_code, raw_callees, fn_sigs, fn_start_line) =
        if let Some(fn_span) = matched_fn {
            (
                fn_span.signature.clone(),
                fn_span.docstring.clone(),
                fn_span.source.clone(),
                fn_span.calls.clone(),
                vec![fn_span.signature.clone()],
                fn_span.start,
            )
        } else {
            let sigs: Vec<String> = parsed.functions.iter().map(|f| f.signature.clone()).collect();
            (
                format!("File: {} ({} functions/methods)", resolved_rel, sigs.len()),
                String::new(),
                content.clone(),
                Vec::new(),
                sigs,
                1usize,
            )
        };

    let ScopedCallerLookup {
        callers,
        caller_snippets,
        project_symbols,
    } = collect_callers_and_project_symbols(
        repo_path,
        &resolved_rel,
        &effective_tid,
        clean_sym,
        short_sym,
        matched_fn,
        &parsed,
    );

    let discovered_specs = discover_target_specs(
        repo_path,
        &resolved_rel,
        short_sym,
        &docstring,
        explicit_spec,
        &caller_snippets,
    );
    let inferred_invariants = if matched_fn.is_some() {
        infer_signature_invariants(&signature, lang)
    } else {
        Vec::new()
    };
    let missing_sibling_guards = if let Some(fn_span) = matched_fn {
        detect_missing_sibling_guards_in_file(fn_span, &parsed.functions, lang, Some(&content))
    } else {
        Vec::new()
    };

    if !blind {
        let _ = GraphMemoryManager::bind_meta_axioms_to_target(
            repo_path,
            &effective_tid,
            &raw_source_code,
        );
    }
    let repo_memory = GraphMemoryManager::get_scoped_memory_prompt(repo_path, &effective_tid);
    let pinned_lessons = GraphMemoryManager::get_pinned_lessons(repo_path, Some(&effective_tid));

    let suggested_test = suggest_test_file(&resolved_rel);
    let abs_test = repo_path.join(&suggested_test);
    let existing_test = read_bounded_source_file(&abs_test);

    let (
        source_code,
        callees,
        graph_rag_prompt,
        unverified_literals,
        integration_slice,
        concurrency_risk,
        config_audit,
    ) = if blind {
        (String::new(), Vec::new(), None, Vec::new(), None, None, None)
    } else {
        let rag_opt = if include_graph_rag {
            let rag = CodeGraphRAG::build_prompt_from_scoped(&effective_tid, &raw_callees, 120);
            if rag.is_empty() { None } else { Some(rag) }
        } else {
            None
        };
        let unverified = collect_unverified_literals(
            &raw_source_code,
            lang,
            &discovered_specs,
            &content,
            fn_start_line,
        );
        let slice_hint = detect_integration_slice(
            &effective_tid,
            &raw_source_code,
            lang,
            &callers,
            &raw_callees,
            &project_symbols,
        );
        let conc_hint = detect_concurrency_risk(&signature, &raw_source_code, lang);
        let cfg_hint = if !resolved_rel.is_empty() {
            let audit =
                audit_changed_files_env_keys(repo_path, std::slice::from_ref(&resolved_rel));
            if audit.referenced_env_keys.is_empty() {
                None
            } else {
                Some(audit)
            }
        } else {
            None
        };
        (
            raw_source_code,
            raw_callees,
            rag_opt,
            unverified,
            slice_hint,
            conc_hint,
            cfg_hint,
        )
    };

    TargetContext {
        status: "success".to_string(),
        track: "rust_native".to_string(),
        target_id: effective_tid,
        language: lang.to_string(),
        source_file: resolved_rel.clone(),
        module_path: module_path_from_rel(&resolved_rel),
        signature,
        docstring,
        source_code,
        imports: parsed.imports,
        functions: fn_sigs,
        callers,
        callees,
        suggested_test_file: suggested_test,
        existing_test_file_exists: existing_test.is_some(),
        existing_test_preview: existing_test.map(|s| s.chars().take(2000).collect()),
        repo_memory,
        graph_rag_prompt,
        pinned_lessons,
        blind_mode: blind,
        discovered_specs,
        inferred_invariants,
        missing_sibling_guards,
        unverified_literals,
        integration_slice,
        concurrency_risk,
        config_audit,
    }
}

pub fn is_test_file_path(rel_path: &str) -> bool {
    let norm = rel_path.replace('\\', "/");
    let lower = norm.to_lowercase();
    if lower.starts_with("tests/")
        || lower.starts_with("test/")
        || lower.contains("/tests/")
        || lower.contains("/test/")
    {
        return true;
    }
    let file_name = lower.rsplit('/').next().unwrap_or(&lower);
    file_name.starts_with("test_")
        || file_name.ends_with("_test.py")
        || file_name.ends_with("_test.go")
        || file_name.ends_with("_test.rs")
        || file_name.ends_with(".test.ts")
        || file_name.ends_with(".test.tsx")
        || file_name.ends_with(".test.js")
        || file_name.ends_with(".test.jsx")
        || file_name.ends_with(".spec.ts")
        || file_name.ends_with(".spec.js")
}

pub fn scan_repository_changes(repo_path: &Path, base: &str, head: &str) -> Value {
    scan_repository_changes_with_options(repo_path, base, head, false)
}

fn push_valid_changed_source_file(repo_path: &Path, candidate: &str, out: &mut Vec<String>) {
    let f = candidate.trim().replace('\\', "/");
    if let Some(safe_f) = sanitize_repo_relative_path(repo_path, &f)
        && !is_noisy_dir(&safe_f, &safe_f)
        && !is_test_file_path(&safe_f)
        && !is_declaration_file(&safe_f)
    {
        let abs_f = repo_path.join(&safe_f);
        let is_supported_source = abs_f
            .extension()
            .and_then(|s| s.to_str())
            .and_then(detect_language_for_ext)
            .is_some();
        if is_supported_source
            && let Ok(meta) = fs::metadata(&abs_f)
            && meta.is_file()
            && meta.len() <= MAX_SOURCE_FILE_BYTES
        {
            out.push(safe_f);
        }
    }
}

pub fn scan_repository_changes_with_options(
    repo_path: &Path,
    base: &str,
    head: &str,
    fan_out: bool,
) -> Value {
    let mut changed_files: Vec<String> = Vec::new();
    if is_safe_git_ref(base) && is_safe_git_ref(head) {
        let diff_out = Command::new("git")
            .args(["diff", "--name-only", &format!("{}...{}", base, head), "--"])
            .current_dir(repo_path)
            .output();

        if let Ok(out) = diff_out
            && out.status.success()
        {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                push_valid_changed_source_file(repo_path, line, &mut changed_files);
            }
        }

        if changed_files.is_empty() {
            let wt_out = Command::new("git")
                .args(["diff", "--name-only", "HEAD", "--"])
                .current_dir(repo_path)
                .output();
            if let Ok(out) = wt_out
                && out.status.success()
            {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    push_valid_changed_source_file(repo_path, line, &mut changed_files);
                }
            }
            let untracked_out = Command::new("git")
                .args(["ls-files", "--others", "--exclude-standard"])
                .current_dir(repo_path)
                .output();
            if let Ok(out) = untracked_out
                && out.status.success()
            {
                for line in String::from_utf8_lossy(&out.stdout).lines() {
                    push_valid_changed_source_file(repo_path, line, &mut changed_files);
                }
            }
        }
    }

    if changed_files.is_empty() {
        let walker = WalkDir::new(repo_path)
            .max_depth(16)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if e.file_type().is_dir() {
                    if e.depth() == 0 {
                        return true;
                    }
                    let name = e.file_name().to_string_lossy();
                    let rel = e
                        .path()
                        .strip_prefix(repo_path)
                        .unwrap_or(e.path())
                        .to_string_lossy();
                    return !is_noisy_dir(&name, &rel);
                }
                true
            });
        for entry in walker.flatten() {
            if entry.file_type().is_file()
                && let Some(ext) = entry.path().extension().and_then(|s| s.to_str())
                && detect_language_for_ext(ext).is_some()
            {
                let rel = entry
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                if !is_test_file_path(&rel)
                    && !is_declaration_file(&rel)
                    && let Ok(meta) = entry.metadata()
                    && meta.len() <= MAX_SOURCE_FILE_BYTES
                {
                    changed_files.push(rel);
                }
            }
        }
    }

    changed_files.sort();
    changed_files.dedup();

    let config_audit = audit_changed_files_env_keys(repo_path, &changed_files);
    let mut warnings: Vec<String> = Vec::new();
    if !config_audit.missing_env_example_keys.is_empty() {
        warnings.push(format!(
            "Config audit: environment variable key(s) [{}] referenced in source files are missing from .env.example / .env.sample / .env.template.",
            config_audit.missing_env_example_keys.join(", ")
        ));
    }

    let mut changed_nodes = Vec::new();
    let mut missing_coverage = Vec::new();
    let mut fan_out_tasks = Vec::new();
    let mut seen_targets = HashSet::new();

    for rel_file in &changed_files {
        let abs_p = repo_path.join(rel_file);
        let Some(content) = read_bounded_source_file(&abs_p) else {
            continue;
        };
        let ext = abs_p.extension().and_then(|s| s.to_str()).unwrap_or("py");
        let Some(lang) = detect_language_for_ext(ext) else {
            continue;
        };
        let summary = parse_source_content(&content, rel_file, lang);
        for fn_span in summary.functions {
            let short_fn = fn_span.name.split('.').next_back().unwrap_or(&fn_span.name);
            if short_fn.starts_with("test_") {
                continue;
            }
            if seen_targets.insert(fn_span.function_id.clone()) {
                let suggested = suggest_test_file(rel_file);
                changed_nodes.push(json!({
                    "id": fn_span.function_id,
                    "label": fn_span.name,
                    "type": "Function",
                    "file": rel_file,
                }));
                missing_coverage.push(json!({
                    "target_id": fn_span.function_id,
                    "file": rel_file,
                    "reason": "Requires runtime execution and AST mutation proof",
                    "suggested_action": format!("Write tests in {} and run proven verify-proof --mutation-check", suggested),
                    "suggested_test_file": suggested,
                }));
                if fan_out {
                    fan_out_tasks.push(json!({
                        "task_id": format!("TASK:{}", fn_span.function_id),
                        "target_id": fn_span.function_id,
                        "source_file": rel_file,
                        "suggested_test_file": suggested,
                        "blind_context_cmd": format!("proven context --target \"{}\" --file \"{}\" --blind --json", fn_span.function_id, rel_file),
                        "context_cmd": format!("proven context --target \"{}\" --file \"{}\" --json", fn_span.function_id, rel_file),
                        "verify_cmd": format!("proven verify-proof --target \"{}\" --test \"{}\" --file \"{}\" --mutation-check --json", fn_span.function_id, suggested, rel_file),
                    }));
                }
            }
        }
    }

    let mut out = json!({
        "status": "success",
        "track": "rust_native",
        "language": detect_repo_language(repo_path),
        "changed_count": changed_nodes.len(),
        "missing_count": missing_coverage.len(),
        "changed_files": changed_files,
        "changed_nodes": changed_nodes,
        "missing_coverage": missing_coverage,
        "config_audit": config_audit,
        "warnings": warnings,
    });
    if fan_out {
        out["fan_out_tasks"] = json!(fan_out_tasks);
    }
    out
}
