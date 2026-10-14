# v0.1.0-alpha.2
> [!WARNING]
> This version is not production-ready.
New:
- Support graceful shutdown
- Support system service component(only on Windows)
- Support forcible shutdown
- Add `ipc` crate, providing IPC ability
- Introduce `Pod` concept, a minimal operational unit
- One `Pod` can listen more than one address
- Implement a new processing model of `datagram`
- Add `list` subcommand which could print currently running pods
- Add `COMPILE_TIME` constant in `constants` crate
- Add support for `echo` protocol based on UDP transport
- Integrate `xtask` build tool into project

Fix:
- Change unreasonable method signature of `Protocol`
- Change `datagram` processing logic

# v0.1.0-alpha.1
The initial unstable version