# Installs usbip-win2 (BSD-2-Clause, signed by Microsoft) and registers the
# scheduled task that attaches Faderust's USB cable. Started elevated by
# Faderust; the arguments are the user to run the tasks as and the number of
# cables (one task and one bus id, 1-1, 1-2, …, each).
param([string]$User, [int]$Cables)
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$result = Join-Path $dir 'result.txt'
try {
    $usbip = Join-Path $env:ProgramFiles 'USBip\usbip.exe'
    if (-not (Test-Path $usbip)) {
        $url = 'https://github.com/vadimgrn/usbip-win2/releases/download/v.0.9.8.1/USBip-0.9.8.1-x64.exe'
        $sha = '38cad6d4432b52d5bb9409d9ad03b72fdffc4ada4cd3a48fbeca1a2752a8518a'
        $file = Join-Path $dir 'usbip-setup.exe'
        [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $file
        $got = (Get-FileHash -Algorithm SHA256 -Path $file).Hash.ToLower()
        if ($got -ne $sha) { throw "Checksum of the download does not match ($got)." }
        $p = Start-Process -FilePath $file -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-' -Wait -PassThru
        if ($p.ExitCode -ne 0) { throw "The usbip-win2 setup failed (exit code $($p.ExitCode))." }
        if (-not (Test-Path $usbip)) { throw 'usbip.exe not found after the setup.' }
    }
    $principal = New-ScheduledTaskPrincipal -UserId $User -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 5)
    for ($i = 1; $i -le $Cables; $i++) {
        $name = if ($i -eq 1) { 'Faderust Cable' } else { "Faderust Cable $i" }
        $action = New-ScheduledTaskAction -Execute $usbip -Argument "attach -r 127.0.0.1 -b 1-$i"
        Register-ScheduledTask -TaskName $name -Action $action -Principal $principal -Settings $settings -Force | Out-Null
    }
} catch {
    $_.Exception.Message | Set-Content -Path $result -Encoding UTF8
    exit 1
}
