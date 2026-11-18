def a(x):
    return 1
    b()
#   ^^^ unreachable-code
    c()

def d(x):
    if x:
        return 1
    elif x > 1:
        raise ValueError
    else:
        return 2
    e()
#   ^^^ unreachable-code

def f(x):
    while True:
        if x:
            break
    g()
    while True:
        with x:
            break
    g()
    while True:
        pass
    h()
#   ^^^ unreachable-code

def i(x):
    try:
        return 1
    except ValueError:
        pass
#       ^^^^ unreachable-code
    j()
    for y in x:
        continue
        k()
#       ^^^ unreachable-code

def m(x):
    if x:
        return 1
    elif x > 1:
        pass
    else:
        return 2
    n()
