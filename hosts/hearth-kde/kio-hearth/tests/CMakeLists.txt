find_package(Qt6 REQUIRED COMPONENTS Test)

# The pieces without KIO: addresses, Hearth Link's answers, the stream's head, entries.
add_executable(hearthcoretest hearthcoretest.cpp)
target_link_libraries(hearthcoretest hearthcore Qt6::Test)
add_test(NAME hearthcoretest COMMAND hearthcoretest)

# The worker itself through KIO (KIO starts it in its own process, as in any app), against a fake Hearth Link
# socket and a fake stream in the test.
add_executable(kiohearthtest kiohearthtest.cpp)
target_link_libraries(kiohearthtest Qt6::Test Qt6::Network KF6::KIOCore)
add_dependencies(kiohearthtest kio_hearth)
add_test(NAME kiohearthtest COMMAND kiohearthtest)
set_tests_properties(kiohearthtest PROPERTIES
    ENVIRONMENT "QT_PLUGIN_PATH=${CMAKE_BINARY_DIR}/plugins;QT_QPA_PLATFORM=offscreen;KDE_FORK_SLAVES=1"
    TIMEOUT 120)
