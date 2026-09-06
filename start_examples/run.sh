#!/bin/bash

echo -n "appdir (see rwserver_config.toml):  "
grep "app" < rwserver_config.toml

rwlang-server --config ./rwserver_config.toml --check-config
echo
rwlang-server --config ./rwserver_config.toml
