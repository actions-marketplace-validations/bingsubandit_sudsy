import b.B
//     ^ import-ordering
import a.A
//     ^ import-ordering

fun f(x: Int) = when (x) {
//              ^ function-signature
//              ^^^^^^^^^^ multiline-expression-wrapping
    1 -> {
        foo()
    }

    2 -> bar()
//     ^ when-entry-bracing
    else -> baz()
//        ^ when-entry-bracing
}

/* one line */
^^^^^^^^^^^^^^ no-single-line-block-comment
val a = 1
