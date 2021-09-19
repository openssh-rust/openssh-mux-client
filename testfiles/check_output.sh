#!/bin/bash

cd $(dirname $(realpath $0))

exec diff data -
