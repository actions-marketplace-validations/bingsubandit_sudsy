list = [1]
^^^^ redefines-builtin-id


def f(id, ok):
#     ^^ redefines-builtin-id
    return ok


print(list)
