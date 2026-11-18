//! The calls into Prism's C library: one parse that records the lexer's tokens (the library's lex
//! callback, which the Rust crate does not wrap) and the node tree (the library's visitor), with
//! nothing of the library's types leaving this file.
//!
//! concept: none (adapter: Prism FFI calls)

#![allow(unsafe_code)]

use ruby_prism_sys as sys;
use std::ffi::{CStr, c_char, c_void};

/// What one parse found.
pub struct Parsed {
    pub tokens: Vec<Token>,
    /// In the order of the C visitor, which is not always source order.
    pub nodes: Vec<Node>,
    pub errors: Vec<Error>,
}

pub struct Token {
    pub kind: u32,
    pub start: u32,
    pub end: u32,
}

pub struct Node {
    pub kind: u16,
    pub start: u32,
    pub end: u32,
}

pub struct Error {
    pub start: u32,
    pub len: u32,
    pub message: String,
}

/// `pm_token_t`
#[repr(C)]
struct RawToken {
    kind: u32,
    start: *const u8,
    end: *const u8,
}

type LexCallback = unsafe extern "C" fn(*mut sys::pm_parser_t, *mut RawToken, *mut c_void);
type VisitCallback = unsafe extern "C" fn(*const sys::pm_node_t, *mut c_void) -> u32;

/// `pm_visit_result_t::PM_VISIT_DESCEND`
const DESCEND: u32 = 1;

// Declared here because the Rust bindings leave them out; the library exports them.
unsafe extern "C" {
    fn pm_parser_lex_callback_set(
        parser: *mut sys::pm_parser_t,
        callback: LexCallback,
        data: *mut c_void,
    );
    fn pm_visit_node(
        node: *const sys::pm_node_t,
        visitor: VisitCallback,
        data: *mut c_void,
    ) -> bool;
    fn pm_token_type(kind: u32) -> *const c_char;
    fn pm_node_type(kind: u16) -> *const c_char;
}

struct Lex {
    base: *const u8,
    tokens: Vec<Token>,
}

unsafe extern "C" fn on_token(_: *mut sys::pm_parser_t, token: *mut RawToken, data: *mut c_void) {
    // SAFETY: `data` is the `Lex` of `parse` and `token` points into its source, both alive
    // for the whole call of `pm_parse`.
    let (lex, token) = unsafe { (&mut *data.cast::<Lex>(), &*token) };
    let offset = |p: *const u8| unsafe { p.offset_from(lex.base) } as u32;
    lex.tokens.push(Token {
        kind: token.kind,
        start: offset(token.start),
        end: offset(token.end),
    });
}

unsafe extern "C" fn on_node(node: *const sys::pm_node_t, data: *mut c_void) -> u32 {
    // SAFETY: `data` is the node vector of `parse`; `node` is a node of its tree.
    let (nodes, node) = unsafe { (&mut *data.cast::<Vec<Node>>(), &*node) };
    let (mut start, mut end) = (
        node.location.start,
        node.location.start + node.location.length,
    );
    if node.type_ == sys::pm_node_type::PM_BEGIN_NODE as u16 {
        // SAFETY: the type says the node is a `pm_begin_node_t`, whose clauses are null or nodes.
        let begin = unsafe { &*std::ptr::from_ref(node).cast::<sys::pm_begin_node_t>() };
        if begin.begin_keyword_loc.length == 0 {
            // The begin of a method or block body that has `rescue`: Prism spans it over the whole
            // method or block, `def` and `end` included. It is the clauses, up to the `end`.
            let clauses = [
                begin.statements.cast::<sys::pm_node_t>(),
                begin.rescue_clause.cast(),
                begin.else_clause.cast(),
                begin.ensure_clause.cast(),
            ];
            // SAFETY: each pointer is null or a node.
            let first = (clauses.iter().filter(|p| !p.is_null()))
                .map(|&p| unsafe { (*p).location.start })
                .min();
            start = first.unwrap_or(start);
            if begin.end_keyword_loc.length > 0 {
                end = begin.end_keyword_loc.start;
            }
        }
    }
    nodes.push(Node {
        kind: node.type_,
        start,
        end,
    });
    DESCEND
}

/// Parses `src` as a Ruby file of the library's newest syntax.
pub fn parse(src: &str) -> Parsed {
    let mut lex = Lex {
        base: src.as_ptr(),
        tokens: Vec::with_capacity(src.len() / 4),
    };
    let mut nodes = Vec::with_capacity(src.len() / 8);
    let mut errors = Vec::new();
    // SAFETY: the parser and its arena are used only inside this block and freed at its end; the
    // callbacks see `lex` and `nodes`, which outlive the calls; the source outlives the parser.
    unsafe {
        let arena = sys::pm_arena_new();
        let parser = sys::pm_parser_new(arena, src.as_ptr(), src.len(), std::ptr::null());
        pm_parser_lex_callback_set(parser, on_token, (&raw mut lex).cast());
        let root = sys::pm_parse(parser);
        pm_visit_node(root, on_node, (&raw mut nodes).cast());
        let mut error = sys::pm_parser_errors_first(parser);
        while !error.is_null() {
            let location = sys::pm_diagnostic_location(error);
            let message = CStr::from_ptr(sys::pm_diagnostic_message(error));
            errors.push(Error {
                start: location.start,
                len: location.length,
                message: message.to_string_lossy().into_owned(),
            });
            error = sys::pm_diagnostic_next(error);
        }
        sys::pm_parser_free(parser);
        sys::pm_arena_free(arena);
    }
    Parsed {
        tokens: lex.tokens,
        nodes,
        errors,
    }
}

/// The library's name of a token type: `COMMA`, `KEYWORD_IF`.
pub fn token_name(kind: u32) -> String {
    // SAFETY: the function returns a static string for every value.
    unsafe { CStr::from_ptr(pm_token_type(kind)) }
        .to_string_lossy()
        .into_owned()
}

/// The library's name of a node type: `PM_CALL_NODE`.
pub fn node_name(kind: u16) -> String {
    // SAFETY: as `token_name`.
    unsafe { CStr::from_ptr(pm_node_type(kind)) }
        .to_string_lossy()
        .into_owned()
}
