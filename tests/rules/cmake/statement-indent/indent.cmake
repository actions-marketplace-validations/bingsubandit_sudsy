if(A)
    set(x 1)
#   ^^^ statement-indent
  # fine: the depth of the body
else()
  foreach(i a b)
    message(STATUS
      ${i}
    )
  endforeach()
endif()
