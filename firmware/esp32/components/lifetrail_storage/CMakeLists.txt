idf_component_register(
    SRCS "lifetrail_gps_batch.c" "lifetrail_batch_store.c" "lifetrail_sha256.c"
         "lifetrail_buffered_file.c" "lifetrail_storage_queue.c"
    INCLUDE_DIRS "include"
    REQUIRES lifetrail_gps lifetrail_gps_policy)
