#!/bin/bash
write_configuration() {
  /usr/bin/plutil -create xml1 "$1"
  /usr/bin/plutil -insert enabled -bool YES "$1"
  /usr/bin/plutil -insert notifier_path -string "$2" "$1"
  /usr/bin/plutil -convert json "$1"
}
