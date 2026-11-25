A = B = 1
try:
    pass
except:
^ default-except-not-last
^ no-broad-catch
    pass
except A:
    pass
try:
    pass
except (A, B):
    pass
try:
    pass
except exit.Exception:
    B = 2
except (A, Exception):
^ no-broad-catch
    B = 2
