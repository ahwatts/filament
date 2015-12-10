#!/bin/bash

mkdir -p /var/mogdata/dev${DEVICE_ID:-1}
exec /usr/local/bin/cmogstored \
     --docroot=/var/mogdata \
     --httplisten=0.0.0.0:7500 \
     --mgmtlisten=0.0.0.0:7501 \
     ${@}
