idf_component_register(
    SRCS "lifetrail_gps.c" "lifetrail_nmea.c"
    INCLUDE_DIRS "include" "."
    REQUIRES minmea)
