# Package receipt — extreme_reactors 0.2.4

Extreme Reactors (called *Big Reactors* before 1.9). The jar's mod id, and the namespace of every
block and item it registers, is still `bigreactors` — that is the mod's own choice for save
compatibility, not our naming. The registry id, the file name and the listing use the project's
current name.

| field | value |
|---|---|
| version | `0.2.4` |
| file | `extreme_reactors.hmod` |
| bytes | 81,742 |
| sha256 | `c0168cedbb2a222886ec053ba0a4c6817956bb13ba5439eda04e9056e771c65b` |
| source | this tag — the tree here is the source that produced the archive |
| game | 26.3 |
| horizon | `0b14f8acfd97fa2c750e42795249dcc981178ca4` |
| horizon-mixins | `e59da9094f0c2fdda6c627fe9eff22dec6a906f0` |
| content namespace | `bigreactors:` (the jar's mod id, unchanged) |

## Verification of these exact bytes

`cargo test` **56 passed / 0 failed** (the new assertion failed 54/1 before the fix) · `clippy --all-targets -- -D warnings` 0 warnings · `fmt --check` clean · two builds `cmp`-identical · `check` exit 0 · `fits --build 0b14f8a` `0b14f8a made for` exit 0 · `fits` `0a875da fits` exit 0 · `verify-sdk-package.py` 1/1 · launcher leg `PASS` + `EXIT:0`; control `EXIT:1`; `SELFTEST=1` `EXIT:1`; `NEGATIVE=1` `EXIT:1`; `PEAK_NEGATIVE=1` `EXIT:1`; un-budgeted counterfactual `EXIT:1` (`a tick with 5 invalidated machines ran 7 fresh scans, not the budgeted 4`); `check-e2e-logs --strict` 6/6.
