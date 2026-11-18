def f():
    a, b = g()
#      ^ unused-unpacked-variable
#          ^ undefined-variable
    return a

class A:
    x = []
#       ^^ mutable-class-default

import os as os
^^^^^^^^^^^^^^^ unsorted-imports
#      ^^^^^^^^ useless-import-alias

def f(a):
#   ^ redeclared-variable
    with g() as a:
#        ^ undefined-variable
#               ^ shadowed-variable
        pass

x = ["a" "b", "c"]
#    ^^^^^^^ implicit-concat-in-collection

__all__ = ["b", "a"]
#         ^^^^^^^^^^ unsorted-dunder-all

class A:
#     ^ redeclared-variable
    __slots__ = ("b", "a")
#               ^^^^^^^^^^ unsorted-dunder-slots

import sys
^ unsorted-imports
import os
#      ^^ redeclared-variable

import functools
class A:
#     ^ redeclared-variable
    @functools.cache
#   ^^^^^^^^^^^^^^^^ cached-method
    def f(self):
        pass

def f(x=list()):
#   ^ redeclared-variable
#       ^^^^^^ function-call-in-default
#       ^^^^^^ mutable-argument-default
    pass

import subprocess
^^^^^^^^^^^^^^^^^ unsorted-imports
subprocess.run(["ls"])
^^^^^^^^^^^^^^^^^^^^^^ subprocess-run-without-check

a = b = "x"
x = ", ".join([a, b])
#   ^^^^^^^^^^^^^^^^^ static-join

x = "%s" % y
#   ^^^^^^^^ printf-string-formatting
#          ^ undefined-variable

x = "{}".format(y)
#   ^^^^^^^^^^^^^^ f-string
#               ^ undefined-variable

import re
^^^^^^^^^ unsorted-imports
x = re.compile("a", re.I)
#                   ^^^^ regex-flag-alias
