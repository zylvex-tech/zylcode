
$files = Get-ChildItem -Path "C:\Projects\zylcode\crates\zylcode-mcp\src" -Filter "*.rs" -Recurse

foreach ($file in $files) {
    $content = Get-Content $file.FullName -Raw
    $lines = Get-Content $file.FullName
    
    # Find all struct names that have impl blocks with pub fn new() -> Self
    $matches = [regex]::Matches($content, "impl\s+(\w+)\s*\{[^}]*pub fn new\(\) -> Self")
    
    foreach ($match in $matches) {
        $structName = $match.Groups[1].Value
        
        # Check if Default is already implemented
        if ($content -notmatch "impl Default for $structName") {
            Write-Host "Adding Default for $structName in $($file.Name)"
            
            # Find the line with "impl $structName {"
            $implPattern = "impl $structName \{"
            $lineIndex = -1
            for ($i = 0; $i -lt $lines.Count; $i++) {
                if ($lines[$i] -match $implPattern) {
                    $lineIndex = $i
                    break
                }
            }
            
            if ($lineIndex -ge 0) {
                # Insert Default implementation before the impl block
                $defaultImpl = @"

impl Default for $structName {
    fn default() -> Self {
        Self::new()
    }
}

"@
                $lines = $lines[0..($lineIndex-1)] + $defaultImpl.Split("`n") + $lines[$lineIndex..($lines.Count-1)]
                $lines | Set-Content $file.FullName
            }
        }
    }
}
