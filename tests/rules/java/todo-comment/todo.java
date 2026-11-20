class A {
  // TODO fix
//   ^^^^ todo-comment
  // todo: lower
//   ^^^^ todo-comment
  // TODO: good
  // fix TODO later
  // TODOS fix
  /* line one
     TODO second */
//   ^^^^ todo-comment
  /**
   * TODO javadoc
   */
  void f() {} // Todo trailing
//               ^^^^ todo-comment
}
