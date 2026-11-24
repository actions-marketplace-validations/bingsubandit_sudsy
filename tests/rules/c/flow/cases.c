int a(int x) {
  if (x) {
    return 1;
  }
  return 2;
#undef Foo
}
int b(int x) {
  return 1;
  x = 2;
//^^^^^^ unreachable-code
}
int c(int x) {
  if (x) goto out;
  return 1;
out:
  x = 3;
  return x;
}
int d(int x) {
  switch (x) {
#ifdef A
  case 1:
#endif
    break;
  default:
    return 3;
  }
  x = 4;
  return x;
}
int e(int x) {
  return x;
  FOO_POP();
}
int f(int x) {
  switch (x) {
  case 1:
    return 1;
  read_sysreg_case(X);
    x = 2;
    break;
  case 2:
    break;
    x = 3;
//  ^^^^^^ unreachable-code
  }
  return x;
}
