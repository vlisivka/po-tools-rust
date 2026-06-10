#!/bin/bash
set -ueE -o pipefail

cargo llvm-cov --lcov --output-path lcov.info
cargo crap --lcov lcov.info
