function F {
    $unused = 1
#   ^^^^^^^ unused-variable
    $read = 2
    "$read"
    $member = @{}
    $member.x = 1
    $list = @(1)
    $list[0] = 2
    $a, $b = 1, 2
    $script:s = 1
    $ErrorActionPreference = 'Stop'
    $count = 1
    $count++
    $block = 1
    $f = { $block }
    & $f
    $Mixed = 1
    "$mixed"
    ${with space} = 1
    ${with space}
    $q = 1
    $script:t = $q
    $splat = @{}
    Get-Item @splat
    $u = 1
#   ^^ unused-variable
}
