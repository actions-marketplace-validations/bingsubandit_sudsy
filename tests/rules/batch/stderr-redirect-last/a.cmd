dir 2>&1 > out.txt
^^^ stderr-redirect-last
dir > out.txt 2>&1
echo 1>&2 oops
^^^^ stderr-redirect-last
(echo a) 1>&2
exit /b 1
