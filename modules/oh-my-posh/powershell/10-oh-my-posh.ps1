$ohMyPoshConfig = Join-Path (Split-Path -Parent $PROFILE) 'oh-my-posh.config.json'
oh-my-posh init pwsh --config $ohMyPoshConfig | Invoke-Expression
