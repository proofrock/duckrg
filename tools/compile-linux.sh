#! /bin/bash
# copy to akiko and run
PATH=/home/mano/local/go/bin:/home/mano/.cargo/bin:/home/mano/bin:/home/mano/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:/usr/games:/usr/local/games
rustup up
telegram.sh "inizio compilazione $(uname -om)"
git clone https://github.com/proofrock/sqliterg build-sqliterg
cd build-sqliterg/
make build-static-nostatic
find bin/ -type f -exec telegram.sh -f {} \;
cd ..
rm -rf build-sqliterg