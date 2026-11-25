package a

func used(a, b int) int { return a + b }

func unused(a, b int) int {
//             ^ unused-parameter
	return a
}

func assigned(a int) { a = 1 }
func blank(_ int, _x string) {}

func lit() func(int) int {
	return func(n int) int { return 1 }
//           ^ unused-parameter
}

func (r *T) method(x int) (n int, err error) {
//                 ^ unused-parameter
	return
}

func shadowed(a int) {
//            ^ unused-parameter
	if true {
		a := 1
		_ = a
	}
}

type F func(a int)
func noBody(a int)

func initReads(ctx int) {
	ctx, c := wrap(ctx)
	_, _ = ctx, c
	if true {
		ctx := ctx + 1
		_ = ctx
	}
}

func readBefore(err error) {
	if err != nil {
		return
	}
	x, err := wrap(2)
	_, _ = x, err
}

func redeclared(ml int) {
	ml, err := wrap(1)
	_, _ = ml, err
}

func newBool(pos int, true bool) bool {
//           ^^^ unused-parameter
	return true
}

func deferred() {
	defer func(iota int) { sink = iota }(1)
}
