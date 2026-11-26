for %%i in (a b) do (
  for %%j in (c d) do echo %%i%%j
::^^^ nested-for
)
for %%i in (a) do echo %%i
