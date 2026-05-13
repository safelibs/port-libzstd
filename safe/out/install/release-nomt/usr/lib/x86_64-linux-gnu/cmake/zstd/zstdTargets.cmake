if(NOT TARGET zstd::libzstd_shared)
  include("${CMAKE_CURRENT_LIST_DIR}/zstdTargets-noconfig.cmake")
endif()
