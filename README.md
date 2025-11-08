# **Buddy GPS Tracking**


# **Section 1 NODE Firmware**
access source code in folder **firmware**

## Features
*   **Asynchronous Processing Task** 2 Events for WIFI and IP Check and 1 Task for main_task
*   **Periodically Data Transmission** telemetry sending interval data to server by HTTP requested -- configurable 
*   **Power Optimization** optimizing power consomption using Light Sleep and Deep Sleep

## Development

*  **C** Developed with C language
*  **VS-Code** with **ESP-IDF** [extension](https://github.com/espressif/vscode-esp-idf-extension)
*  **ESP-IDF CLI** [getting start](https://docs.espressif.com/projects/esp-idf/en/stable/esp32/get-started/index.html#)

## Configuration and Deployment

### 1. Accessing the Configuration Menu

```bash    
idf.py set-target esp32
idf.py menuconfig
```
or use Extension **SDK Configuration Editor** in VS-Code

### 2. Wi-Fi and API Endpoint Settings

| Configuration Item | Kconfig Macro | Description | Default Value
|---|---|---|---|
| Device ID	| CONFIG_DEVICE_ID | A static string used to uniquely identify this specific ESP32 unit in the telemetry payload. | ESP32_001
|WiFi SSID |	CONFIG_WIFI_SSID |	The network name (SSID) the ESP32 should connect to. |	MyNetworkName
|WiFi Password |	CONFIG_WIFI_PASSWORD |	The password for the specified Wi-Fi network. |	secure_password
|API url |	CONFIG_API_URL |	The full HTTP URL of the tracking server endpoint where the JSON telemetry data will be POSTed. If your server is running on a non-standard port (e.g., 8080), the port must be included in this URL. |	http://0.0.0.0:8080/api/data

### 3. Wi-Fi and API Endpoint Settings

| Configuration Item | Kconfig Macro | Description | Default Value
|---|---|---|---|
Start Sending Hour | CONFIG_START_HOUR | The first hour (in 24-hour format) of the day when telemetry transmission is allowed. (e.g., 8 for 8:00 AM). | 8
Stop Sending Hour | CONFIG_END_HOUR | The last hour (in 24-hour format) of the day when telemetry transmission is allowed. Transmission stops immediately after this hour concludes. (e.g., 19 for 7:00 PM). | 19
Polling Interval in Seconds | CONFIG_POLLING_INTERVAL_SEC | The short delay (in seconds) used for Light Sleep during the active operational window (8 AM - 7 PM). This is set low (e.g., 300s = 5 minutes) to ensure the device wakes up precisely to catch the start of every new hour boundary. | 300

### 4. Saving and Building

```bash
idf.py build
```

### 5. Flash and Monitor

```bash
idf.py -p PORT flash
idf.py -p PORT monitor
```


## Firmware State Flow Diagram
![State Flow](assets/firmware_state.png)
--- 

# Section 2 *Web Application*
![Buddy GPS Tracking](assets/app.png)

A web application to track and display GPS data from an ESP32 device, designed for tracking a pet.

## Features

*   **Real-time Data Display:** View the latest GPS data from your device in a table.
*   **Data Refresh:** Manually refresh the data to get the latest updates.
*   **CSV Export:** Download all stored GPS data as a CSV file for further analysis.

## Tech Stack

*   **Frontend:** [Leptos](https://leptos.dev/) (Rust)
*   **Backend:** [Actix](https://actix.rs/) (Rust)
*   **Styling:** [Tailwind CSS](https://tailwindcss.com/)
*   **Testing:** [Playwright](https://playwright.dev/)

## Getting Started

### Prerequisites

*   [Rust](https://www.rust-lang.org/tools/install)
*   [cargo-leptos](https://github.com/leptos-rs/leptos/tree/main/cargo-leptos)
*   [Node.js](https://nodejs.org/en/) (for end-to-end testing and tailwind )
*   [Docker](https://docs.docker.com/desktop/?_gl=1*1h3b8e6*_gcl_au*MjEwNTA0Mjg0MC4xNzYyMjcwMjIx*_ga*MTU5ODE3NzM2Mi4xNzYyMjcwMTY4*_ga_XJWPQMJYHQ*czE3NjIyNzAxNjckbzEkZzEkdDE3NjIyNzIxNzckajU5JGwwJGgw) (Optional: for running app directly)

### Local Installation and Running

1.  **Install `cargo-leptos`:**
    ```bash
    cargo install cargo-leptos
    ```
    Or following getting start here [GettingStart](https://github.com/leptos-rs/cargo-leptos)

2.  **Build and run the application:**
    ```bash
    cargo leptos watch
    ```

3.  **Open your browser** to `http://127.0.0.1:3000`.

### Using Docker for Starting Application

```bash
docker run -p 8080:8080 chinouplus/buddy-gps-tracking:latest
```

### Testing Send Data to Application using `curl`

```bash
curl -X POST http://0.0.0.0:8080/api/data -H "Content-Type: application/json" -d '{
    "id": "ESP32_001",
    "payload": "1A2B3C4DEF", # 10 Hex Chars for 4xlongtitude, 4xlattitude and 2xbattery
    "date": "2025-10-31",
    "time": "18:05:22"
}'
```
