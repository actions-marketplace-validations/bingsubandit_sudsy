//! The code a preprocessor directive carries, read with the layers of the file (the tree for
//! formatting only).
//!
//! concept: none (glue: the facts of the directive payloads of C and C++)

use crate::deep;
use crate::generated::kind as k;
use crate::lexer::LTok;
use crate::structure;
use provider_api::adapter::Node;
use provider_api::directive_payload::{Directives, payload_nodes};

/// The nodes of the payloads of all directives, in opening order.
pub fn nodes(src: &str, toks: &[LTok]) -> Vec<Node> {
    let build = |body: &[LTok], condition: bool| {
        let nodes = structure::build(src, body);
        if condition {
            deep::build_expression(src, body, nodes)
        } else {
            deep::build_body(src, body, nodes)
        }
    };
    let directives = Directives {
        conditions: &["if", "elif"],
        define: "define",
        params: k::PAREN_GROUP,
        brackets: &[
            (k::LPAR, k::RPAR),
            (k::LSQB, k::RSQB),
            (k::LBRACE, k::RBRACE),
        ],
        build: &build,
    };
    payload_nodes(src, toks, &directives)
}
