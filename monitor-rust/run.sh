#!/bin/bash -e

<<COMMENT
curl -X GET -L http://localhost:8080

# output
COMMENT

cargo run
