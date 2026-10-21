# dprint-plugin-shebang

[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/scop/dprint-plugin-shebang/badge)](https://scorecard.dev/viewer/?uri=github.com%2Fscop%2Fdprint-plugin-shebang)

Script [shebang](https://en.wikipedia.org/wiki/Shebang_%28Unix%29)
line formatter plugin for [dprint](https://dprint.dev).

## Usage

```shell
dprint config add scop/shebang
```

This plugin provides no configuration options.

Many common script filename extensions and some whole filenames are
associated with this plugin out of the box. However, scripts without
filename extensions typically need to be associated manually using
[dprint's associations config](https://dprint.dev/config/#associations).
