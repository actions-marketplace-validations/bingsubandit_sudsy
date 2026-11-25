# Does a thing.
function(a)
endfunction()

# Detached.

function(b)
^^^^^^^^ documented-command
endfunction()
#
macro(c)
^^^^^ documented-command
endmacro()
set(d 1)
#[[ block ]]
function(e)
^^^^^^^^ documented-command
endfunction()
# Text.
#
function(f)
endfunction()
