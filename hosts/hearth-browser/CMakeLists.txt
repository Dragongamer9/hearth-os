# Hearth Browser on Chromium (CEF). Built by .github/workflows/os.yml: it downloads CEF's "minimal" Linux
# distribution, puts this folder inside it as hearth/, adds add_subdirectory(hearth) to its CMakeLists.txt and
# builds. Modeled on CEF's cefsimple sample (tests/cefsimple/CMakeLists.txt).
set(CEF_TARGET "hearth-browser")

set(HEARTH_SRCS
  src/main.cc
  src/browser.cc
  src/store.cc
  src/browser.h
  src/store.h
  )

# Logical target for the CEF library and the wrapper (from the CEF distribution's CMake helpers).
ADD_LOGICAL_TARGET("libcef_lib" "${CEF_LIB_DEBUG}" "${CEF_LIB_RELEASE}")
SET_CEF_TARGET_OUT_DIR()

add_executable(${CEF_TARGET} ${HEARTH_SRCS})
SET_EXECUTABLE_TARGET_PROPERTIES(${CEF_TARGET})
if(NOT HEARTH_VERSION)
  set(HEARTH_VERSION "0.1.0")
endif()
target_compile_definitions(${CEF_TARGET} PRIVATE HEARTH_VERSION="${HEARTH_VERSION}")
add_dependencies(${CEF_TARGET} libcef_dll_wrapper)
target_link_libraries(${CEF_TARGET} libcef_lib libcef_dll_wrapper ${CEF_STANDARD_LIBS})

# Next to the program: CEF's binaries and resources, and Hearth's pages.
COPY_FILES("${CEF_TARGET}" "${CEF_BINARY_FILES}" "${CEF_BINARY_DIR}" "${CEF_TARGET_OUT_DIR}")
COPY_FILES("${CEF_TARGET}" "${CEF_RESOURCE_FILES}" "${CEF_RESOURCE_DIR}" "${CEF_TARGET_OUT_DIR}")
