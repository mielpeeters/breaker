#!/bin/bash

# convert file names to remove spaces

target="hi hat"

for file in "$target"*
do
    name="${file%.*}"
    filenumber="${name#hi hat (}"
    filenumber="${filenumber%)}"
    echo "$name"
    echo "$filenumber"
    echo "New file: hihat${filenumber}.wav"
    mv "$file" "hihat$filenumber.wav"
done
