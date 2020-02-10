#!/bin/bash

# prefix a command with `monitor.sh`, but you'll need a tmpfs mounted
# to monitor the size used. Mount one with:
#
#   sudo mount -t tmpfs tmpfs /tmp/test -o rw,nosuid,noatime,nodev,size=8G,mode=1777

cmd=$*
tmpfs=/tmp/test

startts=$(date +'%s')
TMPDIR=$tmpfs $cmd &

cmdpid=$!

sleep 0.2
peakmem=0
peakdisk=0
while kill -0 $cmdpid >& /dev/null
do
  mem=$(cat "/proc/${cmdpid}/status" | grep VmRSS | awk '{print $2}')
  if (( $mem > $peakmem )); then
    peakmem=$mem
  fi
  disk=$(df $tmpfs --output=used | tail -1)
  if (( $disk > $peakdisk )); then
    peakdisk=$disk
  fi
  sleep 0.2
done

echo "Took ~$(expr $(date +'%s') - $startts) seconds"
echo "Peak disk $(expr $peakdisk / 1024) Mb"
echo "Peak mem $(expr $peakmem / 1024) Mb"
