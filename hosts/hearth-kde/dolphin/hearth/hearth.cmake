# Hearth Files' own sources, added to Dolphin's build without editing its CMakeLists: passed to Dolphin's cmake as
# -DCMAKE_PROJECT_INCLUDE=<this file> (.github/scripts/hearth-kde-build.sh). CMake reads it right after Dolphin's
# project() call; the call below waits until all of Dolphin's CMakeLists has been read (so its targets exist), then
# puts these files into the target that builds main.cpp (patches/0001 calls hearthFilesSetup() from main.cpp).
set(HEARTH_FILES_SRC "${CMAKE_CURRENT_LIST_DIR}")

function(hearth_files_add_sources)
    set(_target "")
    foreach(_t dolphin dolphinstatic)
        if(TARGET ${_t})
            get_target_property(_srcs ${_t} SOURCES)
            foreach(_s IN LISTS _srcs)
                if(_s MATCHES "(^|/)main\\.cpp$")
                    set(_target ${_t})
                endif()
            endforeach()
        endif()
    endforeach()
    if(NOT _target)
        set(_target dolphin)
    endif()
    message(STATUS "Hearth Files: adding Hearth's sources to ${_target}")
    target_sources(${_target} PRIVATE
        "${HEARTH_FILES_SRC}/hearthfiles.cpp"
        "${HEARTH_FILES_SRC}/hearthgamepad.cpp")
    foreach(_t dolphin dolphinstatic)
        if(TARGET ${_t})
            target_include_directories(${_t} PRIVATE "${HEARTH_FILES_SRC}")
        endif()
    endforeach()
endfunction()

cmake_language(DEFER CALL hearth_files_add_sources)
