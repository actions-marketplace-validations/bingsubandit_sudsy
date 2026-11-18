import select
import socket

Base = M = f = 1


class A(object):
#       ^^^^^^ useless-object-inheritance
    pass


class B(Base, object, metaclass=M):
#             ^^^^^^ useless-object-inheritance
    pass


x = u"a" "b"
#   ^^^^ unicode-literal
y = f(u"a", "b")
#     ^^^^ unicode-literal

try:
    pass
except IOError:
#      ^^^^^^^ os-error-alias
    pass
except (EnvironmentError, ValueError, socket.error):
#       ^^^^^^^^^^^^^^^^ os-error-alias
#                                     ^^^^^^^^^^^^ os-error-alias
    pass
except select.error:
#      ^^^^^^^^^^^^ os-error-alias
    pass
raise IOError("x")
#     ^^^^^^^ os-error-alias
x = IOError
^^^^^^^^^^^ unreachable-code
