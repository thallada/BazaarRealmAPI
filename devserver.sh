#!/bin/sh
systemfd --no-pid -s 0.0.0.0:3030 -- cargo watch -x run
