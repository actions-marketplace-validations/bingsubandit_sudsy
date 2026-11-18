__all__ = ["present", "missing"]
#                     ^^^^^^^^^ undefined-export
present = 1
print(undefined, present, len)
#     ^^^^^^^^^ undefined-name


def reads_before_assignment():
    print(present)
#         ^^^^^^^ undefined-local
    present = 2
#   ^^^^^^^ unused-variable


def handled():
    try:
        name_error_is_handled
#       ^^^^^^^^^^^^^^^^^^^^^ useless-expression
    except NameError:
        pass
    del present
#       ^^^^^^^ undefined-name
    del not_bound
#       ^^^^^^^^^ undefined-name


def annotations(a: "Missing", b: "bad syntax here"):
#                   ^^^^^^^ undefined-name
#                                ^^^^^^^^^^^^^^^^^ forward-annotation-syntax-error
    x: int
    print(x)
#         ^ undefined-name


class K:
    attr = 1
    print(attr, __qualname__)

    def method(self):
        return attr
#              ^^^^ undefined-name
