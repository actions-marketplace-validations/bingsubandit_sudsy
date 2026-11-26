if a == a: pass
#  ^ undefined-variable
#  ^^^^^^ no-self-compare
#       ^ undefined-variable
if 1 == 2: pass
#  ^^^^^^ no-constant-comparison
if True: pass
#  ^^^^ no-constant-condition
while 1: pass
if (b := f()): pass
^^^^^^^^^^^^^^^^^^^ unreachable-code
#        ^ undefined-variable
d = {"k": 1, "k": 2, **d, **d}
#            ^^^ no-duplicate-keys
#                      ^ undefined-variable
#                           ^ undefined-variable
match a:
#     ^ undefined-variable
    case 1: pass
    case 1: pass
#   ^^^^^^^^^^^^ no-duplicate-case
try:
    int()
except Exception:
    pass  # ignored on purpose
try:
    int()
except Exception:
    # ignored on purpose
    pass
try:
    int()
except Exception:
^ no-empty-catch
    pass
