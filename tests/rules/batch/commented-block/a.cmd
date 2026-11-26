if a==b (
::      ^ commented-block
    echo 1
    echo 2
    echo 3
)
REM what
if a==b (
    echo 1
    echo 2
    echo 3
)
if a==b (
    echo 1
    REM inside
    echo 2
    echo 3
)
if a==b (
    echo 1
    echo 2
)
