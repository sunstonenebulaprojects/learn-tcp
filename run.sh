#!/bin/bash
sudo setcap cap_net_admin=eip target/release/cli
sudo tunctl -3
sudo ip addr add 192.168.0.1/24 dev tun0
sudo ip link set up dev tun0
sudo route add -host 192.168.0.2 dev tun0
