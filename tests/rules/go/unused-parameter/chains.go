package p

func chain(h [][]int, i int, y []int, idx int, m map[string]map[string]int, name Q) {
	h[i][y[idx]] = 1
	m[name.Domain][name.Identifier] = 1
}

func qualified(net string) (net.Conn, error) {
//             ^^^ unused-parameter
	return nil, nil
}
