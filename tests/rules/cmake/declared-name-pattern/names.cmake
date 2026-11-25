set(lower 1)
#   ^^^^^ declared-name-pattern
set(UPPER 1)
set(lc "" CACHE STRING "x")
#   ^^ declared-name-pattern
set(_OK "" CACHE INTERNAL "x")
set(lc2 "" CACHE STRING INTERNAL)
#   ^^^ declared-name-pattern
set(ENV{any} 1)
set(${x} 1)
set(${x}_a 1)
#   ^^^^^^ declared-name-pattern
list(LENGTH UPPER out)
#                 ^^^ declared-name-pattern
foreach(Item a b)
#       ^^^^ declared-name-pattern
endforeach()
function(F_n Arg)
#        ^^^ declared-name-pattern
#            ^^^ declared-name-pattern
  set(Local 1)
#     ^^^^^ declared-name-pattern
  set(local 1)
  set(xy 1 PARENT_SCOPE)
endfunction()
macro(m)
  set(xy 1)
  set(XY 1)
#     ^^ declared-name-pattern
endmacro()
