echo !x!
::   ^^^ feature-not-enabled
setlocal EnableDelayedExpansion
echo !x!
setlocal DisableDelayedExpansion
set y=!z!
::    ^^^ feature-not-enabled
