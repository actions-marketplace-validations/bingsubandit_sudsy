if (a) { b(); } else { if (c) { d(); } }
//                     ^^^^^^^^^^^^^^^ no-lonely-if
if (a) { b(); } else { if (c) { d(); } e(); }
if (a) { b(); } else if (c) { d(); }
function f(a) { if (a) { return 1; } else { return 2; } }
//                                        ^^^^^^^^^^^^^ no-else-return
function g(a) { if (a) { return 1; } else if (b) { return 2; } }
function h(a) { if (a) { return 1; } else if (b) { return 2; } else { return 3; } }
//                                                                  ^^^^^^^^^^^^^ no-else-return
function i(a) { if (a) { x(); } else { return 2; } }
var x = a ? true : false;
//      ^^^^^^^^^^^^^^^^ no-unneeded-ternary
var y = a ? false : true;
//      ^^^^^^^^^^^^^^^^ no-unneeded-ternary
var z = a ? b : false;
function j(a) { if (a) { if (b) { return 1; } else { return 2; } } else { return 3; } }
//                                                 ^^^^^^^^^^^^^ no-else-return
//                                                                      ^^^^^^^^^^^^^ no-else-return
