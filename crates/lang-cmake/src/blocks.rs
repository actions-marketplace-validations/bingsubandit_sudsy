//! The block commands of CMake: an opener, its arms and its closer.
//!
//! concept: none (data of the shared pairing of core_syntax::block_pair)

use core_syntax::block_pair::Block;

pub static BLOCKS: [Block; 6] = [
    Block {
        open: "if",
        arms: &["elseif", "else"],
        last_arm: "else",
        close: "endif",
    },
    Block::plain("foreach", "endforeach"),
    Block::plain("while", "endwhile"),
    Block::plain("function", "endfunction"),
    Block::plain("macro", "endmacro"),
    Block::plain("block", "endblock"),
];
