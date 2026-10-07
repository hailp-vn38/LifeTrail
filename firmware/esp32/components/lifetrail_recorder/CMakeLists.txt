idf_component_register(
    SRCS "lifetrail_recorder.c" "lifetrail_recorder_gps.c" "lifetrail_recorder_storage.c"
         "lifetrail_recorder_config.c"
    INCLUDE_DIRS "include"
    REQUIRES lifetrail_gps_policy lifetrail_storage
    PRIV_REQUIRES esp_driver_uart esp_driver_spi esp_driver_sdspi fatfs sdmmc esp_timer)
