set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[private]
default:
    @just --list --unsorted

[windows]
package:
    cargo build --package hades2-savefile-editor --release
    wix build ./crates/hades2-savefile-editor/packaging/msi/hades2-savefile-editor.wxs

[windows]
install: package
    ./crates/hades2-savefile-editor/packaging/msi/hades2-savefile-editor.msi
