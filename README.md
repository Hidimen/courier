# Courier: An extensible, light-weight network server software.
This software aims to provide a transport and protocol-agnostic developing experience.
It can be used in various fields: web, gaming server and mailing.

## How to use
### From source
1. Make sure that [Rust](https://rust-lang.org) and relevant components are installed in your computer.
```bash
cargo -V # at least 1.98
rustc -V # at least 1.98
```
2. Download source code from Github.
```bash
git clone https://github.com/Hidimen/courier.git
cd courier
```
4. Build with all modules.
```bash
cargo xtask build --with-all-modules
```
5. Run an echo server.
On Linux:
```bash
./output/courier start --config-file ./examples/example.maq
```
On windows:
```powershell
./output/courier.exe start --config-file ./examples/example.maq
```
5. Check functionality
```bash
ncat 127.0.0.1 8080
```

### From binary
1. Download pre-compiled binary file.
2. Execute it in shell.

## How to contribute
All issues and pull-request are welcomed.
