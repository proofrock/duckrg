[TODO]

- `make update`
- `make clean`
- `make test` [~5min]
- Git: commit ("Pre-release commit") + push
- Git(flow): Release start + push the branch
- Update the version number
- Update the changelog
- Git: commit ("Version labeling & changelog") + push
- `make docker` [33min]
- `make docker-zbuild-linux` [1h]
- Compile on macos
  - checkout the release branch
  - `make build-macos` [17min]
- Compile on windows
  - checkout the release branch
  - `cargo build --release` [7min]
  - Zip to a filename like `duckrg-v0.x.y-win-x86_64-bundled.zip`
- Git(flow): Release finish
- Assemble the release on Github
- `make clean` (again)