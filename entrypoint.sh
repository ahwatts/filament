#!/bin/bash

# filament can't (yet) do name lookups.
ext_ip=$(getent ahosts $HOSTNAME | awk '{ print $1; }' | head -n 1)

# The links args need to be like --link tracker_container_name:tracker_n
tracker_ips=()
for v in "${!TRACKER_@}"; do
    if [ -z "${v#*PORT_7001_TCP}" ]; then
        ip_url=${!v}
        tracker_ips+=("${ip_url#tcp://}")
    fi
done

# Collect those tracker IPs into a comma-separated list.
tracker_ips=$(IFS=","; echo "${tracker_ips[*]}")

set -x

exec /filament \
     --tracker-ip 0.0.0.0:7001 \
     --storage-ip 0.0.0.0:7500 \
     --base-url http://$ext_ip:7500 \
     --real-trackers $tracker_ips \
     ${@}
