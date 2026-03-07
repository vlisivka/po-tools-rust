#!/bin/bash
set -ue

cargo test

cargo run -- -c 3 translate --debug --tm test-data/memory.po -d test-data/test_dict.tsv test-data/test.po
