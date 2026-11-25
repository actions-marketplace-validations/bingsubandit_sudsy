package a

func f(a bool) int {
	if a {
		return 1
	} else {
		g()
	}
	g()
	return 0
}

func h(a bool) int {
	return 1
	g()
 ^^^ unreachable-code
}

func k(a bool) int {
	if a {
		return 1
	} else {
		return 2
	}
	g()
 ^^^ unreachable-code
}

func m(a bool) int {
	if a {
		goto done
	}
	return 1
done:
	g()
	return 2
}

func p(ok bool) {
	for ok {
		g()
	}
	g()
}
