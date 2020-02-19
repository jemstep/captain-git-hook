#!/bin/bash

set -e

docker rm capn-pre-receive || true
docker build -f Dockerfile -t capn-pre-receive .
docker create --name capn-pre-receive capn-pre-receive /bin/true
docker export capn-pre-receive | gzip > capn-pre-receive.tar.gz
