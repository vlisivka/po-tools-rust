#!/bin/bash
set -ue

cargo test

cargo run -- \
  --cases 3 translate \
  --debug \
  --tm test-data/memory.po \
  --dictionary test-data/test_dict.tsv \
  --model ollama:VladimirGav/gemma4-26b-16GB-VRAM:latest \
  test-data/test.po
