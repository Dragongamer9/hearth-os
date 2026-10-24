# Hearth's Qt Quick renderer (Experimental): docs/RENDERERS.md. Built by CI inside each Hearth OS image's own base
# (.github/scripts/hearth-qml-build.sh), so it links that image's Qt.
cmake_minimum_required(VERSION 3.21)
project(hearth-qml LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
if(NOT CMAKE_BUILD_TYPE)
  set(CMAKE_BUILD_TYPE Release)
endif()

find_package(Qt6 6.5 REQUIRED COMPONENTS Core Gui Qml Quick)
qt_standard_project_setup(REQUIRES 6.5)

set(HEARTH_QML_VERSION "dev" CACHE STRING "The Hearth version this is part of")
set(HEARTH_FONT "" CACHE FILEPATH "Anybody, made static for headings (anybody-hearth.ttf); left out when empty")
option(HEARTH_WITH_SDL2 "Read controllers with SDL2" ON)

qt_add_executable(hearth-qml
  src/main.cpp
  src/host.cpp src/host.h
  src/corelink.cpp src/corelink.h
  src/pad.cpp src/pad.h
)
target_compile_definitions(hearth-qml PRIVATE HEARTH_QML_VERSION="${HEARTH_QML_VERSION}")
target_link_libraries(hearth-qml PRIVATE Qt6::Core Qt6::Gui Qt6::Qml Qt6::Quick)

if(HEARTH_WITH_SDL2)
  find_package(PkgConfig)
  if(PkgConfig_FOUND)
    pkg_check_modules(SDL2 IMPORTED_TARGET sdl2)
  endif()
  if(SDL2_FOUND)
    target_link_libraries(hearth-qml PRIVATE PkgConfig::SDL2)
    target_compile_definitions(hearth-qml PRIVATE HAVE_SDL2=1)
    message(STATUS "hearth-qml: controllers through SDL2 ${SDL2_VERSION}")
  else()
    message(WARNING "hearth-qml: SDL2 not found; controllers only through Hearth's own pad events")
  endif()
endif()

set(SINGLETONS qml/Theme.qml qml/App.qml qml/Nav.qml)
set_source_files_properties(${SINGLETONS} PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_qml_module(hearth-qml
  URI HearthQml
  VERSION 1.0
  QML_FILES
    ${SINGLETONS}
    qml/Main.qml
    qml/Icons.js
    qml/Icon.qml
    qml/Focusable.qml
    qml/Btn.qml
    qml/Chip.qml
    qml/SetRow.qml
    qml/Bar.qml
    qml/Heading.qml
    qml/ArtImage.qml
    qml/GameTile.qml
    qml/Scroller.qml
    qml/TopBar.qml
    qml/Footer.qml
    qml/Toast.qml
    qml/HomeView.qml
    qml/LibraryView.qml
    qml/GameView.qml
    qml/DownloadsView.qml
    qml/SettingsView.qml
    qml/MissingView.qml
    qml/MainMenu.qml
    qml/QuickAccess.qml
    qml/PowerMenu.qml
    qml/Sheet.qml
    qml/SearchLayer.qml
    qml/LaunchScreen.qml
    qml/OverlayWindow.qml
    qml/SelfTest.qml
    qml/Input.qml
    qml/OskBoard.qml
    qml/OskKey.qml
  RESOURCES
    qml/hearth-osk.js
)

if(HEARTH_FONT AND EXISTS "${HEARTH_FONT}")
  set_source_files_properties("${HEARTH_FONT}" PROPERTIES QT_RESOURCE_ALIAS "anybody-hearth.ttf")
  qt_add_resources(hearth-qml "hearth_fonts" PREFIX "/fonts" FILES "${HEARTH_FONT}")
  message(STATUS "hearth-qml: Anybody built in for headings")
endif()

install(TARGETS hearth-qml RUNTIME DESTINATION .)
