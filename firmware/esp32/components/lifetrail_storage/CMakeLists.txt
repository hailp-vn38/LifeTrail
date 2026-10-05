idf_component_register(
    SRCS "lifetrail_gps_batch.c" "lifetrail_batch_store.c" "lifetrail_sha256.c"
    INCLUDE_DIRS "include"
    REQUIRES lifetrail_gps)
