#!/bin/bash

curl -sL "https://api.github.com/users/proofrock/events?per_page=100" \
    | duckdb -s "COPY (SELECT type, count(*) AS event_count FROM read_json_auto('/dev/stdin') GROUP BY 1 ORDER BY 2 DESC LIMIT 10) TO '/dev/stdout' WITH (FORMAT 'csv', HEADER)" \
    | uplot bar -d, -H -t "GitHub Events for @dproofrock"
