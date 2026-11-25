package a

func f(a, b bool) {
	for {
		if a {
			continue
		} else {
//       ^ superfluous-else
			g()
		}
		if a {
			break
		} else if b {
			g()
		}
		if a {
			g()
			goto end
		} else {
//       ^ superfluous-else
			g()
		}
		if a {
			return
		} else {
			g()
		}
	}
end:
}
