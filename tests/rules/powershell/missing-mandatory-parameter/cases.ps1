Write-Verbose -Verbose "text"
^^^^^^^^^^^^^ missing-mandatory-parameter
Write-Verbose "text"
Write-Verbose -Message "text" -Verbose
Write-Verbose -ErrorAction Stop "text"
Write-Verbose -Verbose:$true "text"
^^^^^^^^^^^^^ missing-mandatory-parameter
Write-Verbose
^^^^^^^^^^^^^ missing-mandatory-parameter
Write-Verbose @args
"text" | Write-Verbose
Join-Path -Path a
Join-Path -Resolve a
^^^^^^^^^ missing-mandatory-parameter
Join-Path a -Resolve
