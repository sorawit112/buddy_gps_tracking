idf_component_register(SRCS "gps_client.c"
                       PRIV_REQUIRES nvs_flash esp_wifi esp_event esp_netif esp_http_client cJSON
                       INCLUDE_DIRS ".")
