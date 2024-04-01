root="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd)"

#--target-dir ./target/windows
if [ "_$OSTYPE" == "_msys" ] ; then
EXE_SUFFIX=".exe"
temp_path=$(cygpath -u "$LOCALAPPDATA")/Temp
trunk_root=$(cygpath -w $root)
# | sed 's|/|\\\\|g'
trunk_root="${trunk_root//\\//}"
export TEMP=$temp_path
LIB_PREFIX=
LIB_SUFFIX=".dll"
else
trunk_root=$root
EXE_SUFFIX=""
LIB_PREFIX="lib"
LIB_SUFFIX=".so"
temp_path=$TMP
if [ "_$temp_path" == "_" ] ; then
temp_path=$TEMP
fi
if [ "_$temp_path" == "_" ] ; then
temp_path=$TMPDIR
fi
if [ "_$temp_path" == "_" ] ; then
temp_path="/tmp"
fi
fi

function build_example {
  examples=$1
  cargo run --release --example $examples
  r=$?
  if [ ! "$r" == "0" ] ; then 
    exit -1;
  fi
}

build_example simple
