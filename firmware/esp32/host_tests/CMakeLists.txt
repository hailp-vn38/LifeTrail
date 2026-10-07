cmake_minimum_required(VERSION 3.16)
project(lifetrail_firmware_host_tests C)

enable_testing()

add_library(gps_policy STATIC
    ../components/lifetrail_gps_policy/lifetrail_gps_policy.c
    ../components/lifetrail_gps_policy/lifetrail_gps_history.c
    ../components/lifetrail_gps_policy/lifetrail_gps_selection.c)
target_include_directories(gps_policy PUBLIC
    ../components/lifetrail_gps/include ../components/lifetrail_gps_policy/include)
target_compile_features(gps_policy PRIVATE c_std_11)
target_compile_options(gps_policy PRIVATE -Wall -Wextra -Werror -pedantic)
target_link_libraries(gps_policy PUBLIC m)
add_executable(gps_policy_filter gps_policy_filter.c)
target_link_libraries(gps_policy_filter PRIVATE gps_policy)
target_compile_options(gps_policy_filter PRIVATE -Wall -Wextra -Werror -pedantic)

add_executable(gps_epoch_batch_tests
    ../components/third_party/minmea/minmea.c
    ../components/lifetrail_gps/lifetrail_gps.c
    ../components/lifetrail_gps/lifetrail_nmea.c
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    tests/test_gps_epoch_batch.c)

target_include_directories(gps_epoch_batch_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_gps
    ../components/lifetrail_gps_policy/include
    ../components/third_party/minmea
    ../components/lifetrail_storage/include)

target_compile_features(gps_epoch_batch_tests PRIVATE c_std_11)
target_compile_options(gps_epoch_batch_tests PRIVATE -Wall -Wextra -Werror -pedantic)
target_compile_definitions(gps_epoch_batch_tests PRIVATE LT_GPS_TEST_FIXTURES_DIR="${CMAKE_CURRENT_SOURCE_DIR}/fixtures")

add_test(NAME gps_epoch_batch_tests COMMAND gps_epoch_batch_tests)

add_executable(gps_policy_tests
    tests/test_gps_policy.c)
target_link_libraries(gps_policy_tests PRIVATE gps_policy)
target_compile_options(gps_policy_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME gps_policy_tests COMMAND gps_policy_tests)

add_executable(batch_store_tests
    ../components/lifetrail_storage/lifetrail_buffered_file.c
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    ../components/lifetrail_storage/lifetrail_batch_store.c
    ../components/lifetrail_storage/lifetrail_sha256.c
    tests/test_batch_store.c)

target_include_directories(batch_store_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_gps_policy/include
    ../components/lifetrail_storage/include)

target_compile_features(batch_store_tests PRIVATE c_std_11)
target_compile_options(batch_store_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME batch_store_tests COMMAND batch_store_tests)

add_executable(provisioning_sync_tests
    ../components/lifetrail_storage/lifetrail_buffered_file.c
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    ../components/lifetrail_storage/lifetrail_batch_store.c
    ../components/lifetrail_storage/lifetrail_sha256.c
    ../components/lifetrail_provisioning/lifetrail_provisioning.c
    ../components/lifetrail_sync/lifetrail_sync.c
    ../components/lifetrail_sync/lifetrail_sync_ack.c
    tests/test_provisioning_sync.c)

target_include_directories(provisioning_sync_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_gps_policy/include
    ../components/lifetrail_storage/include
    ../components/lifetrail_provisioning/include
    ../components/lifetrail_sync/include)

target_compile_features(provisioning_sync_tests PRIVATE c_std_11)
target_compile_options(provisioning_sync_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME provisioning_sync_tests COMMAND provisioning_sync_tests)

add_executable(recording_pipeline_tests
    tests/test_recording_pipeline.c
    ../components/lifetrail_storage/lifetrail_storage_queue.c
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    ../components/lifetrail_storage/lifetrail_batch_store.c
    ../components/lifetrail_storage/lifetrail_buffered_file.c
    ../components/lifetrail_storage/lifetrail_sha256.c)
target_include_directories(recording_pipeline_tests PRIVATE ../components/lifetrail_storage/include)
target_link_libraries(recording_pipeline_tests PRIVATE gps_policy)
target_compile_options(recording_pipeline_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME recording_pipeline_tests COMMAND recording_pipeline_tests)
