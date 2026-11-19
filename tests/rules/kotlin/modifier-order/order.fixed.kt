class A {
    public final fun f() = 1
    public override fun g() = 1
    private lateinit var x: String
    @Deprecated("x") private open fun h() = 1
    data class D(val a: Int)
    private suspend inline fun s() = 1
}
