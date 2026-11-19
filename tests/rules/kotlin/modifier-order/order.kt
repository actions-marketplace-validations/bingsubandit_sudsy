class A {
    public final fun f() = 1
    override public fun g() = 1
//           ^^^^^^ modifier-order
    lateinit private var x: String
//           ^^^^^^^ modifier-order
    @Deprecated("x") open private fun h() = 1
//                        ^^^^^^^ modifier-order
    data class D(val a: Int)
    private suspend inline fun s() = 1
}
