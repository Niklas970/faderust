# Installs usbip-win2 (BSD-2-Clause, signed by Microsoft) and registers the
# scheduled tasks that attach Faderust's USB cables, or removes both again.
# Started elevated by Faderust; the arguments are the user to run the tasks as,
# the number of cables (one task and one bus id, 1-1, 1-2, …, each) and the
# action, "install" or "remove".
param([string]$User, [int]$Cables, [string]$Action = 'install')
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$dir = Split-Path -Parent $MyInvocation.MyCommand.Path
$result = Join-Path $dir 'result.txt'

# usbip-win2's entry in Windows' app list. Faderust manages it, so it is
# marked as a system component: it stays out of the list, and Faderust offers
# to remove it.
function Hide-Entry {
    foreach ($root in 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall', 'HKLM:\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall') {
        foreach ($key in Get-ChildItem $root -ErrorAction SilentlyContinue) {
            try {
                $p = Get-ItemProperty $key.PSPath -ErrorAction Stop
                if ($p.DisplayName -like 'USBip*') {
                    New-ItemProperty -Path $key.PSPath -Name SystemComponent -Value 1 -PropertyType DWord -Force | Out-Null
                }
            } catch { }
        }
    }
}

try {
    $usbip = Join-Path $env:ProgramFiles 'USBip\usbip.exe'
    if ($Action -eq 'remove') {
        for ($i = 1; $i -le 4; $i++) {
            $name = if ($i -eq 1) { 'Faderust Cable' } else { "Faderust Cable $i" }
            Unregister-ScheduledTask -TaskName $name -Confirm:$false -ErrorAction SilentlyContinue
        }
        $uninstall = Join-Path $env:ProgramFiles 'USBip\unins000.exe'
        if (Test-Path $uninstall) {
            $p = Start-Process -FilePath $uninstall -ArgumentList '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART' -Wait -PassThru
            if ($p.ExitCode -ne 0) { throw "Removing usbip-win2 failed (exit code $($p.ExitCode))." }
        }
        exit 0
    }
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
    Hide-Entry
    $principal = New-ScheduledTaskPrincipal -UserId $User -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit (New-TimeSpan -Minutes 5)
    for ($i = 1; $i -le $Cables; $i++) {
        $name = if ($i -eq 1) { 'Faderust Cable' } else { "Faderust Cable $i" }
        $task = New-ScheduledTaskAction -Execute $usbip -Argument "attach -r 127.0.0.1 -b 1-$i"
        Register-ScheduledTask -TaskName $name -Action $task -Principal $principal -Settings $settings -Force | Out-Null
    }
} catch {
    $_.Exception.Message | Set-Content -Path $result -Encoding UTF8
    exit 1
}
