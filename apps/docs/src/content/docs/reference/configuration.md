---
title: Configuration
description: Environment variables used to configure yapzm.
---

YAPZM is configured through environment variables. There is no configuration file.

## Steam

### `YAPZM_STEAM_DIR`

Overrides the Steam install root, used in place of automatic discovery.

```sh
YAPZM_STEAM_DIR=/path/to/steam yapzm health
```

- When set to a non-empty value, YAPZM tries this path first.
- If the path is not a valid Steam install, YAPZM falls back to automatic discovery.
- An empty value is treated as unset.
- Under WSL, a Windows-style path such as `C:\Steam` is translated to the mounted Linux path, for example `/mnt/c/Steam`. The mount root honors `[automount] root` in `/etc/wsl.conf`.

This override affects everything that depends on Steam detection: libraries, game installs, and Workshop content.
