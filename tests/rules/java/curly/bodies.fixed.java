class A {
  void f(boolean x, int[] xs) {
    if (x) { return; }
    if (x) {
      g();
    } else { g(); }
    if (x) {
      g();
    } else if (!x) {
      g();
    } else {
      g();
    }
    for (int i = 0; i < 3; i++) { g(); }
    for (int v : xs) { g(); }
    while (x) { g(); }
    do { g(); } while (x);
    for (int v : xs) {
      g();
    }
  }
}
