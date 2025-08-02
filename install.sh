#!/bin/sh

cargo install --path scampi-binary --force
cargo install --path scampi-driver --force

rm -rf .log
mkdir .log
