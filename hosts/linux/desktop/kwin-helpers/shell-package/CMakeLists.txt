# Hearth's Plasma/Shell package type for KWin's screen locker on Debian (see hearth-shell-package.cpp).
cmake_minimum_required(VERSION 3.16)
project(hearth-shell-package CXX)
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_AUTOMOC ON)
find_package(Qt6 REQUIRED COMPONENTS Core)
find_package(KF6Package REQUIRED)
find_package(KF6CoreAddons REQUIRED)
# KPackage finds the type "Plasma/Shell" by the file name plasma_shell.so (kf6/packagestructure).
add_library(plasma_shell MODULE hearth-shell-package.cpp)
target_link_libraries(plasma_shell KF6::Package KF6::CoreAddons Qt6::Core)
set_target_properties(plasma_shell PROPERTIES PREFIX "")
