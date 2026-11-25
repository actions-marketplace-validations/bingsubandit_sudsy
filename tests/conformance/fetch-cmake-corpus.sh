#!/usr/bin/env bash
# Fetches the pinned CMake corpus into tests/conformance/corpus/cmake/<repo> (gitignored): the
# `CMakeLists.txt` and `*.cmake` files of LLVM, CMake, Qt base, VTK, OpenCV, extra-cmake-modules
# and protobuf.
source "$(dirname "$0")/tarball-corpus.sh"
fetch_tarballs cmake <<'EOT'
llvm/llvm-project 579438300438d6b04db820e418412ab34c48bab9 CMakeLists.txt *.cmake
Kitware/CMake 4207edd6abcaa33719037ec1095d7767f4c684b4 CMakeLists.txt *.cmake
qt/qtbase 30c02b23e5a4ff0fcfcd01da360a5268863b07ff CMakeLists.txt *.cmake
Kitware/VTK ce6eea283e837ea7107fab97cd2986e6c485ae90 CMakeLists.txt *.cmake
opencv/opencv d1abca9a89312703719b82d884b591c82620533e CMakeLists.txt *.cmake
KDE/extra-cmake-modules dc2bc9b098ded38d12a9b8e30df575de6e070095 CMakeLists.txt *.cmake
protocolbuffers/protobuf 6c108d471e24a796df8b21e691ff669ef9a52b65 CMakeLists.txt *.cmake
EOT
