rustup default  stable-x86_64-pc-windows-msvc
REM You need Visual Studio with C++ compiler and Windows SDK components
REM aws-lc-sys needs perl to compile openssl-sys as vendored
REM perl Strawberry will install nasm
call "%ProgramFiles%\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
REM here clang is installed from llvm.org
set PATH=%PATH%;%ProgramFiles%\LLVM\bin

cargo build --release --example simple

rustup default  stable-x86_64-pc-windows-gnu

