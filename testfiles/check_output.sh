#!/bin/bash

set -euxo pipefail

cd "$(dirname "$(realpath "$0")")"

exec diff data -
