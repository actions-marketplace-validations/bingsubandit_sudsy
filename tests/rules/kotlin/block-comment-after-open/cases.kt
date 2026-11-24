fun f(a: Int) {
    if (a > 1) {    /* big */
//                  ^^^^^^^^^ block-comment-after-open
        g()
    }
    if (a > 2) { // line comment
        g()
    }
    if (a > 3) { /* inline */ g() }
}
