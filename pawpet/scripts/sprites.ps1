$projectRoot = "$PSScriptRoot\.."

$aseprite = "C:\Program Files\Aseprite\Aseprite.exe"

$spritePath = "$projectRoot\..\sprites\aseprite"
$destPath = "$projectRoot\..\sprites\png"
    
$slices = @(
)
    
$images = @(
    , ("battery3", "battery_16x8")
    , ("icons8", "icons_8x8")
    , ("sleeptest1", "sleeptest_64x64")
    , ("egg1", "egg_64x64")
    , ("slime1", "slime_32x32")
    , ("creature1", "creature_32x32")
    , ("bg1", "bg1_64x64")
    , ("window", "window")
)
    
$animations = @(
    , ("pet1-sit", "pet_sit_64x64")
    , ("egg-wobble1", "egg_wobble_32x32")
    , ("pet1-idle", "pet1_idle_32x32")
)

# export slices of part file
foreach ($file in $slices) {
    Write-Host $("slicing: {0} -> {1}" -f $file[0], $file[1])
    $in = "$($spritePath)\$($file[0]).aseprite"
    $out = "$($destPath)\$($file[1])_`{slice`}.png"

    $asepriteArgs = @("-b", "$in", "--save-as", $out)
    Start-Process $aseprite -ArgumentList $asepriteArgs -NoNewWindow -Wait
}

# export image data only
foreach ($file in $images) {
    Write-Host $("image: {0} -> {1}" -f $file[0], $file[1])
    $in = "$($spritePath)\$($file[0]).aseprite"
    $out = "$($destPath)\$($file[1]).png" 
    $asepriteArgs = @("-b", "$in", "--save-as", $out)
    Start-Process $aseprite -ArgumentList $asepriteArgs -NoNewWindow -Wait
}

$var = {}
# export animation data and png of other files
foreach ($file in $animations) {
    Write-Host $("animation: {0} -> {1}" -f $file[0], $file[1])
    $in = "$($spritePath)\$($file[0]).aseprite"
    $out = "$($destPath)\$($file[1]).png"
    # --format json-array --data "$($destPath)\$($file[1]).json"
    $asepriteArgs = @("-b", "$in", "--list-tags --sheet", $out)
    Start-Process $aseprite -ArgumentList $asepriteArgs -NoNewWindow -Wait 
}

# invoke png -> .paw file conversion
# TODO new rust export program for converting files from png to internal format
Start-Process python -ArgumentList ("$projectRoot/png2c/png2c.py $projectRoot/../sprites/png $projectRoot/../sprites/") -NoNewWindow -Wait
