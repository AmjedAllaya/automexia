$ErrorActionPreference = 'Stop'
# Legacy Windows PowerShell code pages cannot encode this Unicode fixture.
# Set UTF-8 only in the isolated child; do not change the contributor's shell.
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
# Emit fixed public data without reading profiles, history or machine state.
[Console]::Out.WriteLine('NATIVE-SOURCE ' + ('retained text  ' * 8) + 'end')
[Console]::Out.WriteLine('NATIVE-SECOND ' + [char]0x754c + 'e' + [char]0x301 + ' end')
[Console]::Out.WriteLine('NATIVE_SELECTION_DONE')
