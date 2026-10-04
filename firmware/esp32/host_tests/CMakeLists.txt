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
