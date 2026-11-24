function F {
    param([Parameter(Mandatory = $true)]$Unused, $Used)
#                                       ^^^^^^^ unused-parameter
    $Used
}
function G {
    param($a, $b)
    $a | ForEach-Object { $b }
    { $a }
}
