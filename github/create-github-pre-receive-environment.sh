#!/bin/bash

set -e

SCRIPT_DIR=$(dirname "$0")

if ! command -v docker >/dev/null 2>&1; then
    echo "Docker does not appear to be installed."
    echo "Please install Docker and ensure that it is on the path."
    exit 1
fi

docker rm capn-pre-receive || true
docker build -f $SCRIPT_DIR/Dockerfile -t capn-pre-receive $SCRIPT_DIR
docker create --name capn-pre-receive capn-pre-receive /bin/true
docker export capn-pre-receive | gzip > capn-pre-receive.tar.gz

echo "Successfully created capn-pre-receive.tar.gz"
