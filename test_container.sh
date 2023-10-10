#!/bin/bash

docker run --name duckrg -d -p "80:12321" duckrg duckrg --db /data/tpch.db
sleep 3
curl -sX POST -d @select.json -H "Content-Type: application/json" localhost/tpch | json_pp
docker stop duckrg && docker rm -f duckrg

docker run --rm -v $(pwd)/gh_youplot.sh:/tmp/gh_youplot.sh duckrg /tmp/gh_youplot.sh
