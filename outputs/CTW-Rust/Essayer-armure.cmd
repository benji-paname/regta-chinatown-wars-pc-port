@echo off
cd /d "%~dp0"
start "" "%~dp0CTW-Rust.exe" --slice --armour 127 --info-bands off --pos -1000 -1603 --camera psp %*
