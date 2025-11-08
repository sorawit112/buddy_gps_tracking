#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"
#include "esp_system.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_err.h"
#include "nvs_flash.h"
#include "esp_wifi.h"
#include "esp_sleep.h"
#include "esp_http_client.h"
#include "lwip/err.h"
#include "lwip/sys.h"
#include "esp_sntp.h"
#include "cJSON.h" // A standard library for JSON handling in ESP-IDF

static const char *TAG = "GPS_CLIENT";
static EventGroupHandle_t wifi_event_group;
const int WIFI_CONNECTED_BIT = BIT0;

// --- CONFIGURATION ---
#define DEVICE_ID               CONFIG_DEVICE_ID //ESP32_001"
#define WIFI_SSID               CONFIG_WIFI_SSID
#define WIFI_PASS               CONFIG_WIFI_PASSWORD
#define API_URL                 CONFIG_API_URL
#define START_HOUR              CONFIG_START_HOUR
#define END_HOUR                CONFIG_END_HOUR
#define POLLING_INTERVAL_SEC    CONFIG_POLLING_INTERVAL_SEC
#define WAKEUP_LEAD_TIME_MIN    30 //minutes

// use RTC for deep_sleep compatibilites for wakeup at before next working hour in the next day
// Tracks the last hour (0-23) when data was successfully sent. Initialize to a value outside 0-23.
RTC_DATA_ATTR static int last_tx_hour_rtc = -1;

// --- Function Prototypes ---
static void initialize_sntp(void);
static void sync_time(void);
static void wifi_init_station(void);
static esp_err_t http_event_handler(esp_http_client_event_t *evt);
static char* generate_payload(void);
static esp_err_t send_gps_data(const char *json_data);
static void send_gps_task(void *pvParameters);


/**
 * @brief Generates the 10-character random hexadecimal payload string.
 *
 * Structure: 4 hex (Longitude) + 4 hex (Latitude) + 2 hex (Battery %)
 * Total size: 10 hex characters (5 bytes)
 * The output string is 11 characters (10 for hex + null terminator)
 */
static char* generate_payload(void) {
    // 4 hex for Longitude (2 bytes)
    uint16_t longitude = esp_random() % 0xFFFF;
    // 4 hex for Latitude (2 bytes)
    uint16_t latitude = esp_random() % 0xFFFF;
    // 2 hex for Battery % (1 byte, 0x00 to 0xFF approx 0-100%)
    uint8_t battery = esp_random() % 0x65; // Keeping it realistic (0-100 decimal = 0x00-0x64 hex)

    // Allocate space for the 10 hex chars + null terminator
    char *payload_str = (char *)malloc(11);
    if (payload_str == NULL) {
        ESP_LOGE(TAG, "Failed to allocate memory for payload");
        return NULL;
    }

    // Combine and format the components into the 10-char hex string
    snprintf(payload_str, 11, "%04X%04X%02X", longitude, latitude, battery);
    
    return payload_str;
}

// Calculate the exact seconds remaining until the next desired wakeup time.
long calculate_sleep_duration(time_t current_time) {
    struct tm *timeinfo;
    
    // Calculate the target time for the next day's START_HOUR wakeup
    time_t target_time = current_time;
    timeinfo = localtime(&target_time);

    // Set target date/time to the next day's START_HOUR - WAKEUP min
    timeinfo->tm_hour = START_HOUR;
    timeinfo->tm_min = 0;
    timeinfo->tm_sec = 0;
    
    // Convert to time_t and subtract the 30-minute lead time
    target_time = mktime(timeinfo) - (WAKEUP_LEAD_TIME_MIN * 60);

    // If the target is in the past (e.g., current time is 9:00 AM), advance to the next day.
    if (target_time < current_time) {
        target_time += (24 * 60 * 60); // Add 24 hours
    }

    long sleep_duration_sec = target_time - current_time;

    ESP_LOGI(TAG, "Current time: %ld, Target wake time: %ld", current_time, target_time);
    return sleep_duration_sec;
}

/**
 * @brief Handles all HTTP client events.
 */
static esp_err_t http_event_handler(esp_http_client_event_t *evt) {
    switch (evt->event_id) {
        case HTTP_EVENT_ERROR:
            ESP_LOGE(TAG, "HTTP_EVENT_ERROR");
            break;
        case HTTP_EVENT_ON_CONNECTED:
            ESP_LOGI(TAG, "HTTP_EVENT_ON_CONNECTED");
            break;
        case HTTP_EVENT_HEADERS_SENT:
            ESP_LOGI(TAG, "HTTP_EVENT_HEADERS_SENT");
            break;
        case HTTP_EVENT_ON_HEADER:
            ESP_LOGD(TAG, "HTTP_EVENT_ON_HEADER, key=%s, value=%s", evt->header_key, evt->header_value);
            break;
        case HTTP_EVENT_ON_DATA:
            ESP_LOGD(TAG, "HTTP_EVENT_ON_DATA, len=%d", evt->data_len);
            break;
        case HTTP_EVENT_ON_FINISH:
            ESP_LOGI(TAG, "HTTP_EVENT_ON_FINISH");
            break;
        case HTTP_EVENT_DISCONNECTED:
            ESP_LOGI(TAG, "HTTP_EVENT_DISCONNECTED");
            break;
        case HTTP_EVENT_REDIRECT:
            break; 
    }
    return ESP_OK;
}

/**
 * @brief Assembles and sends the final JSON data.
 */
static esp_err_t send_gps_data(const char *json_data) {
    esp_http_client_config_t config = {
        .url = API_URL,
        .event_handler = http_event_handler,
        .method = HTTP_METHOD_POST,
        .timeout_ms = 5000,
    };
    esp_http_client_handle_t client = esp_http_client_init(&config);
    if (client == NULL) {
        ESP_LOGE(TAG, "Failed to initialize HTTP client");
        return ESP_FAIL;
    }

    // Set headers
    esp_http_client_set_header(client, "Content-Type", "application/json");

    // Set POST data
    esp_http_client_set_post_field(client, json_data, strlen(json_data));
    
    ESP_LOGI(TAG, "Attempting to send JSON payload to %s:\n%s", API_URL, json_data);

    // Perform the POST request
    esp_err_t err = esp_http_client_perform(client);

    if (err == ESP_OK) {
        ESP_LOGI(TAG, "HTTP POST Status = %d, Content_length = %lld",
                 (int)esp_http_client_get_status_code(client),
                 esp_http_client_get_content_length(client));
    } else {
        ESP_LOGE(TAG, "HTTP POST request failed: %s", esp_err_to_name(err));
    }

    esp_http_client_cleanup(client);
    return err;
}

/**
 * @brief The main task that generates and sends gps data periodically.
 */
static void send_gps_task(void *pvParameters) {
    time_t now;
    struct tm timeinfo;
    char date_str[11]; // YYYY-MM-DD\0
    char time_str[9];  // HH:MM:SS\0
    char *payload_str = NULL;
    cJSON *root = NULL;
    char *json_out = NULL;
    bool should_sleep_deep = false;
    bool first_quarter_hour = false;
    long sleep_duration_sec = 0;

    // Wait for wifi connected
    xEventGroupWaitBits(wifi_event_group, WIFI_CONNECTED_BIT, pdFALSE, pdTRUE, portMAX_DELAY);

    // Wait for time synchronized
    sync_time();

    while (1) {
        time(&now);
        localtime_r(&now, &timeinfo);

        // Check if time is valid (year 2000 is a common SNTP check)
        if (timeinfo.tm_year < (2000 - 1900)) {
            ESP_LOGE(TAG, "Time not set yet, waiting...");
            vTaskDelay(pdMS_TO_TICKS(2000));
            continue;
        }

        int current_hour = timeinfo.tm_hour;
        first_quarter_hour = timeinfo.tm_min <= 15;

        // Check 2: Operational Window (8 AM to 7 PM, inclusive)
        if (current_hour >= START_HOUR && current_hour <= END_HOUR ) {
            ESP_LOGI(TAG, "Inside Operational Window (%d:00).", current_hour);
            
            // check 3: Within First Quarter otherwise sleep until next hourt
            if (!first_quarter_hour) {
                ESP_LOGI(TAG, "Not First Quarter Hour (%d:%d).", current_hour, timeinfo.tm_min);

                // Calculate seconds remaining until next WORKING HOURT
                int seconds_to_start = (60 - timeinfo.tm_min) * 60 - timeinfo.tm_sec;
                
                // Safety check: Don't sleep if it's already next WORKING HOUR
                if (seconds_to_start > 0) {
                    ESP_LOGI(TAG, "Entering Light Sleep for %ld seconds.", seconds_to_start);
                    vTaskDelay(pdMS_TO_TICKS(seconds_to_start * 1000));
                    continue;
                }
            }
            
            // Check 3: One-Hour Interval
            if (current_hour != last_tx_hour_rtc) {
                
                ESP_LOGI(TAG, "Sending telemetry for hour %d.", current_hour);

                // --- DATA GENERATION AND TRANSMISSION ---
                
                // 1. Format date and time
                strftime(date_str, sizeof(date_str), "%Y-%m-%d", &timeinfo);
                strftime(time_str, sizeof(time_str), "%H:%M:%S", &timeinfo);

                // 2. Generate payload
                payload_str = generate_payload();
                if (payload_str != NULL) {
                    // 3. Create and Send JSON
                    root = cJSON_CreateObject();
                    if (root) {
                        cJSON_AddStringToObject(root, "id", DEVICE_ID);
                        cJSON_AddStringToObject(root, "payload", payload_str);
                        cJSON_AddStringToObject(root, "date", date_str);
                        cJSON_AddStringToObject(root, "time", time_str);
                        json_out = cJSON_PrintUnformatted(root);

                        if (json_out != NULL) {
                            esp_err_t send_status = send_gps_data(json_out);
                            
                            // 4. Update state ONLY on successful transmission
                            if (send_status == ESP_OK) {
                                last_tx_hour_rtc = current_hour; // Mark this hour as sent
                                ESP_LOGI(TAG, "Telemetry successfully sent. Next transmission will be at %d:00.", (current_hour + 1));
                            } else {
                                ESP_LOGE(TAG, "Transmission failed. Retrying next poll.");
                            }
                            cJSON_free(json_out);
                        } else { ESP_LOGE(TAG, "Failed to format JSON string."); }
                    } else { ESP_LOGE(TAG, "Failed to create cJSON root object."); }
                } else { ESP_LOGE(TAG, "Payload generation failed."); }

                // Cleanup
                if (root) cJSON_Delete(root);
                if (payload_str) free(payload_str);
                
            } else {
                ESP_LOGI(TAG, "Already sent data for hour %d. Waiting for the next hour.", current_hour);
            }

        } else if (current_hour == (START_HOUR - 1)) {
            // --- 2. Pre-Window Wake-up (Hour 7, e.g., 7:30 AM) ---
            // The device just woke up 30 minutes early. It MUST wait for 8 AM using light sleep.
            ESP_LOGI(TAG, "Pre-Window wakeup. Waiting for %d:00 AM using Light Sleep.",START_HOUR);
            last_tx_hour_rtc = -1; // Ensure tracker is reset for the 8 AM send.
            should_sleep_deep = false;
            
            // Calculate seconds remaining until 8:00:00 AM
            int seconds_to_start = (60 - timeinfo.tm_min) * 60 - timeinfo.tm_sec;
            
            // Safety check: Don't sleep if it's already 8 AM
            if (seconds_to_start > 0) {
                ESP_LOGI(TAG, "Entering Light Sleep for %ld seconds.", seconds_to_start);
                vTaskDelay(pdMS_TO_TICKS(seconds_to_start * 1000));
                continue;
            }
            // After this delay, the task loops and 'current_hour' will be START_HOUR, triggering transmission.

        } else {
            // --- 3. Outside Operational Window (Hour 0 to 6, and 20 to 23) ---
            ESP_LOGI(TAG, "Outside operational window (%d:%d). Initiating DEEP SLEEP cycle.", current_hour,timeinfo.tm_min);
            should_sleep_deep = true;
            last_tx_hour_rtc = -1; // Reset tracker
        }

        // --- 4. Execute Deep Sleep ---
        if (should_sleep_deep) {

            // Calculate sleep duration until (START_HOUR - WAKEUP_LEAD_TIME_MIN) the NEXT DAY
            long sleep_duration_sec = calculate_sleep_duration(now);
            ESP_LOGI(TAG, "Entering Deep Sleep for %ld seconds.", sleep_duration_sec);

            // Convert seconds to microseconds for the sleep function
            uint64_t sleep_duration_us = (uint64_t)sleep_duration_sec * 1000000;
            
            // Ensure all logs are printed before shutdown
            fflush(stdout); 

            // Enter Deep Sleep 
            esp_deep_sleep(sleep_duration_us);
        }  else {
            // Wait for the next polling interval
            vTaskDelay(pdMS_TO_TICKS(POLLING_INTERVAL_SEC * 1000));
            // polling interval light sleep
        }
    }
}

// --- SNTP Time Synchronization ---

static void initialize_sntp(void) {
    ESP_LOGI(TAG, "Initializing SNTP");
    esp_sntp_setoperatingmode(SNTP_OPMODE_POLL);
    esp_sntp_setservername(0, "pool.ntp.org");
    esp_sntp_init();
}

static void sync_time(void) {
    initialize_sntp();

    // wait for time to be set
    int retry = 0;
    const int retry_count = 10;
    while (sntp_get_sync_status() == SNTP_SYNC_STATUS_RESET && ++retry < retry_count) {
        ESP_LOGI(TAG, "Waiting for system time to be set... (%d/%d)", retry, retry_count);
        vTaskDelay(pdMS_TO_TICKS(2000));
    }

    if (retry >= retry_count) {
        ESP_LOGE(TAG, "Failed to get time from SNTP after multiple retries.");
        return;
    }
    
    // Set Timezone
    setenv("TZ", "ICT-7", 1); 
    tzset();
    
    time_t now;
    struct tm timeinfo;
    time(&now);
    localtime_r(&now, &timeinfo);
    char strftime_buf[64];
    strftime(strftime_buf, sizeof(strftime_buf), "%c", &timeinfo);
    ESP_LOGI(TAG, "The current date/time is: %s", strftime_buf);
}

// --- Wi-Fi Initialization ---

static void wifi_event_handler(void *arg, esp_event_base_t event_base,
                          int32_t event_id, void *event_data) {
    if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_START) {
        esp_wifi_connect();
    } else if (event_base == WIFI_EVENT && event_id == WIFI_EVENT_STA_DISCONNECTED) {
        ESP_LOGI(TAG, "Wi-Fi disconnected. Retrying connection...");
        esp_wifi_connect();
        xEventGroupClearBits(wifi_event_group, WIFI_CONNECTED_BIT);
    } else if (event_base == IP_EVENT && event_id == IP_EVENT_STA_GOT_IP) {
        ip_event_got_ip_t *event = (ip_event_got_ip_t *)event_data;
        ESP_LOGI(TAG, "Got IP address: " IPSTR, IP2STR(&event->ip_info.ip));
        xEventGroupSetBits(wifi_event_group, WIFI_CONNECTED_BIT);
    }
}

static void wifi_init_station(void) {
    wifi_event_group = xEventGroupCreate();

    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_sta();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    esp_event_handler_instance_t instance_any_id;
    esp_event_handler_instance_t instance_got_ip;
    ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT,
                                                        ESP_EVENT_ANY_ID,
                                                        &wifi_event_handler,
                                                        NULL,
                                                        &instance_any_id));
    ESP_ERROR_CHECK(esp_event_handler_instance_register(IP_EVENT,
                                                        IP_EVENT_STA_GOT_IP,
                                                        &wifi_event_handler,
                                                        NULL,
                                                        &instance_got_ip));

    wifi_config_t wifi_config = {
        .sta = {
            .ssid = WIFI_SSID,
            .password = WIFI_PASS,
            .threshold.authmode = WIFI_AUTH_WPA2_PSK,
            .pmf_cfg = {
                .capable = true,
                .required = false
            },
        },
    };
    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_STA));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_STA, &wifi_config));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "Wi-Fi initialization finished.");

    // Wait for Wi-Fi connection
    xEventGroupWaitBits(wifi_event_group, WIFI_CONNECTED_BIT, pdFALSE, pdTRUE, portMAX_DELAY);
}

// --- Main Application Entry Point ---

void app_main(void) {
    // Initialize NVS (Non-Volatile Storage)
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_ERROR_CHECK(ret);

    // Initialize Wi-Fi and wait for connection
    wifi_init_station();

    // Create the gps sending task
    xTaskCreate(send_gps_task, "send_gps_task", 4096, NULL, 5, NULL);
}