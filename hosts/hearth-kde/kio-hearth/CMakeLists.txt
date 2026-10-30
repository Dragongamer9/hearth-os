# kio-hearth: Hearth Storage as a KIO worker (hearth:/), so every KDE app's file dialogs and Hearth Files (Dolphin)
# open files on a paired Hearth Storage server. It doesn't speak Hearth Link's network protocol itself: it asks the
# running Hearth Link service (hosts/hearth-link, its user socket) exactly what Hearth Files' helper asks
# (storage.status, storage.ls, storage.stream). See docs/KDE-APPS.md.
# Built in CI only (.github/scripts/hearth-kde-build.sh), inside each base image so it links that image's KF6.
cmake_minimum_required(VERSION 3.20)
project(kio-hearth LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(QT_MAJOR_VERSION 6)
set(CMAKE_AUTOMOC ON)
# The worker is a loadable module (KDECMakeSettings would set this; without it KF6 makes plugins static).
set(BUILD_SHARED_LIBS ON)

find_package(ECM 6.0 REQUIRED NO_MODULE)
set(CMAKE_MODULE_PATH ${ECM_MODULE_PATH})
include(KDEInstallDirs)

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Network)
find_package(KF6 6.0 REQUIRED COMPONENTS KIO CoreAddons)

# Everything that doesn't need KIO (addresses, Hearth Link's socket, the local HTTP stream, turning answers into
# entries): a small library the tests use directly.
add_library(hearthcore STATIC
    src/hearthurl.cpp
    src/hearthlink.cpp
    src/httpget.cpp
    src/items.cpp)
target_include_directories(hearthcore PUBLIC ${CMAKE_CURRENT_SOURCE_DIR}/src)
target_link_libraries(hearthcore PUBLIC Qt6::Core Qt6::Network)
set_target_properties(hearthcore PROPERTIES POSITION_INDEPENDENT_CODE ON)

# The worker. KIO finds it by its file name (hearth.so) in kf6/kio.
kcoreaddons_add_plugin(kio_hearth SOURCES src/kio_hearth.cpp INSTALL_NAMESPACE "kf6/kio")
set_target_properties(kio_hearth PROPERTIES
    OUTPUT_NAME "hearth"
    LIBRARY_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/plugins/kf6/kio")
target_link_libraries(kio_hearth PRIVATE hearthcore KF6::KIOCore)

option(BUILD_TESTING "Build the tests" ON)
if(BUILD_TESTING)
    enable_testing()
    add_subdirectory(tests)
endif()
