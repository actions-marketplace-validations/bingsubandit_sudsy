package a

func f(a bool) int {
	if a {
		return 1
	} else {
//      ^ indent-error-flow
		return 2
	}
}

func h(a, b bool) int {
	if a {
		return 1
	} else if b {
		return 2
	}
	if x := g(); x {
		return 1
	} else {
//      ^ indent-error-flow
		g()
	}
	if a {
		g()
	} else {
		return 3
	}
	return 0
}

func n(a, b bool) int {
	if a {
		if b {
			return 1
		} else {
//       ^ indent-error-flow
			return 2
		}
	} else {
		g()
	}
	return 0
}
