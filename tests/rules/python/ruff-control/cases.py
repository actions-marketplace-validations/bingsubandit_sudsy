try:
    pass
except Exception:
^ try-except-pass
#      ^^^^^^^^^ blind-except
    pass

if a:
^ collapsible-if
#  ^ undefined-variable
    if b:
#      ^ undefined-variable
        pass

try:
    pass
except Exception:
^ try-except-pass
#      ^^^^^^^^^ blind-except
    pass

import pdb
^^^^^^^^^^ leftover-debugger

def f(x):
    if x:
#   ^ needless-bool
        return True
    return False

if a:
#  ^ undefined-variable
    x = 1
elif b:
^ if-with-same-arms
#    ^ undefined-variable
    x = 1

f = open("a")
^ redeclared-variable
#   ^^^^^^^^^ open-file-without-with

with a:
^ nested-with
#    ^ undefined-variable
    with b:
#        ^ undefined-variable
        pass

raise Exception("x")
^^^^^^^^^^^^^^^^^^^^ raise-vanilla-class

if not isinstance(x, int):
^ unreachable-code
    raise ValueError("x")
#   ^^^^^^^^^^^^^^^^^^^^^ type-check-without-type-error

def f(x):
#   ^ redeclared-variable
    if x:
        return None
#       ^^^^^^^^^^^ explicit-return-none
    print(x)

x = 1
x
^ useless-expression

try:
    pass
except ValueError as e:
    raise e
#   ^^^^^^^ verbose-raise

def f():
#   ^ redeclared-variable
    raise
#   ^^^^^ bare-raise-outside-handler

exec("x = 1")
^^^^^^^^^^^^^ exec-call

hasattr(x, "__call__")
^^^^^^^^^^^^^^^^^^^^^^ hasattr-call
