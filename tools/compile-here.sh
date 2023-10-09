#! /bin/bash
PATH=/home/devel/local:/home/devel/local/go/bin:/home/devel/.cargo/bin:/home/devel/bin:/home/devel/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games
rustup up
telegram.sh "inizio compilazione $(uname -om)"
git clone https://github.com/proofrock/sqliterg build-sqliterg
cd build-sqliterg/
make build-static-nostatic
find bin/ -type f -exec telegram.sh -f {} \;
cd ..
rm -rf build-sqliterg