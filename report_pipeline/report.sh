#!/bin/sh

cargo run --release -- report election-metadata raw-data preprocessed reports "$@"

