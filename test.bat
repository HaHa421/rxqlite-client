rustup default  stable-x86_64-pc-windows-msvc

REM We need either to copy rxqlited.exe to .\..\rxqlite-client\target\release or set RXQLITED_DIR en var to a dir containing rxqlited.exe
REM COPY .\..\rxqlite\target\release\rxqlited.exe .\..\rxqlite-client\target\release
SET RXQLITED_DIR=.\..\rxqlite\target\release
REM You need Visual Studio with C++ compiler and Windows SDK components
call "%ProgramFiles%\Microsoft Visual Studio\2022\Community\Common7\Tools\VsDevCmd.bat"
set PATH=%PATH%;C:\Program Files\LLVM\bin
set RUST_LOG=error
cargo test  --release 
REM pool_notifications4_insecure_ssl 
REM pool_notifications2_insecure_ssl
REM notifications2_insecure_ssl
REM -- --nocapture

rustup default  stable-x86_64-pc-windows-gnu

