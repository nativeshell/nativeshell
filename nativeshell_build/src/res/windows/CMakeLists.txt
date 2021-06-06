cmake_minimum_required(VERSION 3.10)

project(NativeShellFlutterPlugins)

# Compilation settings that should be applied to most targets.
function(APPLY_STANDARD_SETTINGS TARGET)
  target_compile_features(${TARGET} PUBLIC cxx_std_17)
  target_compile_options(${TARGET} PRIVATE /W4 /WX /wd"4100")
  target_compile_options(${TARGET} PRIVATE /EHsc)
  target_compile_definitions(${TARGET} PRIVATE "_HAS_EXCEPTIONS=0")
  target_compile_definitions(${TARGET} PRIVATE "$<$<CONFIG:Debug>:_DEBUG>")

  # Additional settings
  target_compile_options(${TARGET} PRIVATE "$<$<CONFIG:Debug>:/MDd>") # needs to override cargo-rs /MD
  target_compile_definitions(${TARGET} PRIVATE "-D_UNICODE")
endfunction()

add_library(flutter_wrapper_plugin STATIC
    "cpp_client_wrapper/core_implementations.cc"
    "cpp_client_wrapper/standard_codec.cc"
    "cpp_client_wrapper/plugin_registrar.cc"
)
apply_standard_settings(flutter_wrapper_plugin)

target_include_directories(flutter_wrapper_plugin PUBLIC ${CMAKE_CURRENT_SOURCE_DIR})

add_library(flutter INTERFACE)
target_include_directories(flutter INTERFACE
        ${CMAKE_CURRENT_SOURCE_DIR}
        "${CMAKE_CURRENT_SOURCE_DIR}/cpp_client_wrapper/include")
target_link_libraries(flutter INTERFACE "${CMAKE_CURRENT_SOURCE_DIR}/flutter_windows.dll.lib")

# Subdirectories
