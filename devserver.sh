#!/bin/sh
systemfd --no-pid -s 127.0.0.1:3030 -- cargo watch -x run
