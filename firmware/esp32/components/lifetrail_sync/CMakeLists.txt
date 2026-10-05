idf_component_register(SRCS "lifetrail_sync.c" "lifetrail_sync_ack.c" "lifetrail_sync_esp_http.c"
    INCLUDE_DIRS "include" REQUIRES lifetrail_storage esp_http_client)
