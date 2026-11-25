package a

func f(ok bool, ch chan int, g func() bool) {
	if ok {
//     ^ no-empty-block
	}
	if ok {
//     ^ no-empty-block
		// nothing
	} else {
//      ^ no-empty-block
	}
	for ok {
//      ^ no-empty-block
	}
	for i := 0; i < 3; i++ {
//                      ^ no-empty-block
	}
	for i := range ch {
//                 ^ no-empty-block
	}
	for g() {
	}
	for range ch {
	}
	{
	}
	go func() {}()
	select {
	}
}

func empty() {}

func forever() {
	for {
//   ^ no-empty-block
	}
}
