class A(val a: Int,
//      ^ class-signature
//      ^ parameter-list-wrapping
        val b: Int) {
//               ^ trailing-comma-on-declaration-site
    fun foo(a: Int, b: String, c: Long, d: Double, e: Float, f: Short, g: Byte, h: Char, i: Boolean, j: Unit): Int { return 1 }
//          ^ function-signature
//          ^ parameter-list-wrapping
//                                                                                                                 ^ function-expression-body

    fun baz() = run {
//              ^ function-signature
//              ^^^^^ multiline-expression-wrapping
        1
    }

    val x = listOf(
//          ^^^^^^^ multiline-expression-wrapping
        1,
        2
//      ^ trailing-comma-on-call-site
    )

    fun bar(
        a: Int,
        b: Int
//           ^ trailing-comma-on-declaration-site
    ) = a + b

    fun ok(a: Int) = a

    fun sum(): Int { return 1 }
//                 ^ function-expression-body
}
