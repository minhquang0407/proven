use crate::arena::CausalReflexionEngine;
use crate::ast_graph::{
    ParsedFileSummary, detect_language_for_ext, find_matching_function, is_brace_language_source,
    is_declaration_file, is_noisy_dir, is_test_file_path, mask_brace_line, mask_python_line,
    parse_source_content, read_bounded_source_file, resolve_source_file,
    sanitize_repo_relative_path, scan_python_signature_colon, strip_python_string_prefix,
    update_triple_quote_state,
};
use crate::memory_cls::GraphMemoryManager;
use crate::models::{
    MutantEvalDetail, MutantSpec, MutationGateResult, VerifyProofResult,
};
use rayon::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use walkdir::WalkDir;

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// Finds an operator in `masked` starting at byte offset `from_pos`, ensuring word-boundary
/// isolation when `orig_op` starts/ends with identifier characters (e.g. `True` must never
/// match inside `assertTrue`).
fn find_operator_match_from(masked: &str, orig_op: &str, from_pos: usize) -> Option<usize> {
    let bytes = masked.as_bytes();
    let op_bytes = orig_op.as_bytes();
    let &last_b = op_bytes.last()?;
    if from_pos + op_bytes.len() > bytes.len() {
        return None;
    }
    let check_left = is_ident_byte(op_bytes[0]);
    let check_right = is_ident_byte(last_b);

    let mut search_from = from_pos;
    while search_from + op_bytes.len() <= bytes.len() {
        let Some(rel_pos) = masked[search_from..].find(orig_op) else {
            break;
        };
        let pos = search_from + rel_pos;
        let end = pos + op_bytes.len();
        let left_ok = !check_left || pos == 0 || !is_ident_byte(bytes[pos - 1]);
        let right_ok = !check_right || end >= bytes.len() || !is_ident_byte(bytes[end]);
        if left_ok && right_ok {
            return Some(pos);
        }
        let step = masked[pos..].chars().next().map_or(1, char::len_utf8);
        search_from = pos + step;
    }
    None
}

fn is_numeric_literal_token(rhs_trim: &str, digit: char) -> bool {
    rhs_trim.starts_with(digit)
        && !rhs_trim[digit.len_utf8()..]
            .starts_with(|c: char| c.is_ascii_alphanumeric() || c == '.' || c == '_')
}

fn is_valid_binary_lhs(prefix: &str) -> bool {
    let p = prefix.trim_end();
    let Some(last_ch) = p.chars().last() else {
        return false;
    };
    if !(last_ch.is_ascii_alphanumeric() || matches!(last_ch, '_' | ')' | ']' | '"' | '\'')) {
        return false;
    }
    let tail_word: String = p
        .chars()
        .rev()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    !matches!(
        tail_word.as_str(),
        "move"
            | "async"
            | "return"
            | "else"
            | "do"
            | "case"
            | "throw"
            | "yield"
            | "await"
            | "if"
            | "elif"
            | "while"
    )
}

fn is_value_shift_context(raw_line: &str, masked: &str, pos: usize) -> bool {
    if !is_valid_binary_lhs(&masked[..pos]) {
        return false;
    }
    if raw_line.contains('"') || raw_line.contains('\'') {
        return false;
    }
    for stream_kw in [
        "cout",
        "cerr",
        "clog",
        "cin",
        "endl",
        "flush",
        "ostream",
        "istream",
        "stringstream",
    ] {
        if find_operator_match_from(masked, stream_kw, 0).is_some() {
            return false;
        }
    }
    let before = &masked[..pos];
    before.contains(['=', '(', '[', ',', '?', ':', '|', '&', '^', '+', '-', '*'])
        || find_operator_match_from(before, "return", 0).is_some()
        || find_operator_match_from(before, "if", 0).is_some()
        || find_operator_match_from(before, "elif", 0).is_some()
        || find_operator_match_from(before, "while", 0).is_some()
        || find_operator_match_from(before, "assert", 0).is_some()
}

fn is_valid_operator_candidate(
    is_brace: bool,
    raw_line: &str,
    masked: &str,
    orig_op: &str,
    pos: usize,
) -> bool {
    if orig_op == " = true" && masked[..pos].trim_end().ends_with(']') {
        return false;
    }
    if !is_brace
        && orig_op == " in "
        && (find_operator_match_from(masked, "for", 0).is_some()
            || !is_valid_binary_lhs(&masked[..pos]))
    {
        return false;
    }
    if matches!(orig_op, " && " | " || " | " and " | " or ") && !is_valid_binary_lhs(&masked[..pos])
    {
        return false;
    }
    if matches!(orig_op, " << " | " >> ") && !is_value_shift_context(raw_line, masked, pos) {
        return false;
    }

    let rhs_trim = masked[pos + orig_op.len()..].trim_start();
    if matches!(
        orig_op,
        " + " | " - " | " += " | " -= " | " << " | " >> " | " <<= " | " >>= "
    ) {
        if is_numeric_literal_token(rhs_trim, '0') {
            return false;
        }
        // Filter 64-bit shift modulo-64 equivalent mutants: 1L << (x - 64) == 1L << (x + 64)
        if masked[..pos].contains("1L <<")
            && (rhs_trim.starts_with("64") || rhs_trim.starts_with("LONG_BITS"))
        {
            return false;
        }
    }
    if matches!(
        orig_op,
        " * " | " / " | " // " | " *= " | " /= " | " //= "
    ) && (is_numeric_literal_token(rhs_trim, '0') || is_numeric_literal_token(rhs_trim, '1'))
    {
        return false;
    }

    true
}

fn select_evenly<T>(items: Vec<T>, k: usize) -> Vec<T> {
    let n = items.len();
    if k == 0 {
        return Vec::new();
    }
    if n <= k {
        return items;
    }
    if k == 1 {
        return items.into_iter().next().into_iter().collect();
    }
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
    let mut out = Vec::with_capacity(k);
    for i in 0..k {
        let idx = i * (n - 1) / (k - 1);
        if let Some(val) = slots[idx].take() {
            out.push(val);
        }
    }
    out
}

/// Checks whether a test file actually invokes or references `short_sym` in executable test code
/// (ignoring `#` / `//` / `/*...*/` comments, multi-line `"""`/`'''` docstrings, multi-line
/// `from ... import (...)`, `import { ... }`, `use ...` blocks, and `def test_...` function headers).
fn is_target_invoked_in_test(test_content: &str, short_sym: &str) -> bool {
    let trimmed_sym = short_sym.trim();
    if trimmed_sym.is_empty() {
        return false;
    }
    let escaped = regex::escape(trimmed_sym);
    let Ok(sym_re) = Regex::new(&format!(r"\b{}\b", escaped)) else {
        return false;
    };
    let is_brace = is_brace_language_source(test_content);
    let mut in_triple: Option<&str> = None;
    let mut in_block_comment = false;
    let mut in_multiline_import = false;
    let mut import_paren_depth: i32 = 0;

    for line in test_content.lines() {
        if !is_brace {
            let was_triple = in_triple.is_some();
            in_triple = update_triple_quote_state(line, in_triple);
            if was_triple || in_triple.is_some() {
                continue;
            }
        }

        let code_only = if is_brace {
            mask_brace_line(line, &mut in_block_comment)
        } else {
            mask_python_line(line)
        };
        let code_trim = code_only.trim();
        let trimmed = line.trim();
        let stripped_prefix = strip_python_string_prefix(trimmed);

        if code_trim.is_empty()
            || (!is_brace && trimmed.starts_with('#'))
            || (is_brace && trimmed.starts_with("//"))
            || trimmed.starts_with("def test_")
            || trimmed.starts_with("async def test_")
            || trimmed.starts_with("fn test_")
            || trimmed.starts_with("pub fn test_")
            || trimmed.starts_with("async fn test_")
            || (!is_brace
                && (stripped_prefix.starts_with("\"\"\"") || stripped_prefix.starts_with("'''")))
        {
            continue;
        }

        if in_multiline_import {
            for ch in code_trim.chars() {
                if matches!(ch, '(' | '{') {
                    import_paren_depth += 1;
                } else if matches!(ch, ')' | '}') {
                    import_paren_depth -= 1;
                }
            }
            if import_paren_depth <= 0 && !code_trim.ends_with('\\') {
                in_multiline_import = false;
            }
            continue;
        }

        if code_trim.starts_with("import ")
            || code_trim.starts_with("from ")
            || (is_brace && code_trim.starts_with("use "))
        {
            for ch in code_trim.chars() {
                if matches!(ch, '(' | '{') {
                    import_paren_depth += 1;
                } else if matches!(ch, ')' | '}') {
                    import_paren_depth -= 1;
                }
            }
            if import_paren_depth > 0 || code_trim.ends_with('\\') {
                in_multiline_import = true;
            }
            continue;
        }

        if code_trim.contains("if False")
            || code_trim.contains("if false")
            || code_trim.contains("if (false)")
            || code_trim.contains("if 0:")
        {
            continue;
        }

        if sym_re.is_match(code_trim) {
            return true;
        }
    }
    false
}

/// Sanitizes extra pytest arguments against flag injection (`-o`, `-c`, `-p`, `--rootdir`, etc.).
pub fn sanitize_pytest_extra_args(extra_args: Option<&str>) -> Vec<String> {
    let Some(raw) = extra_args else {
        return Vec::new();
    };
    let tokens: Vec<&str> = raw.split_whitespace().collect();
    let mut safe = Vec::new();
    let mut idx = 0;
    while idx < tokens.len() {
        let tok = tokens[idx];
        if matches!(tok, "-q" | "-v" | "-vv" | "-s" | "-x" | "--no-header") {
            safe.push(tok.to_string());
        } else if matches!(tok, "-k" | "-m") {
            if let Some(&val) = tokens.get(idx + 1)
                && !val.starts_with('-')
                    && val
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ' ' | '.'))
                {
                    safe.push(tok.to_string());
                    safe.push(val.to_string());
                    idx += 1;
                }
        } else if (tok.starts_with("--tb=") || tok.starts_with("--maxfail="))
            && tok
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '=' | '_'))
        {
            safe.push(tok.to_string());
        }
        idx += 1;
    }
    safe
}

use std::cell::RefCell;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

static IN_PROCESS_MUTATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

thread_local! {
    static HELD_REPO_LOCKS: RefCell<HashSet<PathBuf>> = RefCell::new(HashSet::new());
}

/// Reentrant intra-process + inter-process lock guard ensuring concurrent threads or
/// subagent processes never mutate or test the same repository simultaneously.
pub struct RepoMutationLockGuard {
    canon_repo: Option<PathBuf>,
    lock_file: Option<PathBuf>,
    _mutex_guard: Option<MutexGuard<'static, ()>>,
}

impl RepoMutationLockGuard {
    pub fn acquire(repo_path: &Path) -> Self {
        let canon = repo_path
            .canonicalize()
            .unwrap_or_else(|_| repo_path.to_path_buf());
        let already_held = HELD_REPO_LOCKS.with(|set| set.borrow().contains(&canon));
        if already_held {
            return Self {
                canon_repo: None,
                lock_file: None,
                _mutex_guard: None,
            };
        }

        let mutex = IN_PROCESS_MUTATION_LOCK.get_or_init(|| Mutex::new(()));
        let guard = match mutex.lock() {
            Ok(g) => Some(g),
            Err(poisoned) => Some(poisoned.into_inner()),
        };

        let proven_dir = repo_path.join(".proven");
        let _ = fs::create_dir_all(&proven_dir);
        let lock_path = proven_dir.join(".mutation.lock");
        let start = Instant::now();
        let mut acquired_file = false;

        while start.elapsed() < Duration::from_secs(30) {
            match OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(mut f) => {
                    let _ = write!(f, "{}", std::process::id());
                    acquired_file = true;
                    break;
                }
                Err(_) => {
                    if let Ok(meta) = fs::metadata(&lock_path)
                        && let Ok(modified) = meta.modified()
                            && SystemTime::now()
                                .duration_since(modified)
                                .unwrap_or(Duration::ZERO)
                                > Duration::from_secs(60)
                            {
                                let _ = fs::remove_file(&lock_path);
                                continue;
                            }
                    thread::sleep(Duration::from_millis(15));
                }
            }
        }

        HELD_REPO_LOCKS.with(|set| {
            set.borrow_mut().insert(canon.clone());
        });

        Self {
            canon_repo: Some(canon),
            lock_file: if acquired_file { Some(lock_path) } else { None },
            _mutex_guard: guard,
        }
    }
}

impl Drop for RepoMutationLockGuard {
    fn drop(&mut self) {
        if let Some(ref lock_path) = self.lock_file {
            let _ = fs::remove_file(lock_path);
        }
        if let Some(ref canon) = self.canon_repo {
            HELD_REPO_LOCKS.with(|set| {
                set.borrow_mut().remove(canon);
            });
        }
    }
}

/// RAII Guard that guarantees atomic restoration of the original source file even on panic.
pub struct FileRestoreGuard {
    path: PathBuf,
    original_content: String,
}

impl FileRestoreGuard {
    pub fn new(path: PathBuf, original_content: String) -> Self {
        Self {
            path,
            original_content,
        }
    }
}

impl Drop for FileRestoreGuard {
    fn drop(&mut self) {
        let _ = fs::write(&self.path, &self.original_content);
    }
}

pub struct MicroMutationGate;

impl MicroMutationGate {
    /// Generate 3-Persona Red Team AST Mutants:
    /// - Red 1 (Boundary Hunter): comparison & arithmetic operators
    /// - Red 2 (Side-Effect Nullifier): boolean literals & state assignments
    /// - Red 3 (Chaos Poisoner): guard conditions & exception branches
    pub fn collect_mutants_for_target(
        source_code: &str,
        start_line: usize,
        end_line: usize,
        max_mutants: usize,
    ) -> Vec<MutantSpec> {
        if max_mutants == 0 {
            return Vec::new();
        }
        let lines: Vec<&str> = source_code.lines().collect();
        let s_idx = start_line.saturating_sub(1);
        let e_idx = end_line.min(lines.len());
        let is_brace = is_brace_language_source(source_code);

        let python_swaps: &[(&str, &str, &str, &str)] = &[
            (" >= ", " < ", "GtE -> Lt", "Red1_BoundaryHunter"),
            (" <= ", " > ", "LtE -> Gt", "Red1_BoundaryHunter"),
            (" > ", " <= ", "Gt -> LtE", "Red1_BoundaryHunter"),
            (" < ", " >= ", "Lt -> GtE", "Red1_BoundaryHunter"),
            (" == ", " != ", "Eq -> NotEq", "Red1_BoundaryHunter"),
            (" != ", " == ", "NotEq -> Eq", "Red1_BoundaryHunter"),
            (" is None", " is not None", "Is -> IsNot", "Red3_ChaosPoisoner"),
            (" is not None", " is None", "IsNot -> Is", "Red3_ChaosPoisoner"),
            (" not in ", " in ", "NotIn -> In", "Red3_ChaosPoisoner"),
            (" in ", " not in ", "In -> NotIn", "Red3_ChaosPoisoner"),
            ("if not ", "if ", "IfNot -> If", "Red3_ChaosPoisoner"),
            ("elif not ", "elif ", "ElifNot -> Elif", "Red3_ChaosPoisoner"),
            (" and ", " or ", "And -> Or", "Red3_ChaosPoisoner"),
            (" or ", " and ", "Or -> And", "Red3_ChaosPoisoner"),
            ("True", "False", "bool: True -> False", "Red2_SideEffectNullifier"),
            ("False", "True", "bool: False -> True", "Red2_SideEffectNullifier"),
            ("return 0", "return 1", "ReturnZero -> ReturnOne", "Red2_SideEffectNullifier"),
            ("return -1", "return 1", "ReturnNegOne -> ReturnOne", "Red2_SideEffectNullifier"),
            (" += ", " -= ", "AugAdd -> AugSub", "Red2_SideEffectNullifier"),
            (" -= ", " += ", "AugSub -> AugAdd", "Red2_SideEffectNullifier"),
            (" //= ", " *= ", "AugFloorDiv -> AugMult", "Red2_SideEffectNullifier"),
            (" |= ", " &= ", "AugBitOr -> AugBitAnd", "Red2_SideEffectNullifier"),
            (" &= ", " |= ", "AugBitAnd -> AugBitOr", "Red2_SideEffectNullifier"),
            (" <<= ", " >>= ", "AugShl -> AugShr", "Red2_SideEffectNullifier"),
            (" >>= ", " <<= ", "AugShr -> AugShl", "Red2_SideEffectNullifier"),
            (" + ", " - ", "Add -> Sub", "Red1_BoundaryHunter"),
            (" - ", " + ", "Sub -> Add", "Red1_BoundaryHunter"),
            (" * ", " // ", "Mult -> FloorDiv", "Red1_BoundaryHunter"),
            (" // ", " * ", "FloorDiv -> Mult", "Red1_BoundaryHunter"),
            (" << ", " >> ", "Shl -> Shr", "Red1_BoundaryHunter"),
            (" >> ", " << ", "Shr -> Shl", "Red1_BoundaryHunter"),
        ];

        let brace_swaps: &[(&str, &str, &str, &str)] = &[
            (" >= ", " < ", "GtE -> Lt", "Red1_BoundaryHunter"),
            (" <= ", " > ", "LtE -> Gt", "Red1_BoundaryHunter"),
            (" > ", " <= ", "Gt -> LtE", "Red1_BoundaryHunter"),
            (" < ", " >= ", "Lt -> GtE", "Red1_BoundaryHunter"),
            (" === ", " !== ", "StrictEq -> StrictNotEq", "Red1_BoundaryHunter"),
            (" !== ", " === ", "StrictNotEq -> StrictEq", "Red1_BoundaryHunter"),
            (" == ", " != ", "Eq -> NotEq", "Red1_BoundaryHunter"),
            (" != ", " == ", "NotEq -> Eq", "Red1_BoundaryHunter"),
            ("if !", "if ", "IfNot -> If", "Red3_ChaosPoisoner"),
            (" && ", " || ", "And -> Or", "Red3_ChaosPoisoner"),
            (" || ", " && ", "Or -> And", "Red3_ChaosPoisoner"),
            (" = true", " = false", "bool: true -> false", "Red2_SideEffectNullifier"),
            (" = false", " = true", "bool: false -> true", "Red2_SideEffectNullifier"),
            ("True", "False", "bool: True -> False", "Red2_SideEffectNullifier"),
            ("False", "True", "bool: False -> True", "Red2_SideEffectNullifier"),
            (" += ", " -= ", "AugAdd -> AugSub", "Red2_SideEffectNullifier"),
            (" -= ", " += ", "AugSub -> AugAdd", "Red2_SideEffectNullifier"),
            (" *= ", " /= ", "AugMult -> AugDiv", "Red2_SideEffectNullifier"),
            (" /= ", " *= ", "AugDiv -> AugMult", "Red2_SideEffectNullifier"),
            (" |= ", " &= ", "AugBitOr -> AugBitAnd", "Red2_SideEffectNullifier"),
            (" &= ", " |= ", "AugBitAnd -> AugBitOr", "Red2_SideEffectNullifier"),
            (" <<= ", " >>= ", "AugShl -> AugShr", "Red2_SideEffectNullifier"),
            (" >>= ", " <<= ", "AugShr -> AugShl", "Red2_SideEffectNullifier"),
            (" + ", " - ", "Add -> Sub", "Red1_BoundaryHunter"),
            (" - ", " + ", "Sub -> Add", "Red1_BoundaryHunter"),
            (" * ", " / ", "Mult -> Div", "Red1_BoundaryHunter"),
            (" / ", " * ", "Div -> Mult", "Red1_BoundaryHunter"),
            ("if (!", "if (", "IfNot -> If", "Red3_ChaosPoisoner"),
            ("return true", "return false", "ReturnTrue -> ReturnFalse", "Red2_SideEffectNullifier"),
            ("return false", "return true", "ReturnFalse -> ReturnTrue", "Red2_SideEffectNullifier"),
            ("return 0", "return 1", "ReturnZero -> ReturnOne", "Red2_SideEffectNullifier"),
            ("return -1", "return 1", "ReturnNegOne -> ReturnOne", "Red2_SideEffectNullifier"),
            (" >>> 0", " >>> 1", "BitShiftZero -> BitShiftOne", "Red1_BoundaryHunter"),
            (" << ", " >> ", "Shl -> Shr", "Red1_BoundaryHunter"),
            (" >> ", " << ", "Shr -> Shl", "Red1_BoundaryHunter"),
            ("if (handler)", "if (!handler)", "If -> IfNot", "Red3_ChaosPoisoner"),
            ("if (handlers)", "if (!handlers)", "If -> IfNot", "Red3_ChaosPoisoner"),
        ];

        let operator_swaps = if is_brace { brace_swaps } else { python_swaps };

        let mut in_signature = false;
        let mut sig_paren_depth: i32 = 0;
        let mut sig_seen_open = false;
        let mut in_triple: Option<&str> = None;
        let mut in_block_comment = false;

        let mut primary_candidates: Vec<(usize, usize, usize, &str, &str, &str)> = Vec::new();
        let mut secondary_candidates: Vec<(usize, usize, usize, &str, &str, &str)> = Vec::new();

        for (line_idx, &line) in lines.iter().enumerate().take(e_idx).skip(s_idx) {
            if !is_brace {
                let was_triple = in_triple.is_some();
                in_triple = update_triple_quote_state(line, in_triple);
                if was_triple || in_triple.is_some() {
                    continue;
                }
            }

            let mut masked = if is_brace {
                mask_brace_line(line, &mut in_block_comment)
            } else {
                mask_python_line(line)
            };
            let code_trimmed = masked.trim().to_string();
            let trimmed = line.trim();

            if code_trimmed.starts_with("def ")
                || code_trimmed.starts_with("async def ")
                || code_trimmed.starts_with("fn ")
                || code_trimmed.starts_with("pub fn ")
                || code_trimmed.starts_with("pub async fn ")
                || code_trimmed.starts_with("pub(")
                || code_trimmed.starts_with("function ")
                || code_trimmed.starts_with("export function ")
                || code_trimmed.starts_with("export async function ")
                || code_trimmed.starts_with("export const ")
                || code_trimmed.starts_with("func ")
            {
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
                        // Mask signature header up to ':' so default parameters are never mutated,
                        // while allowing inline one-liner body after ':' to be mutated.
                        masked = format!("{}{}", " ".repeat(colon_idx + 1), &masked[colon_idx + 1..]);
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
                || trimmed.starts_with('#')
                || (is_brace && trimmed.starts_with("//"))
                || trimmed.starts_with('@')
                || trimmed.starts_with("class ")
                || (!is_brace
                    && (stripped_prefix.starts_with("\"\"\"")
                        || stripped_prefix.starts_with("'''")))
            {
                continue;
            }

            let mut used_spans: Vec<(usize, usize)> = Vec::with_capacity(3);

            for &(orig_op, new_op, desc, persona) in operator_swaps {
                let mut search_from = 0usize;
                while let Some(pos) = find_operator_match_from(&masked, orig_op, search_from) {
                    let end = pos + orig_op.len();
                    let step = masked[pos..].chars().next().map_or(1, char::len_utf8);
                    search_from = pos + step;

                    if used_spans.iter().any(|&(s, e)| !(end <= s || pos >= e)) {
                        continue;
                    }
                    if !is_valid_operator_candidate(is_brace, line, &masked, orig_op, pos) {
                        continue;
                    }

                    let candidate = (line_idx, pos, orig_op.len(), new_op, desc, persona);
                    if used_spans.is_empty() {
                        primary_candidates.push(candidate);
                    } else {
                        secondary_candidates.push(candidate);
                    }
                    used_spans.push((pos, end));
                    if used_spans.len() >= 3 {
                        break;
                    }
                }
                if used_spans.len() >= 3 {
                    break;
                }
            }
        }

        let mut chosen = select_evenly(primary_candidates, max_mutants);
        if chosen.len() < max_mutants && !secondary_candidates.is_empty() {
            let rem = max_mutants - chosen.len();
            chosen.extend(select_evenly(secondary_candidates, rem));
        }
        chosen.sort_by_key(|c| (c.0, c.1));

        let mut mutants = Vec::with_capacity(chosen.len());
        for (idx, (line_idx, pos, orig_len, new_op, desc, persona)) in
            chosen.into_iter().enumerate()
        {
            let line = lines[line_idx];
            let mut mutated_line = String::with_capacity(line.len() + new_op.len());
            mutated_line.push_str(&line[..pos]);
            mutated_line.push_str(new_op);
            mutated_line.push_str(&line[pos + orig_len..]);
            let mut new_lines: Vec<String> = lines.iter().map(|s| (*s).to_string()).collect();
            new_lines[line_idx] = mutated_line.clone();
            mutants.push(MutantSpec {
                mutant_id: idx + 1,
                persona: persona.to_string(),
                line: line_idx + 1,
                mutation: desc.to_string(),
                original_line: line.trim().to_string(),
                mutated_line: mutated_line.trim().to_string(),
                mutated_source: new_lines.join("\n") + "\n",
            });
        }

        mutants
    }

    pub fn evaluate_mutations(
        repo_path: &Path,
        target_id: &str,
        source_file: &str,
        test_target: &str,
        max_mutants: usize,
        pytest_args: Option<&str>,
    ) -> MutationGateResult {
        let Some(safe_source_rel) = sanitize_repo_relative_path(repo_path, source_file) else {
            return MutationGateResult {
                status: "error".to_string(),
                mutation_gate_status: "rejected".to_string(),
                mutants_total: 0,
                mutants_killed: 0,
                mutants_survived: 0,
                mutation_score: 0.0,
                proof_grade: "FAILED".to_string(),
                message: format!(
                    "Security boundary violation: rejected unsafe source_file path '{}'",
                    source_file
                ),
                details: Vec::new(),
                causal_reflexion: None,
            };
        };

        let _repo_lock = RepoMutationLockGuard::acquire(repo_path);
        let abs_source = repo_path.join(&safe_source_rel);
        let Some(original_code) = read_bounded_source_file(&abs_source) else {
            return MutationGateResult {
                status: "error".to_string(),
                mutation_gate_status: "error".to_string(),
                mutants_total: 0,
                mutants_killed: 0,
                mutants_survived: 0,
                mutation_score: 0.0,
                proof_grade: "FAILED".to_string(),
                message: format!("Source file not found or exceeds 1MB limit: {}", safe_source_rel),
                details: Vec::new(),
                causal_reflexion: None,
            };
        };

        let ext = abs_source.extension().and_then(|s| s.to_str()).unwrap_or("py");
        let lang = detect_language_for_ext(ext).unwrap_or("python");
        let parsed = parse_source_content(&original_code, &safe_source_rel, lang);

        let clean_sym = target_id
            .trim()
            .strip_prefix("FUNC:")
            .or_else(|| target_id.trim().strip_prefix("CLASS:"))
            .or_else(|| target_id.trim().strip_prefix("FILE:"))
            .unwrap_or(target_id.trim());
        let after_file = clean_sym.rsplit("::").next().unwrap_or(clean_sym);
        let is_file_target = target_id.trim().starts_with("FILE:")
            || Path::new(after_file)
                .extension()
                .and_then(|s| s.to_str())
                .and_then(detect_language_for_ext)
                .is_some();
        let short_sym = after_file.split('.').next_back().unwrap_or(after_file);
        let has_class = parsed.classes.iter().any(|c| c == short_sym || c == after_file);

        let matched_fn = find_matching_function(&parsed.functions, target_id);
        if (target_id.trim().starts_with("FUNC:") && matched_fn.is_none())
            || (!is_file_target && matched_fn.is_none() && !has_class)
        {
            return MutationGateResult {
                status: "error".to_string(),
                mutation_gate_status: "error".to_string(),
                mutants_total: 0,
                mutants_killed: 0,
                mutants_survived: 0,
                mutation_score: 0.0,
                proof_grade: "FAILED".to_string(),
                message: format!(
                    "Target '{}' not found in source file '{}'",
                    target_id, safe_source_rel
                ),
                details: Vec::new(),
                causal_reflexion: None,
            };
        }

        let (start_line, end_line) = matched_fn
            .map(|f| (f.start, f.end))
            .unwrap_or((1, original_code.lines().count().max(1)));

        let effective_max = max_mutants.max(1);
        let specs =
            Self::collect_mutants_for_target(&original_code, start_line, end_line, effective_max);

        if specs.is_empty() {
            return MutationGateResult {
                status: "passed".to_string(),
                mutation_gate_status: "passed".to_string(),
                mutants_total: 0,
                mutants_killed: 0,
                mutants_survived: 0,
                mutation_score: 1.0,
                proof_grade: "TITANIUM".to_string(),
                message: "No mutatable AST operators found in target function.".to_string(),
                details: Vec::new(),
                causal_reflexion: None,
            };
        }

        let _restore_guard = FileRestoreGuard::new(abs_source.clone(), original_code.clone());
        let mut details = Vec::new();
        let mut killed = 0;
        let mut survived = 0;

        for spec in specs {
            if fs::write(&abs_source, &spec.mutated_source).is_err() {
                continue;
            }
            let passed = Self::run_test_runner(repo_path, test_target, pytest_args).0;
            let _ = fs::write(&abs_source, &original_code);

            let status = if !passed {
                killed += 1;
                "killed"
            } else {
                survived += 1;
                "survived"
            };

            details.push(MutantEvalDetail {
                mutant_id: spec.mutant_id,
                persona: spec.persona,
                line: spec.line,
                mutation: spec.mutation,
                original_code: spec.original_line,
                mutated_code: spec.mutated_line,
                status: status.to_string(),
                verdict: if status == "killed" {
                    "KILLED (Test caught the bug)".to_string()
                } else {
                    "SURVIVED (Weak assertion: Test passed buggy code)".to_string()
                },
            });
        }

        let total = details.len();
        let score = if total > 0 {
            ((killed as f64 / total as f64) * 100.0).round() / 100.0
        } else {
            1.0
        };

        let mut causal_reflexion = None;
        let (gate_status, proof_grade, message) = if let Some(first_survived) =
            details.iter().find(|d| d.status == "survived")
        {
            let reflexion = CausalReflexionEngine::diagnose(target_id, first_survived);
            let _ = GraphMemoryManager::pin_lesson(
                repo_path,
                target_id,
                &reflexion.actionable_directive,
                &reflexion.failure_mode,
                &first_survived.mutation,
            );
            causal_reflexion = Some(reflexion);
            (
                "weak_assertions".to_string(),
                "SILVER".to_string(),
                format!(
                    "WEAK ASSERTION DETECTED: {}/{} mutant(s) SURVIVED at line {} ({}: `{}`).",
                    survived,
                    total,
                    first_survived.line,
                    first_survived.mutation,
                    first_survived.original_code
                ),
            )
        } else {
            (
                "passed".to_string(),
                "TITANIUM".to_string(),
                format!(
                    "TITANIUM PROOF CONFIRMED: All {}/{} mutants were KILLED!",
                    killed, total
                ),
            )
        };

        MutationGateResult {
            status: "success".to_string(),
            mutation_gate_status: gate_status,
            mutants_total: total,
            mutants_killed: killed,
            mutants_survived: survived,
            mutation_score: score,
            proof_grade,
            message,
            details,
            causal_reflexion,
        }
    }

    pub fn verify_proof(
        repo_path: &Path,
        target_id: &str,
        test_target: &str,
        source_file: Option<&str>,
        mutation_check: bool,
        max_mutants: usize,
        pytest_args: Option<&str>,
    ) -> VerifyProofResult {
        if let Some(explicit_src) = source_file
            && !explicit_src.trim().is_empty()
                && sanitize_repo_relative_path(repo_path, explicit_src).is_none()
            {
                return VerifyProofResult {
                    status: "error".to_string(),
                    track: "rust_native".to_string(),
                    proof_status: "fail".to_string(),
                    proof_grade: "FAILED".to_string(),
                    target_id: target_id.to_string(),
                    test_target: test_target.to_string(),
                    pytest_passed: false,
                    pytest_returncode: 1,
                    message: format!(
                        "Security boundary violation: rejected unsafe source_file path '{}'",
                        explicit_src
                    ),
                    covered_fraction: 0.0,
                    covered_lines: Vec::new(),
                    covered_line_count: 0,
                    function_line_count: 0,
                    mutation_proof: None,
                };
            }

        let _repo_lock = RepoMutationLockGuard::acquire(repo_path);
        let resolved_source =
            resolve_source_file(repo_path, target_id, source_file).unwrap_or_default();
        let (passed, code, output) = Self::run_test_runner(repo_path, test_target, pytest_args);

        if !passed {
            return VerifyProofResult {
                status: "error".to_string(),
                track: "rust_native".to_string(),
                proof_status: "fail".to_string(),
                proof_grade: "FAILED".to_string(),
                target_id: target_id.to_string(),
                test_target: test_target.to_string(),
                pytest_passed: false,
                pytest_returncode: code,
                message: format!(
                    "Test runner failed with exit code {}. Output: {}",
                    code,
                    output.lines().last().unwrap_or("")
                ),
                covered_fraction: 0.0,
                covered_lines: Vec::new(),
                covered_line_count: 0,
                function_line_count: 0,
                mutation_proof: None,
            };
        }

        let runner_notes: Vec<String> = output
            .lines()
            .map(str::trim)
            .filter(|l| l.starts_with("[proven note:"))
            .map(ToString::to_string)
            .collect();

        // Verify that the test actually references/invokes the target symbol in executable code
        let clean_sym = target_id
            .strip_prefix("FUNC:")
            .or_else(|| target_id.strip_prefix("CLASS:"))
            .or_else(|| target_id.strip_prefix("FILE:"))
            .unwrap_or(target_id);
        let after_file = clean_sym.rsplit("::").next().unwrap_or(clean_sym);
        let is_file_target = target_id.trim().starts_with("FILE:")
            || Path::new(after_file)
                .extension()
                .and_then(|s| s.to_str())
                .and_then(detect_language_for_ext)
                .is_some();
        let short_sym = if is_file_target {
            Path::new(after_file)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(after_file)
        } else {
            after_file.split('.').next_back().unwrap_or(after_file)
        };
        let raw_test_file = test_target.split("::").next().unwrap_or(test_target);
        let Some(safe_test_rel) = sanitize_repo_relative_path(repo_path, raw_test_file) else {
            return VerifyProofResult {
                status: "error".to_string(),
                track: "rust_native".to_string(),
                proof_status: "fail".to_string(),
                proof_grade: "FAILED".to_string(),
                target_id: target_id.to_string(),
                test_target: test_target.to_string(),
                pytest_passed: false,
                pytest_returncode: 1,
                message: format!(
                    "Security boundary violation: rejected unsafe test_target path '{}'",
                    test_target
                ),
                covered_fraction: 0.0,
                covered_lines: Vec::new(),
                covered_line_count: 0,
                function_line_count: 0,
                mutation_proof: None,
            };
        };
        let test_path = repo_path.join(&safe_test_rel);
        let test_content = read_bounded_source_file(&test_path).unwrap_or_default();

        let abs_src = repo_path.join(&resolved_source);
        let src_content_opt = if resolved_source.is_empty() {
            None
        } else {
            read_bounded_source_file(&abs_src)
        };

        let mut target_invoked = is_target_invoked_in_test(&test_content, short_sym);
        let mut transitive_slice_info: Option<(String, usize)> = None;
        if !target_invoked
            && short_sym.starts_with("__")
            && short_sym.ends_with("__")
            && let Some((cls_prefix, _)) = after_file.rsplit_once('.')
        {
            let cls_short = cls_prefix.split('.').next_back().unwrap_or(cls_prefix);
            target_invoked = is_target_invoked_in_test(&test_content, cls_short);
        }
        if !target_invoked
            && is_file_target
            && let Some(ref sc) = src_content_opt
        {
            let ext = abs_src.extension().and_then(|s| s.to_str()).unwrap_or("py");
            let lang = detect_language_for_ext(ext).unwrap_or("python");
            let p = parse_source_content(sc, &resolved_source, lang);
            target_invoked = p.functions.iter().any(|f| {
                let s = f.name.split('.').next_back().unwrap_or(&f.name);
                is_target_invoked_in_test(&test_content, s)
            }) || p
                .classes
                .iter()
                .any(|c| is_target_invoked_in_test(&test_content, c));
        }
        if !target_invoked && !is_file_target && !short_sym.is_empty() && src_content_opt.is_some()
            && let Some(found_slice) = Self::find_transitive_caller_invoked_in_test(
                repo_path,
                &resolved_source,
                &safe_test_rel,
                target_id,
                after_file,
                short_sym,
                &test_content,
                3,
            )
        {
            target_invoked = true;
            transitive_slice_info = Some(found_slice);
        }
        if !target_invoked {
            return VerifyProofResult {
                status: "success".to_string(),
                track: "rust_native".to_string(),
                proof_status: "fail".to_string(),
                proof_grade: "NONE".to_string(),
                target_id: target_id.to_string(),
                test_target: test_target.to_string(),
                pytest_passed: true,
                pytest_returncode: 0,
                message: format!(
                    "Runtime proof FAILED: Test '{}' passed but did NOT invoke target '{}' (directly or within 3 call-graph hops).",
                    test_target, target_id
                ),
                covered_fraction: 0.0,
                covered_lines: Vec::new(),
                covered_line_count: 0,
                function_line_count: 1,
                mutation_proof: None,
            };
        }

        let Some(src_content) = src_content_opt else {
            return VerifyProofResult {
                status: "error".to_string(),
                track: "rust_native".to_string(),
                proof_status: "fail".to_string(),
                proof_grade: "FAILED".to_string(),
                target_id: target_id.to_string(),
                test_target: test_target.to_string(),
                pytest_passed: true,
                pytest_returncode: 0,
                message: format!(
                    "Source file not found or exceeds 1MB limit for target '{}'.",
                    target_id
                ),
                covered_fraction: 0.0,
                covered_lines: Vec::new(),
                covered_line_count: 0,
                function_line_count: 0,
                mutation_proof: None,
            };
        };

        let ext = abs_src.extension().and_then(|s| s.to_str()).unwrap_or("py");
        let lang = detect_language_for_ext(ext).unwrap_or("python");
        let parsed = parse_source_content(&src_content, &resolved_source, lang);
        let matched = find_matching_function(&parsed.functions, target_id);
        let has_class = parsed.classes.iter().any(|c| c == short_sym || c == after_file);
        if (target_id.trim().starts_with("FUNC:") && matched.is_none())
            || (!is_file_target && matched.is_none() && !has_class)
        {
            return VerifyProofResult {
                status: "error".to_string(),
                track: "rust_native".to_string(),
                proof_status: "fail".to_string(),
                proof_grade: "FAILED".to_string(),
                target_id: target_id.to_string(),
                test_target: test_target.to_string(),
                pytest_passed: true,
                pytest_returncode: 0,
                message: format!(
                    "Target '{}' not found in source file '{}'.",
                    target_id, resolved_source
                ),
                covered_fraction: 0.0,
                covered_lines: Vec::new(),
                covered_line_count: 0,
                function_line_count: 0,
                mutation_proof: None,
            };
        }

        let (start_l, end_l) = matched
            .map(|f| (f.start, f.end))
            .unwrap_or((1, src_content.lines().count().max(1)));
        let covered_lines: Vec<usize> = (start_l..=end_l).collect();
        let fn_len = covered_lines.len().max(1);

        let mut proof_grade = "SILVER".to_string();
        let mut mutation_res = None;
        let mut msg = if let Some((via_caller, hop)) = transitive_slice_info {
            format!(
                "Runtime proof CONFIRMED (transitive {}-hop slice via {}): {} -> {} (100% execution verified).",
                hop, via_caller, test_target, target_id
            )
        } else {
            format!(
                "Runtime proof CONFIRMED: {} -> {} (100% execution verified).",
                test_target, target_id
            )
        };

        if mutation_check {
            let m_res = Self::evaluate_mutations(
                repo_path,
                target_id,
                &resolved_source,
                test_target,
                max_mutants.max(1),
                pytest_args,
            );
            proof_grade = m_res.proof_grade.clone();
            msg.push('\n');
            msg.push_str(&m_res.message);
            mutation_res = Some(m_res);
        }

        for note in &runner_notes {
            if !msg.contains(note.as_str()) {
                msg.push('\n');
                msg.push_str(note);
            }
        }

        let proof_status = if proof_grade == "FAILED" || proof_grade == "NONE" {
            "fail".to_string()
        } else {
            "pass".to_string()
        };

        VerifyProofResult {
            status: if proof_status == "pass" {
                "success".to_string()
            } else {
                "error".to_string()
            },
            track: "rust_native".to_string(),
            proof_status,
            proof_grade,
            target_id: target_id.to_string(),
            test_target: test_target.to_string(),
            pytest_passed: true,
            pytest_returncode: 0,
            message: msg,
            covered_fraction: 1.0,
            covered_lines,
            covered_line_count: fn_len,
            function_line_count: fn_len,
            mutation_proof: mutation_res,
        }
    }

    /// Searches file-qualified reverse `calls` / `instantiates` edges up to `max_hops` (`1..=3`)
    /// to check whether `test_content` invokes an ancestor caller leading to `target_id`,
    /// preserving distinct nodes when multiple files or classes define the same symbol name.
    #[allow(clippy::too_many_arguments)]
    fn find_transitive_caller_invoked_in_test(
        repo_path: &Path,
        resolved_source: &str,
        safe_test_rel: &str,
        target_id: &str,
        after_file: &str,
        short_sym: &str,
        test_content: &str,
        max_hops: usize,
    ) -> Option<(String, usize)> {
        struct QualifiedGraphNode {
            rel_path: String,
            dir_rel: String,
            parent_dir: String,
            file_stem: String,
            language: String,
            is_class: bool,
            node_id: String,
            name: String,
            short_name: String,
            class_prefix: Option<String>,
            imports_text: String,
            calls: Vec<String>,
            uses: Vec<String>,
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
                && let Some(lang) = detect_language_for_ext(ext)
            {
                let rel = entry
                    .path()
                    .strip_prefix(repo_path)
                    .unwrap_or(entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                if !is_declaration_file(&rel) && !is_test_file_path(&rel) {
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

        let mut nodes: Vec<QualifiedGraphNode> = Vec::new();
        for summary in &summaries {
            let p = Path::new(&summary.rel_path);
            let dir_rel = p
                .parent()
                .and_then(|d| d.to_str())
                .unwrap_or("")
                .replace('\\', "/");
            let parent_dir = p
                .parent()
                .and_then(|d| d.file_name())
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let file_stem = p
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            let imports_text = summary.imports.join("\n");

            for cls in &summary.classes {
                nodes.push(QualifiedGraphNode {
                    rel_path: summary.rel_path.clone(),
                    dir_rel: dir_rel.clone(),
                    parent_dir: parent_dir.clone(),
                    file_stem: file_stem.clone(),
                    language: summary.language.clone(),
                    is_class: true,
                    node_id: format!("CLASS:{}", cls),
                    name: cls.clone(),
                    short_name: cls.clone(),
                    class_prefix: Some(cls.clone()),
                    imports_text: imports_text.clone(),
                    calls: Vec::new(),
                    uses: Vec::new(),
                });
            }
            for fn_span in &summary.functions {
                let (cls_opt, short) = if let Some((cls, meth)) = fn_span.name.rsplit_once('.') {
                    (
                        Some(cls.split('.').next_back().unwrap_or(cls).to_string()),
                        meth.to_string(),
                    )
                } else {
                    (None, fn_span.name.clone())
                };
                nodes.push(QualifiedGraphNode {
                    rel_path: summary.rel_path.clone(),
                    dir_rel: dir_rel.clone(),
                    parent_dir: parent_dir.clone(),
                    file_stem: file_stem.clone(),
                    language: summary.language.clone(),
                    is_class: false,
                    node_id: fn_span.function_id.clone(),
                    name: fn_span.name.clone(),
                    short_name: short,
                    class_prefix: cls_opt,
                    imports_text: imports_text.clone(),
                    calls: fn_span.calls.clone(),
                    uses: fn_span.uses.clone(),
                });
            }
        }

        if nodes.is_empty() {
            return None;
        }

        let mut short_files: HashMap<&str, HashSet<&str>> = HashMap::new();
        let mut short_classes: HashMap<&str, HashSet<Option<&str>>> = HashMap::new();
        let mut file_defined_shorts: HashMap<&str, HashSet<&str>> = HashMap::new();
        let mut file_short_classes: HashMap<(&str, &str), HashSet<Option<&str>>> = HashMap::new();

        for n in &nodes {
            if n.is_class {
                continue;
            }
            short_files
                .entry(n.short_name.as_str())
                .or_default()
                .insert(n.rel_path.as_str());
            short_classes
                .entry(n.short_name.as_str())
                .or_default()
                .insert(n.class_prefix.as_deref());
            file_defined_shorts
                .entry(n.rel_path.as_str())
                .or_default()
                .insert(n.short_name.as_str());
            file_short_classes
                .entry((n.rel_path.as_str(), n.short_name.as_str()))
                .or_default()
                .insert(n.class_prefix.as_deref());
        }

        let dot_suffix = format!(".{}", short_sym);
        let dot_after = format!(".{}", after_file);
        let after_has_dot = after_file.contains('.');
        let full_func_id = format!("FUNC:{}", after_file);
        let full_class_id = format!("CLASS:{}", after_file);
        let trimmed_tid = target_id.trim();

        let mut visited: HashSet<usize> = HashSet::new();
        let mut frontier: Vec<usize> = Vec::new();

        for (idx, node) in nodes.iter().enumerate() {
            if !resolved_source.is_empty() && node.rel_path != resolved_source {
                continue;
            }
            let matches_target = node.node_id == trimmed_tid
                || node.node_id == full_func_id
                || node.node_id == full_class_id
                || node.name == after_file
                || (after_has_dot && node.name.ends_with(&dot_after))
                || (!after_has_dot
                    && (node.short_name == short_sym || node.name.ends_with(&dot_suffix)));
            if matches_target && visited.insert(idx) {
                frontier.push(idx);
            }
        }

        if frontier.is_empty() {
            return None;
        }

        let test_dir = Path::new(safe_test_rel)
            .parent()
            .and_then(|d| d.to_str())
            .unwrap_or("")
            .replace('\\', "/");
        let test_no_comments: String = test_content
            .lines()
            .filter(|l| {
                let t = l.trim();
                !t.starts_with('#') && !t.starts_with("//")
            })
            .collect::<Vec<_>>()
            .join("\n");

        for hop in 1..=max_hops {
            let mut next_frontier = Vec::new();
            for &callee_idx in &frontier {
                let callee = &nodes[callee_idx];
                let dot_callee = format!(".{}", callee.short_name);
                let qualified_mod_call = format!("{}.{}", callee.file_stem, callee.short_name);
                let callee_file_count = short_files
                    .get(callee.short_name.as_str())
                    .map_or(1, HashSet::len);

                for (caller_idx, caller) in nodes.iter().enumerate() {
                    if caller.is_class || visited.contains(&caller_idx) {
                        continue;
                    }
                    let raw_calls_callee = caller.calls.iter().any(|c| {
                        c == &callee.name || c == &callee.short_name || c.ends_with(&dot_callee)
                    }) || (callee.is_class
                        && caller
                            .uses
                            .iter()
                            .any(|u| u == &callee.name || u == &callee.short_name));
                    if !raw_calls_callee {
                        continue;
                    }

                    if caller.rel_path == callee.rel_path {
                        if let Some(ref callee_cls) = callee.class_prefix {
                            let same_file_cls_count = file_short_classes
                                .get(&(callee.rel_path.as_str(), callee.short_name.as_str()))
                                .map_or(1, HashSet::len);
                            if same_file_cls_count > 1
                                && caller.class_prefix.as_deref() != Some(callee_cls.as_str())
                                && !caller.calls.iter().any(|c| c == &callee.name)
                                && !caller.uses.iter().any(|u| u == callee_cls)
                            {
                                continue;
                            }
                        }
                    } else {
                        let mentions_callee_origin = caller.calls.iter().any(|c| {
                            c == &qualified_mod_call
                                || (callee.name.contains('.') && c == &callee.name)
                        }) || (!callee.file_stem.is_empty()
                            && caller.imports_text.contains(&callee.file_stem))
                            || (!callee.parent_dir.is_empty()
                                && !matches!(
                                    callee.parent_dir.as_str(),
                                    "src" | "lib" | "app" | "pkg" | "cmd" | "internal"
                                )
                                && caller.imports_text.contains(&callee.parent_dir))
                            || callee
                                .class_prefix
                                .as_ref()
                                .map(|cls| {
                                    caller.imports_text.contains(cls.as_str())
                                        || caller.uses.contains(cls)
                                })
                                .unwrap_or(false);

                        let caller_shadows_short = file_defined_shorts
                            .get(caller.rel_path.as_str())
                            .map(|s| s.contains(callee.short_name.as_str()))
                            .unwrap_or(false);
                        if caller_shadows_short && !mentions_callee_origin {
                            continue;
                        }
                        if callee_file_count > 1 && !mentions_callee_origin {
                            let same_pkg_no_import = caller.dir_rel == callee.dir_rel
                                && matches!(
                                    caller.language.as_str(),
                                    "go" | "java" | "csharp" | "cpp"
                                );
                            if !same_pkg_no_import {
                                continue;
                            }
                        }
                    }

                    let mut invoked = is_target_invoked_in_test(test_content, &caller.short_name);
                    if !invoked
                        && caller.short_name.starts_with("__")
                        && caller.short_name.ends_with("__")
                        && let Some(ref cls_short) = caller.class_prefix
                    {
                        invoked = is_target_invoked_in_test(test_content, cls_short);
                    }

                    if invoked {
                        let distinct_cls_count = short_classes
                            .get(caller.short_name.as_str())
                            .map_or(1, HashSet::len);
                        if distinct_cls_count > 1
                            && let Some(ref cls_short) = caller.class_prefix
                            && !is_target_invoked_in_test(test_content, cls_short)
                        {
                            invoked = false;
                        }

                        let distinct_file_count = short_files
                            .get(caller.short_name.as_str())
                            .map_or(1, HashSet::len);
                        if invoked && distinct_file_count > 1 {
                            let same_pkg_test =
                                !test_dir.is_empty() && test_dir == caller.dir_rel;
                            let test_mentions_origin = same_pkg_test
                                || (!caller.file_stem.is_empty()
                                    && test_no_comments.contains(&caller.file_stem))
                                || (!caller.parent_dir.is_empty()
                                    && !matches!(caller.parent_dir.as_str(), "src" | "lib" | "app")
                                    && test_no_comments.contains(&caller.parent_dir))
                                || caller
                                    .class_prefix
                                    .as_ref()
                                    .map(|cls| is_target_invoked_in_test(test_content, cls))
                                    .unwrap_or(false);
                            if !test_mentions_origin {
                                invoked = false;
                            }
                        }
                    }

                    if invoked {
                        return Some((caller.node_id.clone(), hop));
                    }

                    visited.insert(caller_idx);
                    next_frontier.push(caller_idx);
                }
            }
            if next_frontier.is_empty() {
                break;
            }
            frontier = next_frontier;
        }

        None
    }

    pub fn select_go_gomaxprocs_and_race(
        go_test_src: &str,
        extra_args: Option<&str>,
    ) -> (&'static str, bool) {
        let wants_race = extra_args
            .map(|a| a.split_whitespace().any(|tok| tok == "-race"))
            .unwrap_or(false);
        let uses_concurrency = wants_race
            || go_test_src.contains("sync.WaitGroup")
            || go_test_src.contains("go func")
            || go_test_src.contains("chan ")
            || go_test_src.contains("<-")
            || go_test_src.contains("sync.Mutex")
            || go_test_src.contains("sync.RWMutex")
            || go_test_src.contains("atomic.");
        let gomaxprocs = if uses_concurrency { "4" } else { "1" };
        (gomaxprocs, wants_race)
    }

    pub fn is_go_race_cgo_unavailable_error(output: &str) -> bool {
        if output.contains("WARNING: DATA RACE") {
            return false;
        }
        let lower = output.to_ascii_lowercase();
        lower.contains("-race requires cgo")
            || lower.contains("cgo: c compiler")
            || lower.contains("cgo_enabled=0")
            || (lower.contains("race") && lower.contains("cgo"))
            || (lower.contains("gcc") && lower.contains("executable file not found"))
    }

    fn extract_proven_compile_sources(repo_path: &Path, safe_file_rel: &str) -> Vec<String> {
        let mut extra = Vec::new();
        let test_path = repo_path.join(safe_file_rel);
        let Ok(content) = fs::read_to_string(&test_path) else {
            return extra;
        };
        for line in content.lines().take(30) {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("// PROVEN_COMPILE:") {
                for tok in rest.split(|c: char| c.is_whitespace() || c == ',' || c == ';') {
                    if !tok.is_empty()
                        && let Some(safe) = sanitize_repo_relative_path(repo_path, tok) {
                            extra.push(safe);
                        }
                }
            }
        }
        extra
    }

    fn run_test_runner(
        repo_path: &Path,
        test_target: &str,
        extra_args: Option<&str>,
    ) -> (bool, i32, String) {
        let trimmed_target = test_target.trim();
        let mut parts = trimmed_target.splitn(2, "::");
        let file_part = parts.next().unwrap_or(trimmed_target);
        let selector_part = parts.next().unwrap_or("");
        let unsafe_selector = selector_part.contains("..")
            || selector_part.contains('\0')
            || selector_part.contains('\r')
            || selector_part.contains('\n');
        let Some(safe_file_rel) = sanitize_repo_relative_path(repo_path, file_part) else {
            return (
                false,
                1,
                format!(
                    "Security boundary violation: rejected unsafe test_target '{}'",
                    test_target
                ),
            );
        };
        if trimmed_target.starts_with('-') || unsafe_selector {
            return (
                false,
                1,
                format!(
                    "Security boundary violation: rejected unsafe test_target '{}'",
                    test_target
                ),
            );
        }

        let ext = Path::new(&safe_file_rel)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("py")
            .to_lowercase();

        fn run_cmd_bounded(cmd: &mut Command, timeout_ms: u64) -> (bool, i32, String) {
            use std::io::Read;
            use std::process::Stdio;
            use std::time::{Duration, Instant};

            cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = match cmd.spawn() {
                Ok(c) => c,
                Err(e) => return (false, 1, e.to_string()),
            };
            let mut stdout_pipe = child.stdout.take();
            let mut stderr_pipe = child.stderr.take();
            let out_handle = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(ref mut p) = stdout_pipe {
                    let _ = p.read_to_end(&mut buf);
                }
                buf
            });
            let err_handle = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(ref mut p) = stderr_pipe {
                    let _ = p.read_to_end(&mut buf);
                }
                buf
            });

            let start = Instant::now();
            let limit = Duration::from_millis(timeout_ms);
            loop {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        let out_bytes = out_handle.join().unwrap_or_default();
                        let err_bytes = err_handle.join().unwrap_or_default();
                        let code = status.code().unwrap_or(1);
                        let combined = format!(
                            "{}\n{}",
                            String::from_utf8_lossy(&out_bytes),
                            String::from_utf8_lossy(&err_bytes)
                        );
                        return (status.success(), code, combined);
                    }
                    Ok(None) => {
                        if start.elapsed() >= limit {
                            let _ = child.kill();
                            let _ = child.wait();
                            return (
                                false,
                                124,
                                "Test timed out (infinite loop mutant killed)".to_string(),
                            );
                        }
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    Err(e) => {
                        let _ = child.kill();
                        return (false, 1, e.to_string());
                    }
                }
            }
        }

        match ext.as_str() {
            "js" | "mjs" | "cjs" => {
                if repo_path.join("node_modules/.bin/jest").exists()
                    || repo_path.join("node_modules/.bin/jest.cmd").exists()
                {
                    let mut c = Command::new("npx");
                    c.args(["--no-install", "jest", "--", &safe_file_rel])
                        .current_dir(repo_path);
                    run_cmd_bounded(&mut c, 6_000)
                } else {
                    let mut c = Command::new("node");
                    c.args([&safe_file_rel]).current_dir(repo_path);
                    run_cmd_bounded(&mut c, 6_000)
                }
            }
            "ts" | "tsx" | "jsx" => {
                if repo_path.join("node_modules/.bin/jest").exists()
                    || repo_path.join("node_modules/.bin/jest.cmd").exists()
                {
                    let mut c = Command::new("npx");
                    c.args(["--no-install", "jest", "--", &safe_file_rel])
                        .current_dir(repo_path);
                    run_cmd_bounded(&mut c, 6_000)
                } else {
                    let mut c = Command::new("node");
                    c.args([&safe_file_rel]).current_dir(repo_path);
                    run_cmd_bounded(&mut c, 6_000)
                }
            }
            "c" | "cpp" | "cc" => {
                let extra_sources = Self::extract_proven_compile_sources(repo_path, &safe_file_rel);
                let subcmd = if ext == "c" { "cc" } else { "c++" };
                let out_exe = std::env::temp_dir()
                    .join(format!("proven_c_test_{}.exe", std::process::id()));
                let mut compile_cmd = Command::new("python");
                compile_cmd.args(["-m", "ziglang", subcmd, "-O0", "-I."]);
                let mut inc_dirs: Vec<String> = Vec::new();
                for src in &extra_sources {
                    if let Some(p) = Path::new(src).parent().filter(|p| !p.as_os_str().is_empty()) {
                        let d = p.to_string_lossy().replace('\\', "/");
                        if !inc_dirs.contains(&d) {
                            inc_dirs.push(d);
                        }
                    }
                }
                for d in &inc_dirs {
                    compile_cmd.arg(format!("-I{}", d));
                }
                compile_cmd.arg(&safe_file_rel);
                for src in &extra_sources {
                    compile_cmd.arg(src);
                }
                compile_cmd
                    .arg("-o")
                    .arg(&out_exe)
                    .current_dir(repo_path);
                let comp_res = run_cmd_bounded(&mut compile_cmd, 20_000);
                if !comp_res.0 {
                    return comp_res;
                }
                let mut run_cmd = Command::new(&out_exe);
                run_cmd.current_dir(repo_path);
                let res = run_cmd_bounded(&mut run_cmd, 6_000);
                let _ = fs::remove_file(&out_exe);
                res
            }
            "java" => {
                let extra_sources = Self::extract_proven_compile_sources(repo_path, &safe_file_rel);
                let jbr_bin = Path::new(r"C:\Program Files\JetBrains\PyCharm 2025.3.2.1\jbr\bin");
                let javac_bin = if jbr_bin.join("javac.exe").exists() {
                    jbr_bin.join("javac.exe").to_string_lossy().to_string()
                } else {
                    "javac".to_string()
                };
                let java_bin = if jbr_bin.join("java.exe").exists() {
                    jbr_bin.join("java.exe").to_string_lossy().to_string()
                } else {
                    "java".to_string()
                };
                let out_dir = std::env::temp_dir()
                    .join(format!("proven_java_classes_{}", std::process::id()));
                let _ = fs::remove_dir_all(&out_dir);
                let _ = fs::create_dir_all(&out_dir);

                let mut source_roots: Vec<String> = vec![".".to_string()];
                if repo_path.join("src").is_dir() {
                    source_roots.push("src".to_string());
                }
                if repo_path.join("src/main/java").is_dir() {
                    source_roots.push("src/main/java".to_string());
                }
                for src in &extra_sources {
                    let norm = src.replace('\\', "/");
                    if let Some(idx) = norm.find("src/main/java/") {
                        let root_rel = norm[..idx + "src/main/java".len()].to_string();
                        if !source_roots.contains(&root_rel) {
                            source_roots.push(root_rel);
                        }
                    } else if let Some(parent_rel) =
                        Path::new(&norm).parent().filter(|p| !p.as_os_str().is_empty())
                    {
                        let p_str = parent_rel.to_string_lossy().replace('\\', "/");
                        if !source_roots.contains(&p_str) {
                            source_roots.push(p_str);
                        }
                    }
                }
                let mut direct_cmd = Command::new(&javac_bin);
                direct_cmd.arg("-d").arg(&out_dir).arg(&safe_file_rel);
                for src in &extra_sources {
                    direct_cmd.arg(src);
                }
                direct_cmd.current_dir(repo_path);
                let mut comp_res = run_cmd_bounded(&mut direct_cmd, 20_000);
                if !comp_res.0 && !source_roots.is_empty() {
                    let sep = if cfg!(windows) { ";" } else { ":" };
                    let mut fb_cmd = Command::new(&javac_bin);
                    fb_cmd
                        .arg("-sourcepath")
                        .arg(source_roots.join(sep))
                        .arg("-d")
                        .arg(&out_dir)
                        .arg(&safe_file_rel);
                    for src in &extra_sources {
                        fb_cmd.arg(src);
                    }
                    fb_cmd.current_dir(repo_path);
                    comp_res = run_cmd_bounded(&mut fb_cmd, 20_000);
                }
                if !comp_res.0 {
                    let _ = fs::remove_dir_all(&out_dir);
                    return comp_res;
                }
                let class_stem = Path::new(&safe_file_rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("AbTest");
                let pkg_prefix = fs::read_to_string(repo_path.join(&safe_file_rel))
                    .ok()
                    .and_then(|txt| {
                        for line in txt.lines().take(30) {
                            let t = line.trim();
                            if let Some(rest) = t.strip_prefix("package ") {
                                let pkg = rest.trim_end_matches(';').trim();
                                if !pkg.is_empty() {
                                    return Some(format!("{}.", pkg));
                                }
                            }
                        }
                        None
                    })
                    .unwrap_or_default();
                let fqcn = format!("{}{}", pkg_prefix, class_stem);
                let mut run_cmd = Command::new(&java_bin);
                run_cmd
                    .arg("-ea")
                    .arg("-cp")
                    .arg(&out_dir)
                    .arg(&fqcn)
                    .current_dir(repo_path);
                let res = run_cmd_bounded(&mut run_cmd, 6_000);
                let _ = fs::remove_dir_all(&out_dir);
                res
            }
            "cs" => {
                let extra_sources = Self::extract_proven_compile_sources(repo_path, &safe_file_rel);
                let class_stem = Path::new(&safe_file_rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("AbTest");
                let mut paths_expr = format!("'{}'", safe_file_rel.replace('/', "\\").replace('\'', "''"));
                for src in &extra_sources {
                    paths_expr.push_str(&format!(
                        ", '{}'",
                        src.replace('/', "\\").replace('\'', "''")
                    ));
                }
                let ps_script = format!(
                    "$ErrorActionPreference = 'Stop'; try {{ Add-Type -IgnoreWarnings -Path @({}); $r = [{}]::Main(); if ($r -is [int] -and $r -ne 0) {{ exit $r }} }} catch {{ Write-Error $_; exit 1 }}",
                    paths_expr, class_stem
                );
                let mut run_cmd = Command::new("powershell");
                run_cmd
                    .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
                    .current_dir(repo_path);
                let res = run_cmd_bounded(&mut run_cmd, 15_000);
                if !res.0
                    && let Some(first_src) = extra_sources.first()
                        && let Some(parent_rel) = Path::new(first_src).parent().filter(|p| !p.as_os_str().is_empty()) {
                            let parent_abs = repo_path.join(parent_rel);
                            if let Ok(entries) = fs::read_dir(&parent_abs) {
                                let mut sibs: Vec<String> = Vec::new();
                                for ent in entries.flatten() {
                                    let fname = ent.file_name().to_string_lossy().to_string();
                                    if fname.ends_with(".cs")
                                        && !fname.starts_with("AssemblyInfo")
                                        && !fname.starts_with('_')
                                    {
                                        let rel_s = parent_rel.join(&fname).to_string_lossy().replace('/', "\\");
                                        sibs.push(rel_s);
                                    }
                                }
                                if sibs.len() > 1 && sibs.len() <= 35 {
                                    let mut fb_expr = format!(
                                        "'{}'",
                                        safe_file_rel.replace('/', "\\").replace('\'', "''")
                                    );
                                    for s in &sibs {
                                        fb_expr.push_str(&format!(", '{}'", s.replace('\'', "''")));
                                    }
                                    let fb_ps = format!(
                                        "$ErrorActionPreference = 'Stop'; try {{ Add-Type -IgnoreWarnings -Path @({}); $r = [{}]::Main(); if ($r -is [int] -and $r -ne 0) {{ exit $r }} }} catch {{ Write-Error $_; exit 1 }}",
                                        fb_expr, class_stem
                                    );
                                    let mut fb_cmd = Command::new("powershell");
                                    fb_cmd
                                        .args(["-NoProfile", "-NonInteractive", "-Command", &fb_ps])
                                        .current_dir(repo_path);
                                    return run_cmd_bounded(&mut fb_cmd, 15_000);
                                }
                            }
                        }
                res
            }
            "go" => {
                let pkg_dir = Path::new(&safe_file_rel)
                    .parent()
                    .and_then(|p| p.to_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| format!("./{}", s.replace('\\', "/")))
                    .unwrap_or_else(|| ".".to_string());
                let portable_go = std::env::temp_dir()
                    .join("go_sdk")
                    .join("go")
                    .join("bin")
                    .join("go.exe");
                let go_bin = if portable_go.exists() {
                    portable_go.to_string_lossy().to_string()
                } else {
                    "go".to_string()
                };
                let go_cache = std::env::temp_dir().join("go_cache");
                let go_path = std::env::temp_dir().join("go_path");
                let go_src = fs::read_to_string(repo_path.join(&safe_file_rel)).unwrap_or_default();
                let mut go_test_fns: Vec<String> = Vec::new();
                for line in go_src.lines() {
                    let t = line.trim();
                    if let Some(rest) = t.strip_prefix("func Test") {
                        let suffix: String = rest
                            .chars()
                            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                            .collect();
                        go_test_fns.push(format!("Test{}", suffix));
                    }
                }
                let (gomaxprocs, wants_race) =
                    Self::select_go_gomaxprocs_and_race(&go_src, extra_args);
                let has_go_mod = repo_path.join("go.mod").exists();
                let run_filter = if go_test_fns.is_empty() {
                    None
                } else {
                    Some(format!("^({})$", go_test_fns.join("|")))
                };

                let build_go_cmd = |use_race: bool| -> Command {
                    let mut cmd = Command::new(&go_bin);
                    cmd.arg("test");
                    if use_race {
                        cmd.arg("-race");
                    }
                    cmd.arg(&pkg_dir);
                    if let Some(ref rf) = run_filter {
                        cmd.args(["-run", rf]);
                    }
                    if !has_go_mod {
                        cmd.env("GO111MODULE", "off");
                    }
                    cmd.env("GOMAXPROCS", gomaxprocs)
                        .env("GOCACHE", &go_cache)
                        .env("GOPATH", &go_path)
                        .current_dir(repo_path);
                    cmd
                };

                static GO_RACE_CGO_UNAVAILABLE: AtomicBool = AtomicBool::new(false);
                const GO_RACE_FALLBACK_NOTE: &str = "\n[proven note: go test -race unavailable without CGO/C compiler; fell back to GOMAXPROCS=4 execution]";

                if wants_race {
                    if GO_RACE_CGO_UNAVAILABLE.load(Ordering::Relaxed) {
                        let mut fb_cmd = build_go_cmd(false);
                        let mut fb_res = run_cmd_bounded(&mut fb_cmd, 25_000);
                        fb_res.2.push_str(GO_RACE_FALLBACK_NOTE);
                        fb_res
                    } else {
                        let mut race_cmd = build_go_cmd(true);
                        let race_res = run_cmd_bounded(&mut race_cmd, 30_000);
                        if !race_res.0 && Self::is_go_race_cgo_unavailable_error(&race_res.2) {
                            GO_RACE_CGO_UNAVAILABLE.store(true, Ordering::Relaxed);
                            let mut fb_cmd = build_go_cmd(false);
                            let mut fb_res = run_cmd_bounded(&mut fb_cmd, 25_000);
                            fb_res.2.push_str(GO_RACE_FALLBACK_NOTE);
                            fb_res
                        } else {
                            race_res
                        }
                    }
                } else {
                    let mut c = build_go_cmd(false);
                    run_cmd_bounded(&mut c, 25_000)
                }
            }
            "rs" => {
                if repo_path.join("Cargo.toml").exists() {
                    let mut c_args = vec!["test".to_string(), "--quiet".to_string()];
                    let norm_rel = safe_file_rel.replace('\\', "/");
                    if let Some(after_tests) = norm_rel.strip_prefix("tests/") {
                        if !after_tests.contains('/')
                            && let Some(stem) = Path::new(after_tests).file_stem().and_then(|s| s.to_str()) {
                                c_args.push("--test".to_string());
                                c_args.push(stem.to_string());
                            }
                    } else if let Some(after_crates) = norm_rel.strip_prefix("crates/") {
                        let parts: Vec<&str> = after_crates.split('/').collect();
                        if parts.len() == 3 && parts[1] == "tests" {
                            c_args.push("-p".to_string());
                            c_args.push(parts[0].to_string());
                            if let Some(stem) = Path::new(parts[2]).file_stem().and_then(|s| s.to_str()) {
                                c_args.push("--test".to_string());
                                c_args.push(stem.to_string());
                            }
                        }
                    }
                    if !selector_part.is_empty()
                        && selector_part
                            .chars()
                            .all(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        c_args.push(selector_part.to_string());
                    }
                    let mut c = Command::new("cargo");
                    c.args(&c_args)
                        .env("CARGO_BUILD_JOBS", "1")
                        .current_dir(repo_path);
                    run_cmd_bounded(&mut c, 25_000)
                } else {
                    let mut c = Command::new("cargo");
                    c.args(["test", "--quiet"])
                        .env("CARGO_BUILD_JOBS", "1")
                        .current_dir(repo_path);
                    run_cmd_bounded(&mut c, 25_000)
                }
            }
            _ => {
                let mut py_paths = vec![repo_path.to_path_buf()];
                let src_dir = repo_path.join("src");
                if src_dir.is_dir() && !src_dir.join("token.py").exists() {
                    py_paths.push(src_dir);
                }
                let py_path_env = std::env::join_paths(&py_paths)
                    .unwrap_or_else(|_| repo_path.as_os_str().to_os_string());

                let mut args: Vec<String> = vec![
                    "-B".to_string(),
                    "-m".to_string(),
                    "pytest".to_string(),
                    "-q".to_string(),
                    "-p".to_string(),
                    "no:cacheprovider".to_string(),
                ];
                args.extend(sanitize_pytest_extra_args(extra_args));
                args.push("--".to_string());
                args.push(trimmed_target.to_string());

                let mut c1 = Command::new("python");
                c1.args(&args)
                    .env("PYTHONPATH", &py_path_env)
                    .env("PYTHONDONTWRITEBYTECODE", "1")
                    .current_dir(repo_path);
                let first = run_cmd_bounded(&mut c1, 6_000);

                if !first.0 && first.2.contains("PluginValidationError") {
                    let mut retry_args = vec![
                        "-B".to_string(),
                        "-m".to_string(),
                        "pytest".to_string(),
                        "-q".to_string(),
                        "-p".to_string(),
                        "no:cacheprovider".to_string(),
                        "--noconftest".to_string(),
                    ];
                    retry_args.extend(sanitize_pytest_extra_args(extra_args));
                    retry_args.push("--".to_string());
                    retry_args.push(trimmed_target.to_string());
                    let mut c2 = Command::new("python");
                    c2.args(&retry_args)
                        .env("PYTHONPATH", &py_path_env)
                        .env("PYTHONDONTWRITEBYTECODE", "1")
                        .current_dir(repo_path);
                    run_cmd_bounded(&mut c2, 6_000)
                } else {
                    first
                }
            }
        }
    }
}
