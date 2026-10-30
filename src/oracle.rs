use crate::ast_graph::{
    detect_language_for_ext, is_brace_language_source, is_noisy_dir, is_test_file_path,
    mask_brace_line, mask_python_line, read_bounded_source_file, sanitize_repo_relative_path,
    scan_python_signature_colon, strip_python_string_prefix, update_triple_quote_state,
};
use crate::models::{
    ConcurrencyRiskHint, ConfigAuditReport, EnvKeyUsage, FunctionSpan, IntegrationSliceHint,
    SiblingGuardHint, SpecSnippet, UnverifiedLiteral,
};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use walkdir::WalkDir;

const MAX_SPEC_SNIPPETS: usize = 6;
const MAX_SPEC_CHARS: usize = 240;

const ALLOWED_ENV_TEMPLATES: &[&str] = &[".env.example", ".env.sample", ".env.template"];

const OS_BUILTIN_ENV_KEYS: &[&str] = &[
    "PATH",
    "HOME",
    "USERPROFILE",
    "TEMP",
    "TMP",
    "TMPDIR",
    "PWD",
    "SHELL",
    "TERM",
    "LANG",
    "LC_ALL",
    "CI",
    "GITHUB_ACTIONS",
    "RUST_BACKTRACE",
    "CARGO_MANIFEST_DIR",
    "PYTHONPATH",
    "GOPATH",
    "GOCACHE",
];

fn truncate_chars(input: &str, max_chars: usize) -> String {
    let collapsed = input.split_whitespace().collect::<Vec<_>>().join(" ");
    let total_chars = collapsed.chars().count();
    if total_chars <= max_chars {
        return collapsed;
    }
    if max_chars <= 3 {
        return collapsed.chars().take(max_chars).collect();
    }
    let keep = max_chars - 3;
    let mut out: String = collapsed.chars().take(keep).collect();
    out.push_str("...");
    out
}

/// Returns true for `.env` or `.env.<env>` secret files while allowing `.env.example`,
/// `.env.sample`, and `.env.template`.
pub fn is_secret_env_file(rel_path: &str) -> bool {
    let norm = rel_path.trim().replace('\\', "/");
    let fname = norm
        .rsplit('/')
        .next()
        .unwrap_or(&norm)
        .to_ascii_lowercase();
    if ALLOWED_ENV_TEMPLATES.contains(&fname.as_str()) {
        return false;
    }
    fname == ".env" || fname.starts_with(".env.")
}

fn is_safe_spec_doc_file(
    repo_path: &Path,
    abs_p: &Path,
    rel_path: &str,
    resolved_source_rel: &str,
) -> bool {
    if is_secret_env_file(rel_path) {
        return false;
    }
    let norm_rel = rel_path.trim().replace('\\', "/");
    let norm_src = resolved_source_rel.trim().replace('\\', "/");
    if !norm_src.is_empty() && norm_rel.eq_ignore_ascii_case(&norm_src) {
        return false;
    }
    if abs_p
        .extension()
        .and_then(|s| s.to_str())
        .and_then(detect_language_for_ext)
        .is_some()
    {
        return false;
    }
    let Ok(canon_p) = abs_p.canonicalize() else {
        return false;
    };
    if let Ok(canon_repo) = repo_path.canonicalize()
        && !canon_p.starts_with(&canon_repo)
    {
        return false;
    }
    if let Some(canon_fname) = canon_p.file_name().and_then(|s| s.to_str())
        && is_secret_env_file(canon_fname)
    {
        return false;
    }
    canon_p.is_file()
}

fn push_unique_spec(
    out: &mut Vec<SpecSnippet>,
    seen: &mut HashSet<String>,
    source: &str,
    location: String,
    raw_excerpt: &str,
) {
    if out.len() >= MAX_SPEC_SNIPPETS {
        return;
    }
    let excerpt = truncate_chars(raw_excerpt.trim(), MAX_SPEC_CHARS);
    if excerpt.is_empty() {
        return;
    }
    let dedup_key = format!("{}|{}", source, excerpt.to_ascii_lowercase());
    if seen.insert(dedup_key) {
        out.push(SpecSnippet {
            source: source.to_string(),
            location,
            excerpt,
        });
    }
}

fn extract_symbol_doc_excerpt(content: &str, short_sym: &str) -> Option<String> {
    if short_sym.len() < 2 {
        return None;
    }
    let pattern = format!(r"\b{}\b", regex::escape(short_sym));
    let Ok(sym_re) = Regex::new(&pattern) else {
        return None;
    };
    let lines: Vec<&str> = content.lines().collect();
    for (idx, &line) in lines.iter().enumerate() {
        if sym_re.is_match(line) {
            let mut parts = vec![line.trim()];
            let mut taken_extra = 0usize;
            for &next_line in lines.iter().skip(idx + 1) {
                let t = next_line.trim();
                if t.is_empty() {
                    if taken_extra > 0 {
                        break;
                    }
                    continue;
                }
                if t.starts_with('#') && taken_extra > 0 {
                    break;
                }
                parts.push(t);
                taken_extra += 1;
                if taken_extra >= 2 {
                    break;
                }
            }
            return Some(parts.join(" "));
        }
    }
    None
}

fn is_candidate_repo_doc(rel_path: &str) -> bool {
    if is_secret_env_file(rel_path) {
        return false;
    }
    let norm = rel_path.replace('\\', "/");
    let lower = norm.to_ascii_lowercase();
    let fname = lower.rsplit('/').next().unwrap_or(&lower);
    let is_doc_ext = fname.ends_with(".md")
        || fname.ends_with(".txt")
        || fname.ends_with(".rst")
        || fname.ends_with(".yaml")
        || fname.ends_with(".yml")
        || fname.ends_with(".json");
    if !is_doc_ext {
        return false;
    }
    if !lower.contains('/') {
        return fname.starts_with("readme")
            || fname.starts_with("spec")
            || fname.starts_with("prd")
            || fname.starts_with("openapi.")
            || fname.starts_with("swagger.")
            || fname.starts_with("requirements")
            || fname.starts_with("architecture");
    }
    lower.starts_with("docs/")
        || lower.starts_with("doc/")
        || lower.starts_with("specs/")
        || lower.starts_with("spec/")
}

/// Discovers specification snippets from up to 5 independent sources outside the function body:
/// `explicit_spec` (`--spec`), `docstring`, `repo_doc` (`README`/`SPEC`/`PRD`/`docs`/`openapi`),
/// `git_commit` (`git log`), and `caller_usage`.
pub fn discover_target_specs(
    repo_path: &Path,
    resolved_rel: &str,
    short_sym: &str,
    docstring: &str,
    explicit_spec: Option<&str>,
    caller_snippets: &[(String, String, String)],
) -> Vec<SpecSnippet> {
    let mut specs = Vec::new();
    let mut seen = HashSet::new();

    // 1. Explicit --spec (inline text or safe repo-relative specification document)
    if let Some(raw_spec) = explicit_spec {
        let trimmed = raw_spec.trim();
        if !trimmed.is_empty() && !is_secret_env_file(trimmed) {
            let mut loaded_from_file = false;
            let mut blocked_file_target = false;
            if let Some(safe_rel) = sanitize_repo_relative_path(repo_path, trimmed) {
                let abs_p = repo_path.join(&safe_rel);
                if fs::symlink_metadata(&abs_p).is_ok() {
                    if is_safe_spec_doc_file(repo_path, &abs_p, &safe_rel, resolved_rel)
                        && let Some(file_txt) = read_bounded_source_file(&abs_p)
                    {
                        let excerpt = extract_symbol_doc_excerpt(&file_txt, short_sym)
                            .unwrap_or_else(|| truncate_chars(&file_txt, MAX_SPEC_CHARS));
                        push_unique_spec(&mut specs, &mut seen, "explicit_spec", safe_rel, &excerpt);
                        loaded_from_file = true;
                    } else {
                        blocked_file_target = true;
                    }
                }
            }
            if !loaded_from_file && !blocked_file_target {
                push_unique_spec(
                    &mut specs,
                    &mut seen,
                    "explicit_spec",
                    "--spec".to_string(),
                    trimmed,
                );
            }
        }
    }

    // 2. Function docstring (Python triple-quoted or brace-language doc comment)
    if !docstring.trim().is_empty() {
        push_unique_spec(
            &mut specs,
            &mut seen,
            "docstring",
            resolved_rel.to_string(),
            docstring,
        );
    }

    // 3. Repository documentation (`README*`, `SPEC*`, `PRD*`, `docs/**`, `openapi.*`)
    if short_sym.len() >= 2 && specs.len() < MAX_SPEC_SNIPPETS {
        let mut doc_files = Vec::new();
        let walker = WalkDir::new(repo_path)
            .max_depth(4)
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
            if entry.file_type().is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                if is_candidate_repo_doc(&rel) {
                    doc_files.push((rel, entry.path().to_path_buf()));
                    if doc_files.len() >= 40 {
                        break;
                    }
                }
            }
        }
        doc_files.sort_by(|a, b| a.0.cmp(&b.0));

        let mut repo_doc_added = 0usize;
        for (rel, abs_p) in doc_files {
            if specs.len() >= MAX_SPEC_SNIPPETS || repo_doc_added >= 2 {
                break;
            }
            if !is_safe_spec_doc_file(repo_path, &abs_p, &rel, resolved_rel) {
                continue;
            }
            let Some(txt) = read_bounded_source_file(&abs_p) else {
                continue;
            };
            if !txt.contains(short_sym) {
                continue;
            }
            if let Some(excerpt) = extract_symbol_doc_excerpt(&txt, short_sym) {
                push_unique_spec(&mut specs, &mut seen, "repo_doc", rel, &excerpt);
                repo_doc_added += 1;
            }
        }
    }

    // 4. Git commit history for the target source file (only when `.git` exists)
    if specs.len() < MAX_SPEC_SNIPPETS
        && !resolved_rel.is_empty()
        && repo_path.join(".git").exists()
    {
        let git_out = Command::new("git")
            .args([
                "log",
                "-n",
                "3",
                "--pretty=format:%h|%s",
                "--",
                resolved_rel,
            ])
            .current_dir(repo_path)
            .output();
        if let Ok(out) = git_out
            && out.status.success()
        {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let mut commit_added = 0usize;
            for line in stdout.lines() {
                if specs.len() >= MAX_SPEC_SNIPPETS || commit_added >= 2 {
                    break;
                }
                let mut parts = line.splitn(2, '|');
                let hash = parts.next().unwrap_or("").trim();
                let subject = parts.next().unwrap_or("").trim();
                if !hash.is_empty() && !subject.is_empty() {
                    push_unique_spec(
                        &mut specs,
                        &mut seen,
                        "git_commit",
                        format!("git:{}", hash),
                        subject,
                    );
                    commit_added += 1;
                }
            }
        }
    }

    // 5. Production caller usage excerpts
    for (caller_id, caller_file, excerpt) in caller_snippets.iter().take(2) {
        if specs.len() >= MAX_SPEC_SNIPPETS {
            break;
        }
        push_unique_spec(
            &mut specs,
            &mut seen,
            "caller_usage",
            format!("{} ({})", caller_id, caller_file),
            excerpt,
        );
    }

    specs
}

#[derive(Debug, Clone)]
struct ParsedParam {
    name: String,
    type_hint: String,
}

fn strip_leading_visibility_and_attrs(mut rest: &str) -> &str {
    loop {
        let r = rest.trim_start();
        let paren_prefix = if r.starts_with("pub(") {
            Some(3usize)
        } else if r.starts_with("__attribute__(") {
            Some("__attribute__".len())
        } else if r.starts_with("__declspec(") {
            Some("__declspec".len())
        } else {
            None
        };
        if let Some(open_off) = paren_prefix {
            let mut depth = 0i32;
            let mut end_off = None;
            for (i, ch) in r[open_off..].char_indices() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        end_off = Some(open_off + i + 1);
                        break;
                    }
                }
            }
            if let Some(e) = end_off {
                rest = &r[e..];
                continue;
            }
        }
        return r;
    }
}

fn find_top_level_assign_eq(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut depth = 0i32;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' | '[' | '<' | '{' => depth += 1,
            ')' | ']' | '>' | '}' => depth = (depth - 1).max(0),
            '=' if depth == 0 => {
                let prev = if i > 0 { bytes.get(i - 1).copied() } else { None };
                let next = bytes.get(i + 1).copied();
                if matches!(prev, Some(b'=' | b'!' | b'<' | b'>'))
                    || matches!(next, Some(b'=' | b'>'))
                {
                    continue;
                }
                return Some(i);
            }
            _ => {}
        }
    }
    None
}

fn extract_raw_param_list(signature: &str, lang: &str) -> Option<String> {
    let trimmed = strip_leading_visibility_and_attrs(signature.trim());
    if trimmed.is_empty() {
        return None;
    }

    // For Go receiver methods (`func (s *Service) Method(a int)`), skip the receiver group.
    let search_slice = if lang == "go" && trimmed.starts_with("func") {
        let after_func = trimmed.strip_prefix("func").unwrap_or(trimmed).trim_start();
        if after_func.starts_with('(') {
            let mut depth = 0i32;
            let mut end_recv = None;
            for (i, ch) in after_func.char_indices() {
                if ch == '(' {
                    depth += 1;
                } else if ch == ')' {
                    depth -= 1;
                    if depth == 0 {
                        end_recv = Some(i + 1);
                        break;
                    }
                }
            }
            end_recv.map(|pos| &after_func[pos..]).unwrap_or(after_func)
        } else {
            after_func
        }
    } else if matches!(lang, "javascript" | "typescript")
        && trimmed.contains("=>")
        && !trimmed.starts_with("function ")
        && !trimmed.starts_with("export function ")
        && !trimmed.starts_with("async function ")
        && !trimmed.starts_with("export async function ")
    {
        if let Some(eq_idx) = find_top_level_assign_eq(trimmed) {
            trimmed[eq_idx + 1..].trim_start()
        } else {
            trimmed
        }
    } else {
        trimmed
    };

    let mut angle_depth = 0i32;
    let mut open_idx = None;
    for (i, ch) in search_slice.char_indices() {
        match ch {
            '<' => angle_depth += 1,
            '>' => angle_depth = (angle_depth - 1).max(0),
            '(' if angle_depth == 0 => {
                open_idx = Some(i);
                break;
            }
            _ => {}
        }
    }

    if let Some(open) = open_idx {
        let mut depth = 0i32;
        for (rel_i, ch) in search_slice[open..].char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
                if depth == 0 {
                    return Some(search_slice[open + 1..open + rel_i].trim().to_string());
                }
            }
        }
    }

    // Single-arg JS/TS arrow function without parens: `const fn = x => ...`
    if matches!(lang, "javascript" | "typescript")
        && let Some(arrow_pos) = search_slice.find("=>")
    {
        let lhs = &search_slice[..arrow_pos];
        let after_eq = lhs
            .rfind('=')
            .map(|eq| &lhs[eq + 1..])
            .unwrap_or(lhs);
        let single = after_eq
            .trim()
            .trim_start_matches("async")
            .trim();
        if !single.is_empty() {
            return Some(single.to_string());
        }
    }

    None
}

fn split_top_level_commas(param_str: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in param_str.char_indices() {
        match ch {
            '(' | '[' | '<' | '{' => depth += 1,
            ')' | ']' | '>' | '}' => depth = (depth - 1).max(0),
            ',' if depth == 0 => {
                let seg = param_str[start..i].trim();
                if !seg.is_empty() {
                    out.push(seg.to_string());
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    let tail = param_str[start..].trim();
    if !tail.is_empty() {
        out.push(tail.to_string());
    }
    out
}

fn parse_signature_params(signature: &str, lang: &str) -> Vec<ParsedParam> {
    let Some(raw_list) = extract_raw_param_list(signature, lang) else {
        return Vec::new();
    };
    if raw_list.is_empty() || raw_list == "void" {
        return Vec::new();
    }

    let segments = split_top_level_commas(&raw_list);
    let mut staged: Vec<(String, String, bool, String)> = Vec::new();

    for seg in segments {
        let no_default = find_top_level_assign_eq(&seg)
            .map(|eq| &seg[..eq])
            .unwrap_or(&seg)
            .trim();
        if no_default.is_empty() || no_default == "/" || no_default == "*" || no_default == "..." {
            continue;
        }
        if matches!(
            no_default,
            "self" | "&self" | "&mut self" | "mut self" | "cls" | "this"
        ) || no_default.starts_with("self:")
        {
            continue;
        }

        let (raw_name, raw_type) = if matches!(lang, "python" | "rust" | "typescript" | "javascript")
        {
            if let Some((lhs, rhs)) = no_default.split_once(':') {
                (lhs.trim(), rhs.trim())
            } else {
                (no_default, "")
            }
        } else if lang == "go" {
            let first = no_default.split_whitespace().next().unwrap_or("");
            (first, no_default[first.len()..].trim())
        } else {
            // C, C++, Java, C#: `<modifiers/type> <name>` (with optional `[]` array suffix)
            let stripped_arr = no_default.trim_end_matches("[]").trim();
            let last_tok_start = stripped_arr
                .rfind(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let name_part = &stripped_arr[last_tok_start..];
            let type_part = &no_default[..last_tok_start];
            (name_part.trim(), type_part.trim())
        };

        let mut clean_name = raw_name
            .trim()
            .trim_start_matches("mut ")
            .trim_start_matches("ref ")
            .trim_start_matches("...")
            .trim_start_matches('*')
            .trim_start_matches('&')
            .trim();
        let mut optional_marker = false;
        if let Some(stripped_q) = clean_name.strip_suffix('?') {
            clean_name = stripped_q.trim();
            optional_marker = true;
        }
        if clean_name.is_empty()
            || matches!(clean_name, "self" | "cls" | "this" | "void")
            || !clean_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }

        staged.push((
            clean_name.to_string(),
            raw_type.trim().to_string(),
            optional_marker,
            seg,
        ));
    }

    // Go grouped parameters (`a, b int` or `sender, recipient *Account`) inherit the trailing type.
    if lang == "go" {
        let mut inherited_type = String::new();
        for (_, rtype, _, _) in staged.iter_mut().rev() {
            if rtype.is_empty() {
                if !inherited_type.is_empty() {
                    *rtype = inherited_type.clone();
                }
            } else {
                inherited_type = rtype.clone();
            }
        }
    }

    let mut params = Vec::with_capacity(staged.len());
    for (clean_name, raw_type, optional_marker, seg) in staged {
        let mut hint = raw_type.to_ascii_lowercase();
        if optional_marker {
            hint.push('?');
        }
        if seg.contains("= None")
            || seg.contains("=None")
            || seg.contains("= null")
            || seg.contains("=null")
            || seg.contains("= nil")
        {
            hint.push_str(" nullable");
        }
        params.push(ParsedParam {
            name: clean_name,
            type_hint: hint,
        });
    }

    params
}

fn normalize_core_type(type_hint: &str) -> String {
    let mut s = type_hint.trim();
    if let Some(inner) = s
        .strip_prefix("option<")
        .and_then(|r| r.strip_suffix('>'))
        .or_else(|| {
            s.strip_prefix("optional[")
                .and_then(|r| r.strip_suffix(']'))
        })
    {
        s = inner.trim();
    }
    let cleaned = s
        .trim_end_matches("nullable")
        .trim()
        .trim_end_matches('?')
        .trim_start_matches('&')
        .trim();
    let mut tokens = Vec::new();
    for tok in cleaned.split_whitespace() {
        let t = tok.trim_matches(|c| matches!(c, '&' | '?' | '*'));
        if matches!(
            t,
            "" | "const" | "final" | "volatile" | "mut" | "in" | "out" | "ref" | "readonly"
                | "nullable"
        ) {
            continue;
        }
        tokens.push(t);
    }
    tokens.join(" ")
}

fn name_has_token(name_lower: &str, tokens: &[&str]) -> bool {
    if tokens.contains(&name_lower) {
        return true;
    }
    for part in name_lower.split('_') {
        if !part.is_empty() && tokens.contains(&part) {
            return true;
        }
    }
    for &tok in tokens {
        if tok.len() >= 4 && name_lower.ends_with(tok) {
            return true;
        }
    }
    false
}

/// Generates mandatory black-box domain & boundary probes solely from the function `signature`
/// across all 9 languages, without inspecting the function body.
pub fn infer_signature_invariants(signature: &str, lang: &str) -> Vec<String> {
    if extract_raw_param_list(signature, lang).is_none() {
        return Vec::new();
    }
    let params = parse_signature_params(signature, lang);
    if params.is_empty() {
        return vec![
            "Probe zero-argument call across initial/empty state and repeated idempotent invocations"
                .to_string(),
        ];
    }

    let entity_tokens = &[
        "user", "account", "actor", "caller", "client", "customer", "member", "admin", "owner",
        "session", "wallet", "order", "invoice", "merchant", "tenant",
    ];
    let nullable_ctx_tokens = &[
        "ctx", "context", "req", "request", "payload", "body", "event", "msg", "message", "node",
        "target", "config", "opts", "options",
    ];
    let numeric_tokens = &[
        "amount",
        "price",
        "cost",
        "fee",
        "tax",
        "discount",
        "rate",
        "ratio",
        "total",
        "subtotal",
        "balance",
        "qty",
        "quantity",
        "count",
        "num",
        "limit",
        "max",
        "min",
        "threshold",
        "size",
        "len",
        "length",
        "idx",
        "index",
        "offset",
        "age",
        "score",
        "timeout",
        "retries",
        "delta",
        "weight",
        "capacity",
        "budget",
        "credit",
        "debit",
    ];
    let collection_str_tokens = &[
        "name", "text", "str", "code", "token", "key", "secret", "query", "sql", "path", "url",
        "email", "items", "list", "arr", "array", "tags", "rows", "records", "lines", "buf",
        "buffer", "data", "headers", "params", "map", "dict", "entries", "ids", "keys", "values",
    ];

    let mut probes = Vec::new();
    for p in &params {
        if probes.len() >= 6 {
            break;
        }
        let n_low = p.name.to_ascii_lowercase();
        let t = p.type_hint.as_str();
        let core_t = normalize_core_type(t);
        let core_s = core_t.as_str();

        let is_entity = name_has_token(&n_low, entity_tokens);
        let is_nullable = t.contains("option<")
            || t.contains("optional[")
            || t.contains("nullable")
            || t.contains("null")
            || t.contains("undefined")
            || t.contains("none")
            || t.contains('?')
            || t.starts_with('*')
            || t.contains(" *")
            || t.ends_with('*')
            || name_has_token(&n_low, nullable_ctx_tokens);

        let is_bool_type = matches!(core_s, "bool" | "boolean")
            || n_low.starts_with("is_")
            || n_low.starts_with("has_")
            || n_low.starts_with("can_")
            || n_low.starts_with("allow_")
            || n_low.starts_with("enable");

        let is_unsigned = matches!(
            core_s,
            "usize"
                | "u8"
                | "u16"
                | "u32"
                | "u64"
                | "u128"
                | "uint"
                | "uint8"
                | "uint16"
                | "uint32"
                | "uint64"
                | "uint8_t"
                | "uint16_t"
                | "uint32_t"
                | "uint64_t"
                | "uintptr"
                | "uintptr_t"
                | "size_t"
                | "ulong"
                | "ushort"
                | "byte"
        ) || core_s.starts_with("unsigned ");

        let is_numeric_type = is_unsigned
            || matches!(
                core_s,
                "int"
                    | "integer"
                    | "float"
                    | "float32"
                    | "float64"
                    | "double"
                    | "decimal"
                    | "bigdecimal"
                    | "biginteger"
                    | "number"
                    | "i8"
                    | "i16"
                    | "i32"
                    | "i64"
                    | "i128"
                    | "isize"
                    | "f32"
                    | "f64"
                    | "int8"
                    | "int16"
                    | "int32"
                    | "int64"
                    | "int8_t"
                    | "int16_t"
                    | "int32_t"
                    | "int64_t"
                    | "intptr_t"
                    | "ptrdiff_t"
                    | "long"
                    | "short"
                    | "sbyte"
                    | "ssize_t"
            );
        let is_numeric = is_numeric_type || name_has_token(&n_low, numeric_tokens);

        let is_collection_or_str = t.contains("str")
            || t.contains("string")
            || t.contains("vec<")
            || t.contains("list")
            || t.contains("dict")
            || t.contains("map")
            || t.contains("set")
            || t.contains("slice")
            || t.contains("[]")
            || t.contains("bytes")
            || t.contains("char*")
            || t.contains("char *")
            || name_has_token(&n_low, collection_str_tokens);

        let probe = if is_entity {
            format!(
                "Probe entity parameter '{}' with None/null and restricted domain states (e.g. locked, inactive, unverified, unauthorized)",
                p.name
            )
        } else if is_numeric {
            if is_unsigned && is_nullable {
                format!(
                    "Probe nullable unsigned numeric parameter '{}' with None/null/nil, 0, 1, and upper-boundary values",
                    p.name
                )
            } else if is_unsigned {
                format!(
                    "Probe unsigned numeric parameter '{}' with 0, 1, and upper-boundary values",
                    p.name
                )
            } else if is_nullable {
                format!(
                    "Probe nullable numeric parameter '{}' with None/null/nil, 0, negative (-1), and exact threshold boundary values",
                    p.name
                )
            } else {
                format!(
                    "Probe numeric parameter '{}' with 0, negative (-1), and exact threshold boundary values",
                    p.name
                )
            }
        } else if is_collection_or_str && is_nullable {
            format!(
                "Probe nullable string/collection parameter '{}' with None/null/nil and empty value (\"\", [], {{}}) edge cases",
                p.name
            )
        } else if is_collection_or_str {
            format!(
                "Probe string/collection parameter '{}' with empty value (\"\", [], {{}}) and single-element edge cases",
                p.name
            )
        } else if is_nullable {
            format!(
                "Probe nullable/context parameter '{}' with None/null/nil and missing-field inputs",
                p.name
            )
        } else if is_bool_type {
            format!(
                "Probe boolean flag '{}' across both True and False branches",
                p.name
            )
        } else {
            format!(
                "Probe parameter '{}' with None/null, zero (0), negative (-1), and empty (\"\", []) boundary inputs",
                p.name
            )
        };
        probes.push(probe);
    }

    probes.truncate(6);
    probes
}

fn guard_helper_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"\b((?:self\.|this\.)?(?:(?:validate|ensure|check|verify|authorize|require|guard)_[A-Za-z0-9_]+|(?:validate|ensure|check|verify|authorize|require)[A-Z][A-Za-z0-9_]*))\s*\(\s*([A-Za-z_][A-Za-z0-9_]*)?",
        )
        .ok()
    })
    .as_ref()
}

fn security_attr_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"((?:not\s+|!)?[A-Za-z_][A-Za-z0-9_]*\.(?:is_locked|locked|is_frozen|frozen|is_suspended|is_banned|is_disabled|is_deleted|is_expired|is_closed|is_active|isActive|is_verified|isVerified|is_authenticated|isAuthenticated|is_authorized|isAuthorized|is_admin|isAdmin|has_permission|hasPermission)\b(?:\s*\(\s*\))?)",
        )
        .ok()
    })
    .as_ref()
}

fn extract_masked_body_lines(fn_source: &str, is_brace: bool) -> Vec<(usize, String, String)> {
    let lines: Vec<&str> = fn_source.lines().collect();
    let mut out = Vec::new();
    let mut in_signature = false;
    let mut sig_paren_depth = 0i32;
    let mut sig_seen_open = false;
    let mut in_triple: Option<&str> = None;
    let mut in_block_comment = false;

    for (idx, &raw_line) in lines.iter().enumerate() {
        if !is_brace {
            let was_triple = in_triple.is_some();
            in_triple = update_triple_quote_state(raw_line, in_triple);
            if was_triple || in_triple.is_some() {
                continue;
            }
        }

        let mut masked = if is_brace {
            mask_brace_line(raw_line, &mut in_block_comment)
        } else {
            mask_python_line(raw_line)
        };
        let trimmed = raw_line.trim();

        if idx == 0 {
            in_signature = true;
            sig_paren_depth = 0;
            sig_seen_open = false;
        }

        if in_signature {
            if !is_brace {
                if let Some(colon_idx) = scan_python_signature_colon(
                    &masked,
                    &mut sig_paren_depth,
                    &mut sig_seen_open,
                ) {
                    in_signature = false;
                    let inline_body = masked[colon_idx + 1..].trim();
                    if inline_body.is_empty() {
                        continue;
                    }
                    masked = format!(
                        "{}{}",
                        " ".repeat(colon_idx + 1),
                        &masked[colon_idx + 1..]
                    );
                } else {
                    continue;
                }
            } else {
                for ch in masked.chars() {
                    if ch == '(' {
                        sig_paren_depth += 1;
                        sig_seen_open = true;
                    } else if ch == ')' {
                        sig_paren_depth -= 1;
                    }
                }
                if sig_seen_open && sig_paren_depth <= 0 {
                    if let Some(brace_idx) = masked.find('{') {
                        in_signature = false;
                        let inline_body =
                            masked[brace_idx + 1..].trim().trim_end_matches('}').trim();
                        if inline_body.is_empty() {
                            continue;
                        }
                        masked = format!(
                            "{}{}",
                            " ".repeat(brace_idx + 1),
                            &masked[brace_idx + 1..]
                        );
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
        }

        let stripped_prefix = strip_python_string_prefix(trimmed);
        if masked.trim().is_empty()
            || (!is_brace
                && (stripped_prefix.starts_with("\"\"\"")
                    || stripped_prefix.starts_with("'''")))
        {
            continue;
        }

        out.push((idx, raw_line.to_string(), masked));
    }

    out
}

#[derive(Debug, Clone)]
struct ExtractedGuard {
    pattern: String,
    token_probe: String,
    param_ref: Option<String>,
    is_decorator: bool,
}

fn is_early_exit_line(masked_trim: &str) -> bool {
    masked_trim.starts_with("raise ")
        || masked_trim == "raise"
        || masked_trim.starts_with("return ")
        || masked_trim == "return"
        || masked_trim.starts_with("return;")
        || masked_trim.starts_with("throw ")
        || masked_trim.contains("panic!(")
        || masked_trim.contains("bail!(")
        || masked_trim.contains(" Err(")
        || masked_trim.starts_with("return Err(")
}

fn strip_if_keyword(line_trim: &str) -> Option<&str> {
    let s = line_trim.trim_start_matches('}').trim_start();
    for kw in ["else if", "elif", "if"] {
        if let Some(rest) = s.strip_prefix(kw)
            && (rest.starts_with(' ') || rest.starts_with('('))
        {
            return Some(rest.trim_start());
        }
    }
    None
}

fn extract_guard_decorators(file_lines: &[&str], start_line_1based: usize) -> Vec<ExtractedGuard> {
    if start_line_1based <= 1 || file_lines.is_empty() {
        return Vec::new();
    }
    let sig_idx = (start_line_1based - 1).min(file_lines.len());
    let min_idx = sig_idx.saturating_sub(12);
    let mut cursor = sig_idx;
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    while cursor > min_idx {
        let raw_t = file_lines[cursor - 1].trim();
        if raw_t.is_empty() {
            break;
        }
        // Allow single-line comments immediately between decorators and signature
        if (raw_t.starts_with('#') && !raw_t.starts_with("#[")) || raw_t.starts_with("//") {
            cursor -= 1;
            continue;
        }

        let no_comment = if raw_t.starts_with('@') {
            raw_t
                .split("  #")
                .next()
                .unwrap_or(raw_t)
                .split(" //")
                .next()
                .unwrap_or(raw_t)
                .trim()
        } else {
            raw_t.split(" //").next().unwrap_or(raw_t).trim()
        };

        let is_attr = no_comment.starts_with('@')
            || no_comment.starts_with("#[")
            || (no_comment.starts_with('[')
                && no_comment.ends_with(']')
                && !no_comment.starts_with("/*"));
        if !is_attr {
            break;
        }

        let lower = no_comment.to_ascii_lowercase();
        let is_structural = lower.starts_with("@staticmethod")
            || lower.starts_with("@classmethod")
            || lower.starts_with("@property")
            || lower.starts_with("@override")
            || lower.starts_with("@deprecated")
            || lower.starts_with("@dataclass")
            || lower.starts_with("@overload")
            || lower.starts_with("#[derive")
            || lower.starts_with("#[allow")
            || lower.starts_with("#[warn")
            || lower.starts_with("#[cfg")
            || lower.starts_with("#[inline")
            || lower.starts_with("#[serde")
            || lower.starts_with("#[must_use")
            || lower.starts_with("[serializable")
            || lower.starts_with("[obsolete");

        let is_security_or_guard = !is_structural
            && [
                "login_required",
                "auth",
                "permission",
                "role",
                "admin",
                "secured",
                "guard",
                "verify",
                "verified",
                "validate",
                "csrf",
                "antiforgery",
                "anti_forgery",
                "kyc",
                "rate_limit",
                "ratelimit",
                "throttle",
                "require_",
                "requires_",
            ]
            .iter()
            .any(|kw| lower.contains(kw));

        if is_security_or_guard {
            let stripped = no_comment
                .trim_start_matches("#[")
                .trim_start_matches('[')
                .trim_start_matches('@')
                .trim();
            let core_name: String = stripped
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '.')
                .collect();
            let pattern = no_comment.split_whitespace().collect::<Vec<_>>().join(" ");
            if !core_name.is_empty() && seen.insert(pattern.clone()) {
                out.push(ExtractedGuard {
                    pattern,
                    token_probe: core_name,
                    param_ref: None,
                    is_decorator: true,
                });
            }
        }
        cursor -= 1;
    }

    out
}

fn extract_guards_from_function(fn_span: &FunctionSpan, is_brace: bool) -> Vec<ExtractedGuard> {
    let body_lines = extract_masked_body_lines(&fn_span.source, is_brace);
    let mut guards = Vec::new();
    let mut seen_patterns = HashSet::new();

    for (b_idx, (_line_offset, raw_line, masked_line)) in body_lines.iter().enumerate() {
        let masked_trim = masked_line.trim();

        // 1. Guard helper function calls (`ensure_kyc_verified(user)`, `validate_token(req)`, etc.)
        if let Some(re) = guard_helper_regex() {
            for cap in re.captures_iter(masked_trim) {
                let Some(callee_m) = cap.get(1) else {
                    continue;
                };
                let callee = callee_m.as_str();
                let short_callee = callee
                    .strip_prefix("self.")
                    .or_else(|| callee.strip_prefix("this."))
                    .unwrap_or(callee);
                let fn_short = fn_span
                    .name
                    .split('.')
                    .next_back()
                    .unwrap_or(&fn_span.name);
                if short_callee == fn_short {
                    continue;
                }
                let arg_opt = cap
                    .get(2)
                    .map(|m| m.as_str().trim().to_string())
                    .filter(|s| {
                        !s.is_empty()
                            && !matches!(
                                s.as_str(),
                                "self" | "this" | "true" | "false" | "True" | "False"
                            )
                    });
                let pattern = if let Some(ref a) = arg_opt {
                    format!("{}({})", callee, a)
                } else {
                    format!("{}()", callee)
                };
                if seen_patterns.insert(pattern.clone()) {
                    guards.push(ExtractedGuard {
                        pattern,
                        token_probe: short_callee.to_string(),
                        param_ref: arg_opt,
                        is_decorator: false,
                    });
                }
            }
        }

        // 2. State / attribute / permission / null / early-exit `if` / `elif` / `else if` guards
        let Some(after_if_masked) = strip_if_keyword(masked_trim) else {
            continue;
        };

        if let Some(attr_re) = security_attr_regex() {
            for cap in attr_re.captures_iter(masked_trim) {
                let Some(m) = cap.get(1) else {
                    continue;
                };
                let expr = m.as_str().split_whitespace().collect::<Vec<_>>().join(" ");
                let base_ident = expr
                    .trim_start_matches("not ")
                    .trim_start_matches('!')
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .trim();
                let attr_name = expr
                    .split('.')
                    .nth(1)
                    .unwrap_or(&expr)
                    .trim_end_matches("()")
                    .trim()
                    .to_string();
                let param_ref =
                    if !base_ident.is_empty() && !matches!(base_ident, "self" | "this") {
                        Some(base_ident.to_string())
                    } else {
                        None
                    };
                if seen_patterns.insert(expr.clone()) {
                    guards.push(ExtractedGuard {
                        pattern: expr,
                        token_probe: attr_name,
                        param_ref,
                        is_decorator: false,
                    });
                }
            }
        }

        // Check if this `if` / `elif` / `else if` branch is an early-exit guard
        let next_is_exit = body_lines
            .get(b_idx + 1)
            .map(|(_, _, next_masked)| is_early_exit_line(next_masked.trim()))
            .unwrap_or(false);
        let inline_after_cond = if !is_brace {
            after_if_masked
                .split_once(':')
                .map(|(_, rhs)| is_early_exit_line(rhs.trim()))
                .unwrap_or(false)
        } else {
            after_if_masked
                .split_once('{')
                .map(|(_, rhs)| is_early_exit_line(rhs.trim().trim_end_matches('}').trim()))
                .unwrap_or(false)
        };

        if next_is_exit || inline_after_cond {
            let clean_no_comment = strip_comments_keep_strings(raw_line, is_brace);
            let after_if_clean =
                strip_if_keyword(clean_no_comment.trim()).unwrap_or(after_if_masked);
            let cond_slice = if !is_brace {
                after_if_clean
                    .rsplit_once(':')
                    .map(|(lhs, _)| lhs)
                    .unwrap_or("")
            } else {
                let s = after_if_clean
                    .split('{')
                    .next()
                    .unwrap_or(after_if_clean)
                    .trim();
                s.strip_prefix('(')
                    .and_then(|inner| inner.strip_suffix(')'))
                    .unwrap_or(s)
            };

            let masked_has_guard_signal = after_if_masked.contains(" is None")
                || after_if_masked.contains(" == None")
                || after_if_masked.contains(" == nil")
                || after_if_masked.contains(" == null")
                || after_if_masked.contains(" === null")
                || after_if_masked.contains(" == nullptr")
                || after_if_masked.contains(".is_none()")
                || after_if_masked.contains("permission")
                || after_if_masked.contains("role")
                || after_if_masked.contains(" <= 0")
                || after_if_masked.contains(" < 0")
                || after_if_masked.contains('!')
                || after_if_masked.contains("not ");
            if !masked_has_guard_signal {
                continue;
            }

            for sub_clause in cond_slice.split(" or ").flat_map(|p| p.split(" || ")) {
                let clause = sub_clause.trim();
                if clause.is_empty() || clause.len() > 80 {
                    continue;
                }
                let is_null_or_perm_or_bound = clause.contains(" is None")
                    || clause.contains(" == None")
                    || clause.contains(" == nil")
                    || clause.contains(" == null")
                    || clause.contains(" === null")
                    || clause.contains(" == nullptr")
                    || clause.contains(".is_none()")
                    || clause.contains("permission")
                    || clause.contains("role")
                    || clause.contains(" <= 0")
                    || clause.contains(" < 0")
                    || clause.starts_with('!')
                    || clause.starts_with("not ");
                if !is_null_or_perm_or_bound {
                    continue;
                }
                let lead_ident: String = clause
                    .trim_start_matches("not ")
                    .trim_start_matches('!')
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                let param_ref = if !lead_ident.is_empty()
                    && !matches!(lead_ident.as_str(), "self" | "this" | "len")
                {
                    Some(lead_ident)
                } else {
                    None
                };
                if seen_patterns.insert(clause.to_string()) {
                    guards.push(ExtractedGuard {
                        pattern: clause.to_string(),
                        token_probe: clause.to_string(),
                        param_ref,
                        is_decorator: false,
                    });
                }
            }
        }
    }

    guards
}

/// Detects guard checks (`user.is_locked`, `ensure_kyc_verified(user)`, decorators, null/permission/early-exit
/// preconditions) present across sibling functions in the same module that are missing from `target_fn`.
pub fn detect_missing_sibling_guards(
    target_fn: &FunctionSpan,
    sibling_fns: &[FunctionSpan],
    lang: &str,
) -> Vec<SiblingGuardHint> {
    detect_missing_sibling_guards_in_file(target_fn, sibling_fns, lang, None)
}

/// File-aware sibling guard detector that also inspects decorators/annotations/attributes
/// immediately preceding function definitions in `file_content`.
pub fn detect_missing_sibling_guards_in_file(
    target_fn: &FunctionSpan,
    sibling_fns: &[FunctionSpan],
    lang: &str,
    file_content: Option<&str>,
) -> Vec<SiblingGuardHint> {
    let is_brace = lang != "python" && is_brace_language_source(&target_fn.source);
    let target_params: Vec<String> = parse_signature_params(&target_fn.signature, lang)
        .into_iter()
        .map(|p| p.name)
        .collect();
    let target_first_param = target_params.first().cloned();

    let file_lines: Vec<&str> = file_content
        .map(|c| c.lines().collect())
        .unwrap_or_default();
    let target_decorators = extract_guard_decorators(&file_lines, target_fn.start);

    let target_body_masked = extract_masked_body_lines(&target_fn.source, is_brace)
        .into_iter()
        .map(|(_, _, m)| m)
        .collect::<Vec<_>>()
        .join("\n");
    let target_active_code = strip_comments_keep_strings(&target_fn.source, is_brace);

    struct GuardObservation {
        token_probe: String,
        is_decorator: bool,
        siblings: Vec<(String, bool)>,
    }

    let mut observed: HashMap<String, GuardObservation> = HashMap::new();

    for sib in sibling_fns {
        if sib.function_id == target_fn.function_id {
            continue;
        }
        let sib_short = sib.name.split('.').next_back().unwrap_or(&sib.name);
        if sib_short.starts_with("test_")
            || matches!(
                sib_short,
                "__init__" | "__repr__" | "__str__" | "new" | "default" | "fmt"
            )
        {
            continue;
        }
        let sib_params: Vec<String> = parse_signature_params(&sib.signature, lang)
            .into_iter()
            .map(|p| p.name)
            .collect();
        let shares_first_param = match (&target_first_param, sib_params.first()) {
            (Some(tp), Some(sp)) => tp == sp,
            _ => false,
        };

        let sib_is_brace = lang != "python" && is_brace_language_source(&sib.source);
        let mut sib_guards = extract_guards_from_function(sib, sib_is_brace);
        sib_guards.extend(extract_guard_decorators(&file_lines, sib.start));

        for g in sib_guards {
            if let Some(ref p_ref) = g.param_ref
                && !target_params.contains(p_ref)
            {
                continue;
            }
            let entry = observed.entry(g.pattern).or_insert_with(|| GuardObservation {
                token_probe: g.token_probe,
                is_decorator: g.is_decorator,
                siblings: Vec::new(),
            });
            if !entry.siblings.iter().any(|(id, _)| id == &sib.function_id) {
                entry
                    .siblings
                    .push((sib.function_id.clone(), shares_first_param));
            }
        }
    }

    let mut hints = Vec::new();
    for (pattern, mut obs) in observed {
        obs.siblings.sort_by(|a, b| a.0.cmp(&b.0));
        let count = obs.siblings.len();
        let any_shared_first = obs.siblings.iter().any(|(_, shared)| *shared);
        if count < 2 && !any_shared_first {
            continue;
        }

        // Check whether target_fn already enforces this guard in active (non-comment, non-string) code
        let already_guarded = if obs.is_decorator {
            target_decorators.iter().any(|d| {
                d.pattern == pattern
                    || (!obs.token_probe.is_empty() && d.token_probe == obs.token_probe)
            }) || (!obs.token_probe.is_empty() && target_body_masked.contains(&obs.token_probe))
        } else if pattern.contains('"') || pattern.contains('\'') {
            target_active_code.contains(&pattern)
        } else {
            target_body_masked.contains(&pattern)
                || (!obs.token_probe.is_empty() && target_body_masked.contains(&obs.token_probe))
        };
        if already_guarded {
            continue;
        }

        let observed_in_siblings: Vec<String> =
            obs.siblings.into_iter().map(|(id, _)| id).collect();
        let sib_list = observed_in_siblings.join(", ");
        hints.push(SiblingGuardHint {
            guard_pattern: pattern.clone(),
            observed_in_siblings,
            suggested_probe: format!(
                "Probe {} with input violating `{}` (guard enforced in {} but absent in target)",
                target_fn.function_id, pattern, sib_list
            ),
        });
    }

    hints.sort_by(|a, b| {
        b.observed_in_siblings
            .len()
            .cmp(&a.observed_in_siblings.len())
            .then_with(|| a.guard_pattern.cmp(&b.guard_pattern))
    });
    hints.truncate(6);
    hints
}

fn spec_number_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:^|[^A-Za-z0-9_])(-?(?:\d[\d,_]*(?:\.\d+)?|\.\d+)(?:[eE][+-]?\d+)?)\b\s*(%)?",
        )
        .ok()
    })
    .as_ref()
}

fn code_number_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:^|[^A-Za-z0-9_.)\]])((?:\d[\d_]*(?:\.\d[\d_]*)?|\.\d[\d_]*)(?:[eE][+-]?\d[\d_]*)?)(?:[uif](?:8|16|32|64|128|size)|[lLfFdDuU]+)?\b",
        )
        .ok()
    })
    .as_ref()
}

fn module_const_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^\s*(?:pub(?:\([^)]*\))?\s+|export\s+|private\s+|public\s+|internal\s+|static\s+|final\s+|readonly\s+|const\s+|constexpr\s+)*([A-Z][A-Z0-9_]{2,})\s*(?::[^=]+)?=\s*(-?(?:\d[\d_]*(?:\.\d[\d_]*)?|\.\d[\d_]*)(?:[eE][+-]?\d[\d_]*)?)\b",
        )
        .ok()
    })
    .as_ref()
}

fn parse_clean_f64(raw: &str) -> Option<f64> {
    let cleaned: String = raw.chars().filter(|&c| c != '_' && c != ',').collect();
    cleaned.parse::<f64>().ok()
}

fn values_match_with_percentage(code_val: f64, spec_val: f64) -> bool {
    if (code_val - spec_val).abs() < 1e-9 {
        return true;
    }
    // 0.15 in code <-> 15% or 15 in spec
    if code_val > 0.0 && code_val < 1.0 && ((code_val * 100.0) - spec_val).abs() < 1e-6 {
        return true;
    }
    // 15 in code <-> 0.15 in spec
    if spec_val > 0.0 && spec_val < 1.0 && ((spec_val * 100.0) - code_val).abs() < 1e-6 {
        return true;
    }
    false
}

fn is_unary_minus_before(masked: &str, num_start: usize) -> bool {
    let before = masked[..num_start].trim_end();
    let Some(without_minus) = before.strip_suffix('-') else {
        return false;
    };
    let prev = without_minus.trim_end();
    if prev.is_empty() {
        return true;
    }
    let Some(last_ch) = prev.chars().last() else {
        return true;
    };
    !(last_ch.is_ascii_alphanumeric() || matches!(last_ch, '_' | ')' | ']'))
}

/// Extracts hardcoded business numeric literals in `fn_source` that are not backed by any
/// `discovered_specs` entry (including percentage equivalence `0.15 <-> 15%`) or module-level
/// `UPPER_SNAKE_CASE` constant definition outside `fn_source`.
pub fn collect_unverified_literals(
    fn_source: &str,
    lang: &str,
    discovered_specs: &[SpecSnippet],
    file_content: &str,
    start_line: usize,
) -> Vec<UnverifiedLiteral> {
    let mut spec_numbers: Vec<f64> = Vec::new();
    if let Some(s_re) = spec_number_regex() {
        for snip in discovered_specs {
            for cap in s_re.captures_iter(&snip.excerpt) {
                if let Some(m) = cap.get(1)
                    && let Some(v) = parse_clean_f64(m.as_str())
                {
                    spec_numbers.push(v);
                }
            }
        }
    }

    let is_brace = lang != "python" && is_brace_language_source(fn_source);
    let base_line = start_line.max(1);
    let fn_line_count = fn_source.lines().count().max(1);
    let total_file_lines = file_content.lines().count().max(1);
    let is_whole_file = base_line == 1 && fn_line_count >= total_file_lines && total_file_lines > 1;
    let fn_end_line = base_line + fn_line_count - 1;

    let mut module_const_numbers: Vec<f64> = Vec::new();
    if let Some(c_re) = module_const_regex() {
        let mut in_triple: Option<&str> = None;
        let mut in_block_comment = false;
        for (line_idx, line) in file_content.lines().enumerate() {
            let line_no = line_idx + 1;
            if !is_brace {
                let was_triple = in_triple.is_some();
                in_triple = update_triple_quote_state(line, in_triple);
                if was_triple || in_triple.is_some() {
                    continue;
                }
            } else {
                let was_bc = in_block_comment;
                let _ = mask_brace_line(line, &mut in_block_comment);
                if was_bc {
                    continue;
                }
            }

            // Never allow local constants inside `target_fn` (or indented inner scopes) to verify themselves
            if !is_whole_file && line_no >= base_line && line_no <= fn_end_line {
                continue;
            }
            if (lang == "python" || is_whole_file)
                && (line.starts_with(' ') || line.starts_with('\t'))
            {
                continue;
            }

            if let Some(cap) = c_re.captures(line)
                && let Some(val_m) = cap.get(2)
                && let Some(v) = parse_clean_f64(val_m.as_str())
            {
                module_const_numbers.push(v);
            }
        }
    }

    let body_lines = extract_masked_body_lines(fn_source, is_brace);
    let Some(num_re) = code_number_regex() else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for (rel_idx, raw_line, masked_line) in body_lines {
        for cap in num_re.captures_iter(&masked_line) {
            let Some(m) = cap.get(1) else {
                continue;
            };
            let pos = m.start();

            let is_neg = is_unary_minus_before(&masked_line, pos);
            let raw_lit = if is_neg {
                format!("-{}", m.as_str())
            } else {
                m.as_str().to_string()
            };
            let Some(val) = parse_clean_f64(&raw_lit) else {
                continue;
            };

            // Skip structural constants (-1, 0, 1, 2, 0.0, 1.0, 2.0)
            if (val - (-1.0)).abs() < 1e-12
                || val.abs() < 1e-12
                || (val - 1.0).abs() < 1e-12
                || (val - 2.0).abs() < 1e-12
            {
                continue;
            }

            // Skip standard bit widths (8, 16, 32, 64) immediately after bitwise shifts `<<` / `>>`
            if matches!(raw_lit.as_str(), "8" | "16" | "32" | "64") {
                let prefix = masked_line[..pos].trim_end().trim_end_matches('(').trim_end();
                if prefix.ends_with("<<")
                    || prefix.ends_with(">>")
                    || prefix.ends_with("<<=")
                    || prefix.ends_with(">>=")
                {
                    continue;
                }
            }

            let verified_by_spec = spec_numbers
                .iter()
                .any(|&sv| values_match_with_percentage(val, sv));
            let verified_by_const = module_const_numbers
                .iter()
                .any(|&cv| values_match_with_percentage(val, cv));

            if !verified_by_spec && !verified_by_const {
                out.push(UnverifiedLiteral {
                    line: base_line + rel_idx,
                    literal: raw_lit,
                    code_line: raw_line.trim().to_string(),
                });
            }
        }
    }

    out
}

fn strip_comments_keep_strings(fn_source: &str, is_brace: bool) -> String {
    let mut out_lines = Vec::new();
    let mut in_triple: Option<&str> = None;
    let mut in_block_comment = false;

    for line in fn_source.lines() {
        if !is_brace {
            let was_triple = in_triple.is_some();
            in_triple = update_triple_quote_state(line, in_triple);
            if was_triple || in_triple.is_some() {
                continue;
            }
            let masked = mask_python_line(line);
            // Keep characters up to any unquoted `#` comment
            let mut cut = line.len();
            for (i, ch) in line.char_indices() {
                if ch == '#' && masked.as_bytes().get(i) == Some(&b' ') {
                    // Check if `#` itself started the trailing comment mask
                    if i == 0 || masked[i..].bytes().all(|b| b == b' ') {
                        cut = i;
                        break;
                    }
                }
            }
            out_lines.push(line[..cut].to_string());
        } else {
            let mut prev_bc = in_block_comment;
            let _ = mask_brace_line(line, &mut in_block_comment);
            if prev_bc && in_block_comment {
                continue;
            }
            // Strip unquoted `//` tail while preserving string literals
            let bytes = line.as_bytes();
            let mut i = 0usize;
            let mut in_q: Option<u8> = None;
            let mut buf = Vec::with_capacity(bytes.len());
            while i < bytes.len() {
                let b = bytes[i];
                if prev_bc {
                    if b == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        prev_bc = false;
                        i += 2;
                    } else {
                        i += 1;
                    }
                    continue;
                }
                if let Some(q) = in_q {
                    buf.push(b);
                    if b == b'\\' && i + 1 < bytes.len() {
                        buf.push(bytes[i + 1]);
                        i += 2;
                        continue;
                    }
                    if b == q {
                        in_q = None;
                    }
                    i += 1;
                } else {
                    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                        break;
                    }
                    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'*' {
                        prev_bc = true;
                        i += 2;
                        continue;
                    }
                    if b == b'\''
                        && let Some(tok_len) = crate::ast_graph::non_quote_apostrophe_len(bytes, i)
                    {
                        buf.extend_from_slice(&bytes[i..i + tok_len]);
                        i += tok_len;
                        continue;
                    }
                    if b == b'"' || b == b'\'' || b == b'`' {
                        in_q = Some(b);
                    }
                    buf.push(b);
                    i += 1;
                }
            }
            out_lines.push(String::from_utf8(buf).unwrap_or_default());
        }
    }

    out_lines.join("\n")
}

/// Detects cross-layer integration slice boundaries (`entry_callers`, `internal_callees`, and
/// real SQL / JSON / HTTP I/O contracts) so tests exercise the real project call slice without
/// mocking internal project functions.
pub fn detect_integration_slice(
    target_id: &str,
    fn_source: &str,
    lang: &str,
    callers: &[String],
    callees: &[String],
    project_symbols: &HashMap<String, String>,
) -> Option<IntegrationSliceHint> {
    let mut internal_callees = Vec::new();
    for c in callees {
        let short = c.split('.').next_back().unwrap_or(c);
        if let Some(node_id) = project_symbols.get(c).or_else(|| project_symbols.get(short))
            && node_id != target_id
            && !internal_callees.contains(node_id)
        {
            internal_callees.push(node_id.clone());
        }
    }

    let is_brace = lang != "python" && is_brace_language_source(fn_source);
    let active_code = strip_comments_keep_strings(fn_source, is_brace);
    let upper_code = active_code.to_ascii_uppercase();

    let mut io_boundaries = Vec::new();
    let has_sql = upper_code.contains("SELECT ")
        || upper_code.contains("INSERT INTO ")
        || (upper_code.contains("UPDATE ") && upper_code.contains(" SET "))
        || upper_code.contains("DELETE FROM ")
        || upper_code.contains("CREATE TABLE ")
        || active_code.contains("sqlite3")
        || active_code.contains("rusqlite")
        || active_code.contains("sqlx::")
        || active_code.contains(".execute(")
        || active_code.contains(".executemany(")
        || active_code.contains("db.Query")
        || active_code.contains("db.Exec")
        || active_code.contains("PreparedStatement")
        || active_code.contains("SqlCommand");
    if has_sql {
        io_boundaries.push("sql_query".to_string());
    }

    let has_json = active_code.contains("json.loads")
        || active_code.contains("json.dumps")
        || active_code.contains("json.load(")
        || active_code.contains("json.dump(")
        || active_code.contains("serde_json")
        || active_code.contains("JSON.parse")
        || active_code.contains("JSON.stringify")
        || active_code.contains("json.Unmarshal")
        || active_code.contains("json.Marshal")
        || active_code.contains("ObjectMapper")
        || active_code.contains("JsonSerializer")
        || active_code.contains("Newtonsoft.Json");
    if has_json {
        io_boundaries.push("json_serde".to_string());
    }

    if callers.is_empty() && internal_callees.is_empty() && io_boundaries.is_empty() {
        return None;
    }

    let entry_callers: Vec<String> = callers.iter().take(5).cloned().collect();
    let mut chain_parts = Vec::new();
    if !entry_callers.is_empty() {
        chain_parts.push(entry_callers.join("|"));
    }
    chain_parts.push(target_id.to_string());
    if !internal_callees.is_empty() {
        chain_parts.push(internal_callees.join("|"));
    }
    let slice_chain = chain_parts.join(" -> ");

    let io_note = if io_boundaries.is_empty() {
        "real internal data structures".to_string()
    } else {
        format!(
            "real {} contract (e.g. in-memory SQLite / real JSON serializer)",
            io_boundaries.join(" + ")
        )
    };

    Some(IntegrationSliceHint {
        entry_callers,
        internal_callees,
        io_boundaries_detected: io_boundaries,
        directive: format!(
            "Write a Slice Integration Test exercising `{}` with {}; do NOT mock internal project functions.",
            slice_chain, io_note
        ),
    })
}

fn state_mutation_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?:\b(?:self|this)\.[A-Za-z_][A-Za-z0-9_]*\s*(?:\+=|-=|=)|\b[A-Za-z_][A-Za-z0-9_]*\.(?:balance|stock|inventory|quota|count|counter|credits|seats|available)\b\s*(?:\+=|-=|=)|\[['\x22](?:balance|stock|inventory|quota|count|counter)['\x22]\]|\b(?:cache|balances|accounts|wallets|inventory|sessions)\s*\[)",
        )
        .ok()
    })
    .as_ref()
}

/// Detects concurrency, synchronization, and read-modify-write shared-state mutation signals,
/// recommending a multi-worker barrier test harness.
pub fn detect_concurrency_risk(
    signature: &str,
    fn_source: &str,
    lang: &str,
) -> Option<ConcurrencyRiskHint> {
    let is_brace = lang != "python" && is_brace_language_source(fn_source);
    let active_code = strip_comments_keep_strings(fn_source, is_brace);
    let upper_code = active_code.to_ascii_uppercase();
    let mut signals = Vec::new();

    if signature.contains("async ")
        || active_code.contains("await ")
        || active_code.contains(".await")
        || active_code.contains("asyncio.")
        || active_code.contains("tokio::")
        || active_code.contains("Promise.all")
    {
        signals.push("async_fn".to_string());
    }

    if active_code.contains("Mutex")
        || active_code.contains("RwLock")
        || active_code.contains("Atomic")
        || active_code.contains("atomic.")
        || active_code.contains("sync.WaitGroup")
        || active_code.contains("threading.Lock")
        || active_code.contains("threading.RLock")
        || active_code.contains("._lock")
        || active_code.contains("with self.lock")
        || active_code.contains(".acquire(")
        || active_code.contains(".lock(")
        || active_code.contains(".Lock(")
        || active_code.contains(".RLock(")
        || active_code.contains("lock_guard")
        || active_code.contains("unique_lock")
        || active_code.contains("scoped_lock")
        || active_code.contains("pthread_mutex_")
        || active_code.contains("Semaphore")
        || active_code.contains("synchronized")
        || active_code.contains("lock (")
        || active_code.contains("Interlocked.")
        || active_code.contains("ConcurrentHashMap")
    {
        signals.push("mutex_or_atomic".to_string());
    }

    if active_code.contains("go func")
        || active_code.contains("chan ")
        || active_code.contains("<-")
        || active_code.contains("thread::spawn")
        || active_code.contains("threading.Thread")
        || active_code.contains("ThreadPoolExecutor")
    {
        signals.push("concurrent_workers".to_string());
    }

    let has_direct_state_mut = state_mutation_regex()
        .map(|re| re.is_match(&active_code))
        .unwrap_or(false);
    let has_sql_rmw = upper_code.contains("SELECT ")
        && upper_code.contains("UPDATE ")
        && upper_code.contains(" SET ");
    let has_sql_balance_deduct = upper_code.contains("UPDATE ")
        && (upper_code.contains("BALANCE")
            || upper_code.contains("STOCK")
            || upper_code.contains("INVENTORY")
            || upper_code.contains("QUOTA"));

    if has_direct_state_mut || has_sql_rmw || has_sql_balance_deduct {
        signals.push("shared_state_mutation".to_string());
    }

    if signals.is_empty() {
        return None;
    }

    let recommended_harness = match lang {
        "go" => "Spawn 16-32 goroutines synchronized on a start channel + sync.WaitGroup and assert conservation invariants (no double-spend / lost update).".to_string(),
        "rust" => "Spawn 16-32 threads synchronized with Arc<Barrier> and assert exact final state conservation without lost updates.".to_string(),
        "typescript" | "javascript" => "Launch 16-32 concurrent calls via Promise.all and assert exact state/balance conservation without double-spend.".to_string(),
        "java" => "Launch 16-32 worker threads released simultaneously via CyclicBarrier and assert exact state conservation.".to_string(),
        "csharp" => "Launch 16-32 concurrent tasks synchronized via System.Threading.Barrier / Task.WhenAll and assert exact state conservation.".to_string(),
        _ => "Launch 16-32 concurrent workers synchronized via threading.Barrier (or asyncio.gather for async functions) and assert conservation invariants (e.g. exact remaining balance, zero double-spend).".to_string(),
    };

    let runner_flag_hint = if lang == "go" {
        Some("--pytest-args \"-race\"".to_string())
    } else {
        None
    };

    Some(ConcurrencyRiskHint {
        signals,
        recommended_harness,
        runner_flag_hint,
    })
}

fn env_read_regexes() -> &'static Vec<Regex> {
    static RES: OnceLock<Vec<Regex>> = OnceLock::new();
    RES.get_or_init(|| {
        let patterns = [
            r#"\b(?:os\.)?getenv\(\s*["']([A-Za-z_][A-Za-z0-9_]*)["']"#,
            r#"\b(?:os\.)?environ\.get\(\s*["']([A-Za-z_][A-Za-z0-9_]*)["']"#,
            r#"\b(?:os\.)?environ\[\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\]"#,
            r#"(?:std::)?env::var(?:_os)?\(\s*"([A-Za-z_][A-Za-z0-9_]*)""#,
            r#"(?:option_)?env!\(\s*"([A-Za-z_][A-Za-z0-9_]*)""#,
            r#"process\.env\.([A-Za-z_][A-Za-z0-9_]*)"#,
            r#"process\.env\[\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\]"#,
            r#"import\.meta\.env\.([A-Za-z_][A-Za-z0-9_]*)"#,
            r#"import\.meta\.env\[\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\]"#,
            r#"Deno\.env\.get\(\s*["']([A-Za-z_][A-Za-z0-9_]*)["']"#,
            r#"Bun\.env\.([A-Za-z_][A-Za-z0-9_]*)"#,
            r#"Bun\.env\[\s*["']([A-Za-z_][A-Za-z0-9_]*)["']\s*\]"#,
            r#"os\.(?:Getenv|LookupEnv)\(\s*["`]([A-Za-z_][A-Za-z0-9_]*)["`]"#,
            r#"(?:std::)?getenv\(\s*"([A-Za-z_][A-Za-z0-9_]*)""#,
            r#"System\.getenv\(\s*"([A-Za-z_][A-Za-z0-9_]*)""#,
            r#"Environment\.GetEnvironmentVariable\(\s*"([A-Za-z_][A-Za-z0-9_]*)""#,
        ];
        patterns
            .iter()
            .filter_map(|p| Regex::new(p).ok())
            .collect()
    })
}

fn env_destructure_regex() -> Option<&'static Regex> {
    static RE: OnceLock<Option<Regex>> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\{\s*([^{};]+?)\s*\}\s*=\s*(?:process\.env|import\.meta\.env|Bun\.env)\b").ok()
    })
    .as_ref()
}

fn mask_python_comments_keep_strings(line: &str) -> String {
    let bytes = line.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    let mut in_quote: Option<&[u8]> = None;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(delim) = in_quote {
            if b == b'\\' && i + 1 < bytes.len() {
                out.push(b);
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if i + delim.len() <= bytes.len() && &bytes[i..i + delim.len()] == delim {
                out.extend_from_slice(delim);
                in_quote = None;
                i += delim.len();
                continue;
            }
            out.push(b);
            i += 1;
        } else {
            if b == b'#' {
                out.extend(std::iter::repeat_n(b' ', bytes.len() - i));
                break;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"\"\"\"" {
                out.extend_from_slice(b"\"\"\"");
                in_quote = Some(b"\"\"\"");
                i += 3;
                continue;
            }
            if i + 3 <= bytes.len() && &bytes[i..i + 3] == b"'''" {
                out.extend_from_slice(b"'''");
                in_quote = Some(b"'''");
                i += 3;
                continue;
            }
            if b == b'"' {
                in_quote = Some(b"\"");
                out.push(b);
                i += 1;
                continue;
            }
            if b == b'\'' {
                in_quote = Some(b"'");
                out.push(b);
                i += 1;
                continue;
            }
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

fn mask_brace_comments_keep_strings(line: &str, in_block_comment: &mut bool) -> String {
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
                out.push(b);
                out.push(bytes[i + 1]);
                i += 2;
                continue;
            }
            if b == q {
                out.push(b);
                in_quote = None;
                i += 1;
                continue;
            }
            out.push(b);
            i += 1;
        } else {
            if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                out.extend(std::iter::repeat_n(b' ', bytes.len() - i));
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
                && let Some(tok_len) = crate::ast_graph::non_quote_apostrophe_len(bytes, i)
            {
                out.extend_from_slice(&bytes[i..i + tok_len]);
                i += tok_len;
                continue;
            }
            if b == b'"' || b == b'\'' || b == b'`' {
                in_quote = Some(b);
                out.push(b);
                i += 1;
                continue;
            }
            out.push(b);
            i += 1;
        }
    }
    String::from_utf8(out).unwrap_or_else(|_| line.to_string())
}

fn build_env_scan_buffers(content: &str, is_brace: bool) -> (String, String) {
    let mut keep_lines = Vec::new();
    let mut masked_lines = Vec::new();
    let mut in_triple: Option<&str> = None;
    let mut bc_keep = false;
    let mut bc_mask = false;

    for raw_line in content.split('\n') {
        if !is_brace {
            let was_triple = in_triple.is_some();
            in_triple = update_triple_quote_state(raw_line, in_triple);
            if was_triple || in_triple.is_some() {
                let blank = " ".repeat(raw_line.len());
                keep_lines.push(blank.clone());
                masked_lines.push(blank);
                continue;
            }
            keep_lines.push(mask_python_comments_keep_strings(raw_line));
            masked_lines.push(mask_python_line(raw_line));
        } else {
            keep_lines.push(mask_brace_comments_keep_strings(raw_line, &mut bc_keep));
            masked_lines.push(mask_brace_line(raw_line, &mut bc_mask));
        }
    }

    (keep_lines.join("\n"), masked_lines.join("\n"))
}

fn line_number_at_offset(buf: &str, byte_offset: usize) -> usize {
    let end = byte_offset.min(buf.len());
    1 + buf.as_bytes()[..end]
        .iter()
        .filter(|&&b| b == b'\n')
        .count()
}

fn parse_env_template_keys(content: &str) -> Vec<String> {
    let mut keys = Vec::new();
    let stripped_bom = content.trim_start_matches('\u{feff}');
    for raw_line in stripped_bom.lines() {
        let trimmed = raw_line.trim().trim_start_matches('\u{feff}');
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let without_export = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        let Some((lhs, _)) = without_export.split_once('=') else {
            continue;
        };
        let key = lhs.trim();
        if !key.is_empty()
            && key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
            && !keys.contains(&key.to_string())
        {
            keys.push(key.to_string());
        }
    }
    keys
}

/// Scans `changed_files` across all 9 languages for environment variable reads and compares
/// referenced key names against `.env.example` / `.env.sample` / `.env.template` (never reading `.env`).
pub fn audit_changed_files_env_keys(
    repo_path: &Path,
    changed_files: &[String],
) -> ConfigAuditReport {
    let mut env_example_files = Vec::new();
    let mut declared_set = HashSet::new();
    let mut declared_env_example_keys = Vec::new();

    for &tmpl in ALLOWED_ENV_TEMPLATES {
        let abs_tmpl = repo_path.join(tmpl);
        if let Ok(meta) = fs::symlink_metadata(&abs_tmpl)
            && meta.is_file()
            && !meta.file_type().is_symlink()
            && let Some(content) = read_bounded_source_file(&abs_tmpl)
        {
            env_example_files.push(tmpl.to_string());
            for k in parse_env_template_keys(&content) {
                if declared_set.insert(k.clone()) {
                    declared_env_example_keys.push(k);
                }
            }
        }
    }
    declared_env_example_keys.sort();

    let regexes = env_read_regexes();
    let mut referenced_env_keys = Vec::new();
    let mut seen_usages = HashSet::new();

    for rel_file in changed_files {
        if is_test_file_path(rel_file) || is_secret_env_file(rel_file) {
            continue;
        }
        let Some(safe_rel) = sanitize_repo_relative_path(repo_path, rel_file) else {
            continue;
        };
        let abs_p = repo_path.join(&safe_rel);
        let Some(ext) = abs_p.extension().and_then(|s| s.to_str()) else {
            continue;
        };
        let Some(lang) = detect_language_for_ext(ext) else {
            continue;
        };
        let Some(content) = read_bounded_source_file(&abs_p) else {
            continue;
        };

        let is_brace = lang != "python";
        let (active_keep_strings, fully_masked) = build_env_scan_buffers(&content, is_brace);
        if fully_masked.trim().is_empty() {
            continue;
        }

        let keep_bytes = active_keep_strings.as_bytes();
        let masked_bytes = fully_masked.as_bytes();

        for re in regexes {
            for cap in re.captures_iter(&active_keep_strings) {
                let (Some(full_m), Some(key_m)) = (cap.get(0), cap.get(1)) else {
                    continue;
                };
                let start = full_m.start();
                // Verify the call site itself is active code (not inside a comment or string literal)
                if masked_bytes.get(start) != keep_bytes.get(start)
                    || masked_bytes.get(start) == Some(&b' ')
                {
                    continue;
                }
                let key = key_m.as_str().trim();
                if key.is_empty() {
                    continue;
                }
                let line_no = line_number_at_offset(&active_keep_strings, start);
                let usage_id = (key.to_string(), safe_rel.clone(), line_no);
                if seen_usages.insert(usage_id) {
                    referenced_env_keys.push(EnvKeyUsage {
                        key: key.to_string(),
                        file: safe_rel.clone(),
                        line: line_no,
                    });
                }
            }
        }

        if matches!(lang, "typescript" | "javascript")
            && let Some(destr_re) = env_destructure_regex()
        {
            for cap in destr_re.captures_iter(&active_keep_strings) {
                let (Some(full_m), Some(inner_m)) = (cap.get(0), cap.get(1)) else {
                    continue;
                };
                let start = full_m.start();
                if masked_bytes.get(start) != keep_bytes.get(start)
                    || masked_bytes.get(start) == Some(&b' ')
                {
                    continue;
                }
                let inner_start = inner_m.start();
                let inner_end = inner_m.end().min(fully_masked.len());
                if inner_start >= inner_end {
                    continue;
                }
                let masked_inner = &fully_masked[inner_start..inner_end];
                let mut seg_start = 0usize;
                for (rel_i, b) in masked_inner.bytes().enumerate() {
                    if b == b',' {
                        let seg = &masked_inner[seg_start..rel_i];
                        let trimmed_seg = seg.trim();
                        if !trimmed_seg.is_empty() && !trimmed_seg.starts_with("...") {
                            let before_eq = trimmed_seg.split('=').next().unwrap_or(trimmed_seg);
                            let key = before_eq.split(':').next().unwrap_or(before_eq).trim();
                            if !key.is_empty()
                                && key
                                    .chars()
                                    .enumerate()
                                    .all(|(i, c)| {
                                        c == '_'
                                            || (i == 0 && c.is_ascii_alphabetic())
                                            || (i > 0 && c.is_ascii_alphanumeric())
                                    })
                            {
                                let lead_ws = seg.len() - seg.trim_start().len();
                                let key_offset = inner_start + seg_start + lead_ws;
                                let line_no =
                                    line_number_at_offset(&active_keep_strings, key_offset);
                                let usage_id = (key.to_string(), safe_rel.clone(), line_no);
                                if seen_usages.insert(usage_id) {
                                    referenced_env_keys.push(EnvKeyUsage {
                                        key: key.to_string(),
                                        file: safe_rel.clone(),
                                        line: line_no,
                                    });
                                }
                            }
                        }
                        seg_start = rel_i + 1;
                    }
                }
                if seg_start <= masked_inner.len() {
                    let seg = &masked_inner[seg_start..];
                    let trimmed_seg = seg.trim();
                    if !trimmed_seg.is_empty() && !trimmed_seg.starts_with("...") {
                        let before_eq = trimmed_seg.split('=').next().unwrap_or(trimmed_seg);
                        let key = before_eq.split(':').next().unwrap_or(before_eq).trim();
                        if !key.is_empty()
                            && key
                                .chars()
                                .enumerate()
                                .all(|(i, c)| {
                                    c == '_'
                                        || (i == 0 && c.is_ascii_alphabetic())
                                        || (i > 0 && c.is_ascii_alphanumeric())
                                })
                        {
                            let lead_ws = seg.len() - seg.trim_start().len();
                            let key_offset = inner_start + seg_start + lead_ws;
                            let line_no = line_number_at_offset(&active_keep_strings, key_offset);
                            let usage_id = (key.to_string(), safe_rel.clone(), line_no);
                            if seen_usages.insert(usage_id) {
                                referenced_env_keys.push(EnvKeyUsage {
                                    key: key.to_string(),
                                    file: safe_rel.clone(),
                                    line: line_no,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    referenced_env_keys.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.key.cmp(&b.key))
    });

    let mut missing_set = HashSet::new();
    let mut missing_env_example_keys = Vec::new();
    for usage in &referenced_env_keys {
        if OS_BUILTIN_ENV_KEYS.contains(&usage.key.as_str()) {
            continue;
        }
        if !declared_set.contains(&usage.key) && missing_set.insert(usage.key.clone()) {
            missing_env_example_keys.push(usage.key.clone());
        }
    }
    missing_env_example_keys.sort();

    ConfigAuditReport {
        env_example_files,
        referenced_env_keys,
        declared_env_example_keys,
        missing_env_example_keys,
    }
}
