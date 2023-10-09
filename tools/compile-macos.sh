#! /bin/bash
# copy to Moverspace and run
rustup up
cd /Users/mano/MoverSpace
./telegram.sh "inizio compilazione $(uname -om)"
git clone https://github.com/proofrock/sqliterg build-sqliterg
cd build-sqliterg/
make build-macos
find bin/ -type f -exec ../telegram.sh -f {} \;
cd ..
rm -rf build-sqliterg