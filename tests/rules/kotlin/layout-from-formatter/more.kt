fun f(x: Int) {
    val a = if (x > 0)
//          ^ multiline-if-else
//          ^ statement-wrapping
//          ^^^^^^^^^^ multiline-expression-wrapping
        1
//      ^ multiline-if-else
    else 2
//     ^ multiline-if-else
    val b = when (x) {
//          ^ statement-wrapping
//          ^^^^^^^^^^ multiline-expression-wrapping
        1 -> { println(1) }
//             ^ statement-wrapping
        else -> println(2)
//            ^ when-entry-bracing
    }
    foo(); bar()
    val c = someVeryLongReceiverName.firstFunctionCall(argumentNumberOne).secondFunctionCall(argumentNumberTwo).thirdFunctionCall(argumentNumberThree)
//          ^ chain-method-continuation
//          ^ statement-wrapping
    callTheFunction(argumentNumberOne, argumentNumberTwo, argumentNumberThree, argumentNumberFour, argumentNumberFive, argumentNumberSix, argumentNumberSeven)
//                  ^ argument-list-wrapping
//                  ^ statement-wrapping
}
