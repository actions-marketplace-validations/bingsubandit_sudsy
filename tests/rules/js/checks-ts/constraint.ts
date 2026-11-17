function f<T extends unknown>(x: T) {}
//         ^^^^^^^^^^^^^^^^^ no-unnecessary-type-constraint
function g<T extends string>(x: T) {}
let a = b?.c!;
//      ^^^^^ no-non-null-asserted-optional-chain
let d = (b?.c)!;
//      ^^^^^^^ no-non-null-asserted-optional-chain
let e = b!.c;
b?.c()!;
^^^^^^^ no-non-null-asserted-optional-chain
b?.c!.d;
