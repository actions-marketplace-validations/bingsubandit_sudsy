return 1
^^^^^^ return-outside-function


def f():
    return 1


class A:
    return 2
#   ^^^^^^ return-outside-function

