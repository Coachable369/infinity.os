cmake_minimum_required(VERSION 3.20)
project(InfinityLinkerProbe C)
add_executable(probe probe.c)
target_link_options(probe PRIVATE -nostdlib -static -Wl,-e,probe_entry)
