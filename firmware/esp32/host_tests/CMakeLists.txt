cmake_minimum_required(VERSION 3.16)
project(lifetrail_firmware_host_tests C)

enable_testing()

add_executable(gps_epoch_batch_tests
    ../components/third_party/minmea/minmea.c
    ../components/lifetrail_gps/lifetrail_gps.c
    ../components/lifetrail_gps/lifetrail_nmea.c
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    tests/test_gps_epoch_batch.c)

target_include_directories(gps_epoch_batch_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_gps
    ../components/third_party/minmea
    ../components/lifetrail_storage/include)

target_compile_features(gps_epoch_batch_tests PRIVATE c_std_11)
target_compile_options(gps_epoch_batch_tests PRIVATE -Wall -Wextra -Werror -pedantic)
target_compile_definitions(gps_epoch_batch_tests PRIVATE LT_GPS_TEST_FIXTURES_DIR="${CMAKE_CURRENT_SOURCE_DIR}/fixtures")

add_test(NAME gps_epoch_batch_tests COMMAND gps_epoch_batch_tests)

add_executable(batch_store_tests
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    ../components/lifetrail_storage/lifetrail_batch_store.c
    ../components/lifetrail_storage/lifetrail_sha256.c
    tests/test_batch_store.c)

target_include_directories(batch_store_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_storage/include)

target_compile_features(batch_store_tests PRIVATE c_std_11)
target_compile_options(batch_store_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME batch_store_tests COMMAND batch_store_tests)

add_executable(provisioning_sync_tests
    ../components/lifetrail_storage/lifetrail_gps_batch.c
    ../components/lifetrail_storage/lifetrail_batch_store.c
    ../components/lifetrail_storage/lifetrail_sha256.c
    ../components/lifetrail_provisioning/lifetrail_provisioning.c
    ../components/lifetrail_sync/lifetrail_sync.c
    ../components/lifetrail_sync/lifetrail_sync_ack.c
    tests/test_provisioning_sync.c)

target_include_directories(provisioning_sync_tests PRIVATE
    ../components/lifetrail_gps/include
    ../components/lifetrail_storage/include
    ../components/lifetrail_provisioning/include
    ../components/lifetrail_sync/include)

target_compile_features(provisioning_sync_tests PRIVATE c_std_11)
target_compile_options(provisioning_sync_tests PRIVATE -Wall -Wextra -Werror -pedantic)
add_test(NAME provisioning_sync_tests COMMAND provisioning_sync_tests)
