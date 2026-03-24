#!/bin/sh

cargo build
doas mv target/debug/ncc /bin/ncc
doas chmod +x /bin/ncc