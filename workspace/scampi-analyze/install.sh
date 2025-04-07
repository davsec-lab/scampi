#!/bin/sh

cargo install --path scampi-binary --force
cargo install --path scampi-driver --force

rm -rf output
mkdir output

rm -rf .log
mkdir .log