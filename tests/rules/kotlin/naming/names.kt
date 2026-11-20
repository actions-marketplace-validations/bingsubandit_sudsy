const val maxCount = 1
//        ^^^^^^^^ property-naming
const val MAX_COUNT = 1
const val serialVersionUID = 1L
val Default = 1
val a_b = 1
//  ^^^ property-naming
var Global = 1

class A {
    val MAX = 1
//      ^^^ property-naming
    val ok = 1
    val _backing = 1
    val trailing_ = 1
//      ^^^^^^^^^ property-naming
    fun My_fun() {}
//      ^^^^^^ function-naming
    fun Factory(): A = A()
    fun my_fun2(): Int = 1
//      ^^^^^^^ function-naming
    fun `a test name`() {}
    override fun other_name() {}
}

object B {
    val LIMIT = 1
    val Mixed = 1
}

fun local() {
    val LOCAL = 1
//      ^^^^^ property-naming
}

@Composable
fun Screen() {}
