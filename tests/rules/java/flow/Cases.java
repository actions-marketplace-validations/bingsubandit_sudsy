class A {
    int a(int x) {
        return 1;
        b();
//      ^^^^ unreachable-code
        c();
    }
    void d(int[] xs, boolean y) {
        for (int i : xs) { continue; e(); }
//                                   ^^^^ unreachable-code
        for (;;) { if (y) { break; } }
        f();
        outer: while (true) { while (y) { break outer; } }
        g();
        while (true) { }
//                   ^^^ no-empty-block
        h();
//      ^^^^ unreachable-code
    }
    void i(int x) {
        try { return; } finally { j(); }
        k();
//      ^^^^ unreachable-code
    }
    void l(int x) {
        switch (x) {
            case 1:
                m();
            case 2:
//          ^ no-fallthrough
                n();
                break;
            case 3:
            case 4:
                o();
                // falls through
            default:
                p();
        }
    }
}
